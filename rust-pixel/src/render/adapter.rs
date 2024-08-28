// RustPixel
// copyright zipxing@hotmail.com 2022~2024

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use crate::render::style::Color;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use crate::util::{
    Rand, {PointI32, ARect},
};
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use crate::LOGO_FRAME;
use crate::{
    event::Event,
    render::{buffer::Buffer, sprite::Sprites},
    util::Rect,
};
use std::any::Any;
use std::time::Duration;
use log::info;

// add more files to this list when needed
// max 255 textures
// merge l,u,e1,e2 to a complete image
//
// c64l.png  c64u.png    -->  c64.png
// c64e1.png c64e2.png 

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const PIXEL_TEXTURE_FILES: [&'static str; 1] = [
    "assets/pix/c64.png",
];

pub const PIXEL_SYM_WIDTH: f32 = 16.0;
pub const PIXEL_SYM_HEIGHT: f32 = 16.0;
pub const PIXEL_LOGO_WIDTH: usize = 27;
pub const PIXEL_LOGO_HEIGHT: usize = 12;
pub const PIXEL_LOGO: [u8; PIXEL_LOGO_WIDTH * PIXEL_LOGO_HEIGHT * 3] = [
    32, 15, 1, 32, 202, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 239, 1, 32, 15, 1, 100, 239, 1, 32,
    239, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 165, 1, 32,
    165, 0, 32, 87, 1, 32, 15, 1, 18, 202, 1, 21, 202, 1, 19, 202, 1, 20, 202, 1, 32, 15, 1, 47,
    239, 1, 47, 239, 1, 116, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 87, 1, 32, 165, 0, 32, 165, 1, 32, 240, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 100, 239, 1, 81, 49, 1, 47, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15,
    1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15, 0, 32, 87, 1, 32, 15, 0, 32, 165, 0,
    47, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 238, 1, 47,
    238, 1, 47, 238, 1, 47, 239, 1, 100, 239, 1, 46, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1,
    104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1, 84,
    239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160,
    49, 1, 81, 49, 1, 32, 15, 1, 160, 86, 1, 32, 15, 1, 160, 49, 1, 47, 236, 1, 47, 236, 1, 46,
    234, 1, 160, 49, 1, 47, 239, 1, 81, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 47,
    239, 1, 160, 49, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 87, 1, 160, 45,
    1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 160, 45, 1, 32, 15, 1, 160, 45, 1, 32, 235, 1, 116, 235, 1,
    160, 45, 1, 47, 236, 1, 160, 45, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 46, 234, 1, 32, 15, 1,
    46, 234, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 1,
    32, 15, 0, 32, 197, 1, 160, 147, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 160, 147, 1, 32, 15,
    1, 160, 147, 1, 32, 235, 1, 116, 235, 1, 46, 235, 1, 81, 147, 1, 47, 239, 1, 47, 239, 1, 100,
    239, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 47, 239, 1, 32, 15, 1, 160, 147, 1,
    32, 239, 1, 84, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 32, 239, 1, 160, 147, 1, 47,
    239, 1, 104, 239, 1, 104, 240, 1, 160, 147, 1, 32, 15, 1, 160, 147, 1, 32, 15, 1, 116, 235, 1,
    160, 147, 1, 47, 239, 1, 160, 147, 1, 47, 239, 1, 47, 239, 1, 160, 147, 1, 104, 238, 1, 104,
    238, 1, 104, 238, 1, 104, 238, 1, 47, 242, 1, 160, 147, 1, 47, 239, 1, 104, 239, 1, 104, 239,
    1, 104, 239, 1, 47, 239, 1, 84, 239, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 81,
    214, 1, 47, 239, 1, 81, 214, 1, 47, 239, 1, 160, 214, 1, 47, 239, 1, 32, 0, 1, 46, 235, 1, 160,
    214, 1, 47, 236, 1, 81, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 47, 242, 1,
    81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 47, 239, 1, 32, 165, 1, 160, 214,
    1, 103, 239, 1, 32, 242, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 0, 1,
    32, 0, 1, 32, 87, 1, 32, 87, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 165, 0, 32,
    165, 0, 160, 214, 1, 103, 239, 1, 32, 242, 1, 32, 97, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97,
    0, 32, 165, 0, 32, 15, 1, 90, 214, 1, 47, 239, 1, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32,
    0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32,
    15, 1, 32, 15, 1, 32, 15, 1,
];

