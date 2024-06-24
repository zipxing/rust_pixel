// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! Sprite further encapsulates Buffer
//! It is also the most common component in RustPixel
//! It provides drawing methods such as set_border，draw_line, draw_circle
//! Refer to util/shape.rs for an example of how to draw a line

use crate::{
    asset::{AssetManager, AssetState, AssetType},
    render::buffer::Buffer,
    render::cell::cellsym,
    // render::image::*,
    render::style::{Color, Style},
    util::shape::{circle, line, prepare_line},
    util::{PointF32, Rect},
};
use bitflags::bitflags;
// use log::info;
// use std::f32;

mod sprites;
pub use sprites::Sprites;

/// Defines some common tabs symbol (in text mode)
pub const SYMBOL_LINE: [&str; 37] = [
    "│", "║", "┃", "─", "═", "━", "┐", "╮", "╗", "┓", "┌", "╭", "╔", "┏", "┘", "╯", "╝", "┛", "└",
    "╰", "╚", "┗", "┤", "╣", "┫", "├", "╠", "┣", "┬", "╦", "┳", "┴", "╩", "┻", "┼", "╬", "╋",
];

// border's bigflags
bitflags! {
    pub struct Borders: u32 {
        const NONE   = 0b0000_0001;
        const TOP    = 0b0000_0010;
        const RIGHT  = 0b0000_0100;
        const BOTTOM = 0b0000_1000;
        const LEFT   = 0b0001_0000;
        const ALL    = Self::TOP.bits() | Self::RIGHT.bits() | Self::BOTTOM.bits() | Self::LEFT.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderType {
    Plain,
    Rounded,
    Double,
    Thick,
}

/// Used to simplify the call to set_content_by_asset method
#[macro_export]
macro_rules! asset2sprite {
    ($spr:expr, $ctx:expr, $loc:expr $(, $arg:expr)* ) => {
        let ll = $loc.to_lowercase();
        // determine asset type...
        let mut at = AssetType::ImgPix;
        if ll.ends_with(".txt") {
            at = AssetType::ImgEsc;
        }
        if ll.ends_with(".pix") {
            at = AssetType::ImgPix;
        }
        if ll.ends_with(".ssf") {
            at = AssetType::ImgSsf;
        }
        // collect other args...
        let mut va = Vec::new();
        $( va.push($arg); )*
        let mut frame_idx = 0;
        let mut x = 0;
        let mut y = 0;
        match va.len() {
            1 => {
                frame_idx = va[0];
            },
            3 => {
                frame_idx = va[0];
                x = va[1] as u16;
                y = va[2] as u16;
            },
            _ => {},
        }
        #[cfg(not(target_arch = "wasm32"))]
        let nl = &format!("{}{}{}{}assets{}{}",
            $ctx.prefix_path,
            std::path::MAIN_SEPARATOR,
            $ctx.game_name,
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR,
            $loc);
        #[cfg(target_arch = "wasm32")]
        let nl = &format!("assets{}{}", std::path::MAIN_SEPARATOR, $loc);
        // call spr.set_content_by_asset...
        $spr.set_content_by_asset(
            &mut $ctx.asset_manager,
            at,
            nl,
            frame_idx,
            x,
            y,
        );
    };
}

pub trait Widget {
    fn render(&mut self, am: &mut AssetManager, buf: &mut Buffer);
}

#[derive(Clone)]
pub struct Sprite {
    pub content: Buffer,
    pub angle: f64,
    pub alpha: u8,
    pub asset_request: Option<(AssetType, String, usize, u16, u16)>,
    render_weight: i32,
}

impl Widget for Sprite {
    fn render(&mut self, am: &mut AssetManager, buf: &mut Buffer) {
        if !self.is_hidden() {
            self.check_asset_request(am);
            buf.merge(&self.content, self.alpha, true);
        }
    }
}

impl Sprite {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        let area = Rect::new(x, y, width, height);
        let buffer = Buffer::empty(area);
        Self {
            content: buffer,
            angle: 0.0,
            alpha: 255,
            asset_request: None,
            render_weight: 1,
        }
    }

    pub fn set_alpha(&mut self, a: u8) {
        self.alpha = a;
    }

    pub fn set_sdl_content(&mut self, x: u16, y: u16, sym: u8, fg: u8, bg: u8) {
        self.content.set_str(
            x,
            y,
            cellsym(sym),
            Style::default()
                .fg(Color::Indexed(fg))
                .bg(Color::Indexed(bg)),
        );
    }

    pub fn set_content_by_asset(
        &mut self,
        am: &mut AssetManager,
        atype: AssetType,
        location: &str,
        frame_idx: usize,
        off_x: u16,
        off_y: u16,
    ) {
        self.asset_request = Some((atype, location.to_string(), frame_idx, off_x, off_y));
        am.load(atype, location);
        self.check_asset_request(am);
    }

    pub fn check_asset_request(&mut self, am: &mut AssetManager) {
        if let Some(req) = &self.asset_request {
            if let Some(ast) = am.get(&req.1) {
                match ast.get_state() {
                    AssetState::Ready => {
                        ast.set_sprite(self, req.2, req.3, req.4);
                        self.asset_request = None;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn set_angle(&mut self, a: f64) {
        self.angle = a;
    }

    pub fn get_center_point(&self) -> PointF32 {
        PointF32 {
            x: self.content.area.x as f32 + self.content.area.width as f32 / 2.0,
            y: self.content.area.y as f32 + self.content.area.height as f32 / 2.0,
        }
    }

    pub fn set_hidden(&mut self, flag: bool) {
        if flag {
            self.render_weight = -1 * self.render_weight.abs();
        } else {
            self.render_weight = self.render_weight.abs();
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.render_weight < 0
    }

    pub fn set_border(&mut self, borders: Borders, border_type: BorderType, style: Style) {
        // vertical horizontal
        // top_right top_left bottom_right bottom_left
        // vertical_left vertical_right horizontal_down horizontal_up
        // cross
        let lineidx: [usize; 11] = match border_type {
            BorderType::Plain => [0, 3, 6, 10, 14, 18, 22, 25, 28, 31, 34],
            BorderType::Rounded => [0, 3, 7, 11, 15, 19, 22, 25, 28, 31, 34],
            BorderType::Double => [1, 4, 8, 12, 16, 20, 23, 26, 29, 33, 35],
            BorderType::Thick => [2, 5, 9, 13, 17, 21, 24, 27, 30, 34, 36],
        };
        if borders.intersects(Borders::LEFT) {
            for y in 0..self.content.area.height {
                self.content.set_str(0, y, SYMBOL_LINE[lineidx[0]], style);
            }
        }
        if borders.intersects(Borders::TOP) {
            for x in 0..self.content.area.width {
                self.content.set_str(x, 0, SYMBOL_LINE[lineidx[1]], style);
            }
        }
        if borders.intersects(Borders::RIGHT) {
            let x = self.content.area.width - 1;
            for y in 0..self.content.area.height {
                self.content.set_str(x, y, SYMBOL_LINE[lineidx[0]], style);
            }
        }
        if borders.intersects(Borders::BOTTOM) {
            let y = self.content.area.height - 1;
            for x in 0..self.content.area.width {
                self.content.set_str(x, y, SYMBOL_LINE[lineidx[1]], style);
            }
        }
        if borders.contains(Borders::RIGHT | Borders::BOTTOM) {
            self.content.set_str(
                self.content.area.width - 1,
                self.content.area.height - 1,
                SYMBOL_LINE[lineidx[4]],
                style,
            );
        }
        if borders.contains(Borders::RIGHT | Borders::TOP) {
            self.content.set_str(
                self.content.area.width - 1,
                0,
                SYMBOL_LINE[lineidx[2]],
                style,
            );
        }
        if borders.contains(Borders::LEFT | Borders::BOTTOM) {
            self.content.set_str(
                0,
                self.content.area.height - 1,
                SYMBOL_LINE[lineidx[5]],
                style,
            );
        }
        if borders.contains(Borders::LEFT | Borders::TOP) {
            self.content.set_str(0, 0, SYMBOL_LINE[lineidx[3]], style);
        }
    }

    pub fn copy_content(&mut self, sp: &Sprite) {
        let backup_area = self.content.area;
        //set the pos to (0,0) to merge with boxes
        self.content.area = Rect::new(0, 0, backup_area.width, backup_area.height);
        self.content.reset();
        self.content.merge(&sp.content, sp.alpha, false);

        //after merging, set back to its original pos
        self.content.area = backup_area;
    }

    pub fn set_pos(&mut self, x: u16, y: u16) {
        self.content.area = Rect::new(x, y, self.content.area.width, self.content.area.height);
    }

    pub fn draw_circle(
        &mut self,
        x0: u16,
        y0: u16,
        radius: u16,
        sym: &str,
        fg_color: u8,
        bg_color: u8,
    ) {
        for p in circle(x0, y0, radius) {
            if (p.0 as u16) < self.content.area.width && (p.1 as u16) < self.content.area.height {
                self.content.set_str(
                    p.0 as u16,
                    p.1 as u16,
                    sym,
                    Style::default()
                        .fg(Color::Indexed(fg_color))
                        .bg(Color::Indexed(bg_color)),
                );
            }
        }
    }

    pub fn draw_line(
        &mut self,
        px0: u16,
        py0: u16,
        px1: u16,
        py1: u16,
        sym: Option<Vec<Option<u8>>>,
        fg_color: u8,
        bg_color: u8,
    ) {
        let (x0, y0, x1, y1) = prepare_line(px0, py0, px1, py1);
        // start, end, v, h, s, bs...
        let mut syms: Vec<Option<u8>> = vec![None, None, Some(119), Some(116), Some(77), Some(78)];
        if let Some(s) = sym {
            syms = s;
        }
        for p in line(x0, y0, x1, y1) {
            let x = p.0 as u16;
            let y = p.1 as u16;
            let sym = syms[p.2 as usize];
            if let Some(s) = sym {
                if x < self.content.area.width && y < self.content.area.height {
                    self.content.set_str(
                        x,
                        y,
                        cellsym(s),
                        Style::default()
                            .fg(Color::Indexed(fg_color))
                            .bg(Color::Indexed(bg_color)),
                    );
                }
            }
        }
    }
}