pub struct AdapterBase {
    pub game_name: String,
    pub path_prefix: String,
    pub project_path: String,
    pub title: String,
    pub cell_w: u16,
    pub cell_h: u16,
    pub pixel_w: u32,
    pub pixel_h: u32,
    pub ratio_x: f32,
    pub ratio_y: f32,
}

impl AdapterBase {
    pub fn new(pre: &str, gn: &str, project_path: &str) -> Self {
        Self {
            game_name: gn.to_string(),
            path_prefix: pre.to_string(),
            project_path: project_path.to_string(),
            title: "".to_string(),
            cell_w: 0,
            cell_h: 0,
            pixel_w: 0,
            pixel_h: 0,
            ratio_x: 1.0,
            ratio_y: 1.0,
        }
    }
}

pub trait Adapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String);
    fn reset(&mut self);
    fn get_base(&mut self) -> &mut AdapterBase;
    fn poll_event(&mut self, timeout: Duration, ev: &mut Vec<Event>) -> bool;
    fn render_buffer(
        &mut self,
        cb: &Buffer,
        pb: &Buffer,
        ps: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String>;

    fn set_size(&mut self, w: u16, h: u16) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.cell_w = w;
        bs.cell_h = h;
        self
    }

    fn size(&mut self) -> Rect {
        let bs = self.get_base();
        Rect::new(0, 0, bs.cell_w, bs.cell_h)
    }

    fn set_ratiox(&mut self, rx: f32) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.ratio_x = rx;
        self
    }

    fn set_ratioy(&mut self, ry: f32) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.ratio_y = ry;
        self
    }

    fn set_pixel_size(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.pixel_w = ((bs.cell_w + 2) as f32 * PIXEL_SYM_WIDTH / bs.ratio_x) as u32;
        bs.pixel_h = ((bs.cell_h + 2) as f32 * PIXEL_SYM_HEIGHT / bs.ratio_y) as u32;
        self
    }

    fn set_title(&mut self, s: String) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.title = s;
        self
    }

    fn cell_width(&self) -> f32;
    fn cell_height(&self) -> f32;
    fn hide_cursor(&mut self) -> Result<(), String>;
    fn show_cursor(&mut self) -> Result<(), String>;
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), String>;
    fn get_cursor(&mut self) -> Result<(u16, u16), String>;
    fn as_any(&self) -> &dyn Any;
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
fn render_helper(
    cell_w: u16,
    rx: f32,
    ry: f32,
    i: usize,
    sh: &(u8, u8, Color),
    px: u16,
    py: u16,
    is_border: bool,
) -> (ARect, ARect, usize, usize) {
    let w = PIXEL_SYM_WIDTH as i32;
    let h = PIXEL_SYM_HEIGHT as i32;
    let dstx = i as u16 % cell_w;
    let dsty = i as u16 / cell_w;
    let tex_count = PIXEL_TEXTURE_FILES.len() as u8 * 4;
    let tx = if sh.1 < tex_count { sh.1 as usize } else { 1 };
    let srcy = sh.0 as u32 / w as u32 + (tx as u32 / 2u32) * w as u32;
    let srcx = sh.0 as u32 % w as u32 + (tx as u32 % 2u32) * w as u32;

    (
        ARect {
            x: (w + 1) * srcx as i32,
            y: (h + 1) * srcy as i32,
            w: w as u32,
            h: h as u32,
        },
        ARect {
            x: (dstx + if is_border { 0 } else { 1 }) as i32 * (w as f32 / rx) as i32 + px as i32,
            y: (dsty + if is_border { 0 } else { 1 }) as i32 * (h as f32 / ry) as i32 + py as i32,
            w: (w as f32 / rx) as u32,
            h: (h as f32 / ry) as u32,
        },
        tx,
        sh.0 as usize,
    )
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_pixel_sprites<F>(pixel_spt: &mut Sprites, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize, f64, PointI32),
{
    // sort by render_weight...
    pixel_spt.update_render_index();
    for si in &pixel_spt.render_index {
        let s = &pixel_spt.sprites[si.0];
        if s.is_hidden() {
            continue;
        }
        let px = s.content.area.x;
        let py = s.content.area.y;
        let pw = s.content.area.width;
        let ph = s.content.area.height;

        for (i, cell) in s.content.content.iter().enumerate() {
            let sh = &cell.get_cell_info();
            let (s1, s2, texidx, symidx) = render_helper(pw, rx, ry, i, sh, px, py, false);
            let x = i % pw as usize;
            let y = i / pw as usize;
            // center point ...
            let ccp = PointI32 {
                x: ((pw as f32 / 2.0 - x as f32) * PIXEL_SYM_WIDTH as f32 / rx) as i32,
                y: ((ph as f32 / 2.0 - y as f32) * PIXEL_SYM_HEIGHT as f32 / ry) as i32,
            };
            let mut fc = sh.2.get_rgba();
            fc.3 = s.alpha;
            f(&fc, s1, s2, texidx, symidx, s.angle, ccp);
        }
    }
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_main_buffer<F>(buf: &Buffer, width: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize),
{
    for (i, cell) in buf.content.iter().enumerate() {
        for sh in &cell.draw_history {
            let (s1, s2, texidx, symidx) = render_helper(width, rx, ry, i, sh, 0, 0, false);
            if texidx == 2 && symidx == 28 {
                info!("CATCH IT@@@@@@@@@@@@ {:?}", cell.draw_history);
            }
            let fc = sh.2.get_rgba();
            f(&fc, s1, s2, texidx, symidx);
        }
    }
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize),
{
    let sh_top = (102u8, 1u8, Color::Indexed(7));
    let sh_other = (24u8, 2u8, Color::Indexed(7));
    let sh_close = (214u8, 1u8, Color::Indexed(7));

    for n in 0..cell_h as usize + 2 {
        for m in 0..cell_w as usize + 2 {
            if n != 0 && n != cell_h as usize + 1 && m != 0 && m != cell_w as usize + 1 {
                continue;
            }
            let rsh;
            if n == 0 {
                if m as u16 <= cell_w {
                    rsh = &sh_top;
                } else {
                    rsh = &sh_close;
                }
            } else {
                rsh = &sh_other;
            }
            let (s1, s2, texidx, symidx) = render_helper(
                cell_w + 2,
                rx,
                ry,
                n * (cell_w as usize + 2) + m,
                rsh,
                0,
                0,
                true,
            );
            let fc = rsh.2.get_rgba();
            f(&fc, s1, s2, texidx, symidx);
        }
    }
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_logo<F>(srx: f32, sry: f32, spw: u32, sph: u32, rd: &mut Rand, stage: u32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize),
{
    let rx = srx * 1.0;
    let ry = sry * 1.0;
    for y in 0usize..PIXEL_LOGO_HEIGHT {
        for x in 0usize..PIXEL_LOGO_WIDTH {
            let sci = y * PIXEL_LOGO_WIDTH + x;
            let symw = PIXEL_SYM_WIDTH / rx;
            let symh = PIXEL_SYM_HEIGHT / ry;

            let (s1, mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                rx,
                ry,
                sci,
                &(
                    PIXEL_LOGO[sci * 3],
                    PIXEL_LOGO[sci * 3 + 2],
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]),
                ),
                spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                false,
            );
            let fc = Color::Indexed(PIXEL_LOGO[sci * 3 + 1]).get_rgba();

            let randadj = 12 - (rd.rand() % 24) as i32;
            let sg = LOGO_FRAME as u8 / 3;
            let r: u8;
            let g: u8;
            let b: u8;
            let a: u8;
            if stage <= sg as u32 {
                r = (stage as u8).saturating_mul(10);
                g = (stage as u8).saturating_mul(10);
                b = (stage as u8).saturating_mul(10);
                a = 255; 
                s2.x = s2.x + randadj;
            } else if stage <= sg as u32 * 2 {
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255; 
            } else {
                let cc = (stage as u8 - sg as u8 * 2).saturating_mul(10);
                r = fc.0.saturating_sub(cc);
                g = fc.1.saturating_sub(cc);
                b = fc.2.saturating_sub(cc);
                a = 255; 
            }
            f(&(r, g, b, a), s1, s2, texidx, symidx);
        }
    }
}

/// sdl driver...
#[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
pub mod sdl;

/// web driver...
#[cfg(target_arch = "wasm32")]
pub mod web;

/// crossterm driver...
#[cfg(not(any(
    feature = "sdl",
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
)))]
pub mod cross;
