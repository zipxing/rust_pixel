// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements an Adapter class. Moreover,
//! all web related processing is handled here.

use crate::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
};
use crate::{
    render::{
        adapter::{
            render_border, render_logo, render_main_buffer, render_pixel_sprites, ARect, Adapter,
            AdapterBase, PointI32, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH,
        },
        buffer::Buffer,
        sprite::Sprites,
    },
    util::Rand,
    LOGO_FRAME,
};
use log::info;
use std::any::Any;
use std::time::Duration;
// use wasm_bindgen::prelude::*;
// use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct WebCell {
    pub r: u32,
    pub g: u32,
    pub b: u32,
    pub a: u32,
    pub texsym: u32,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub angle: u32,
    pub cx: i32,
    pub cy: i32,
}

pub struct WebAdapter {
    pub web_buf: Vec<WebCell>,
    pub base: AdapterBase,
    pub rd: Rand,
}

impl WebAdapter {
    pub fn new(pre: &str, gn: &str, project_path: &str) -> Self {
        Self {
            web_buf: vec![],
            base: AdapterBase::new(pre, gn, project_path),
            rd: Rand::new(),
        }
    }

    pub fn push_web_buffer(
        &mut self,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        texidx: usize,
        symidx: usize,
        s: ARect,
        angle: f64,
        ccp: &PointI32,
    ) {
        let mut wc: WebCell = Default::default();
        wc.r = r as u32;
        wc.g = g as u32;
        wc.b = b as u32;
        wc.a = a as u32;
        let y = symidx as u32 / 16u32 + (texidx as u32 / 2u32) * 16u32;
        let x = symidx as u32 % 16u32 + (texidx as u32 % 2u32) * 16u32;
        wc.texsym = y * 32u32 + x;
        wc.x = s.x;
        wc.y = s.y;
        wc.w = s.w;
        wc.h = s.h;
        if angle == 0.0 {
            wc.angle = 0u32;
        } else {
            let mut aa = (1.0 - angle / 180.0) * std::f64::consts::PI;
            let pi2 = std::f64::consts::PI * 2.0;
            while aa < 0.0 {
                aa += pi2;
            }
            while aa > pi2 {
                aa -= pi2;
            }
            wc.angle = (aa * 1000.0) as u32;
        }
        wc.cx = ccp.x as i32;
        wc.cy = ccp.y as i32;
        self.web_buf.push(wc);
    }
}

impl Adapter for WebAdapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String) {
        self.set_size(w, h)
            .set_ratiox(rx)
            .set_ratioy(ry)
            .set_pixel_size()
            .set_title(s);
        info!("web adapter init ... {:?}", (w, h));
    }

    fn get_base(&mut self) -> &mut AdapterBase {
        &mut self.base
    }

    fn reset(&mut self) {}

    fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH / self.base.ratio_x
    }

    fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT / self.base.ratio_y
    }

    fn poll_event(&mut self, _timeout: Duration, _es: &mut Vec<Event>) -> bool {
        false
    }

    fn render_buffer(
        &mut self,
        current_buffer: &Buffer,
        _p: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String> {
        self.web_buf.clear();
        let width = current_buffer.area.width;
        if stage <= LOGO_FRAME {
            let mut tv = vec![];
            render_logo(
                self.base.ratio_x,
                self.base.ratio_y,
                self.base.pixel_w,
                self.base.pixel_h,
                &mut self.rd,
                stage,
                |fc, _s1, s2, texidx, symidx| {
                    tv.push((fc.0, fc.1, fc.2, fc.3, texidx, symidx, s2));
                },
            );
            for tmp in tv {
                self.push_web_buffer(
                    tmp.0,
                    tmp.1,
                    tmp.2,
                    tmp.3,
                    tmp.4,
                    tmp.5,
                    tmp.6,
                    0.0,
                    &PointI32 { x: 0, y: 0 },
                );
            }
            return Ok(());
        }

        let cw = self.base.cell_w;
        let ch = self.base.cell_h;
        let rx = self.base.ratio_x;
        let ry = self.base.ratio_y;
        let mut rfunc =
            |fc: &(u8, u8, u8, u8), _s1: ARect, s2: ARect, texidx: usize, symidx: usize| {
                self.push_web_buffer(
                    fc.0,
                    fc.1,
                    fc.2,
                    fc.3,
                    texidx,
                    symidx,
                    s2,
                    0.0,
                    &PointI32 { x: 0, y: 0 },
                );
            };
        render_border(cw, ch, rx, ry, &mut rfunc);
        if stage > LOGO_FRAME {
            render_main_buffer(current_buffer, width, rx, ry, &mut rfunc);
        }
        if stage > LOGO_FRAME {
            for idx in 0..pixel_sprites.len() {
                if pixel_sprites[idx].is_pixel {
                    render_pixel_sprites(
                        &mut pixel_sprites[idx],
                        rx,
                        ry,
                        |fc, _s1, s2, texidx, symidx, angle, ccp| {
                            self.push_web_buffer(
                                fc.0, fc.1, fc.2, fc.3, texidx, symidx, s2, angle, &ccp,
                            );
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn set_cursor(&mut self, _x: u16, _y: u16) -> Result<(), String> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok((0, 0))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

macro_rules! web_event {
    ($ek:expr, $ei:expr, $($btn:expr)* ) => {
        Some(MouseEvent {
            kind: $ek$(($btn))*,
            column: $ei.5 as u16,
            row: $ei.6 as u16,
            modifiers: KeyModifiers::NONE,
        })
    };
}

/// Convert web I/O events to RustPixel event, for the sake of unified event processing
/// For keyboard and mouse event, please refer to the handle_input method in game/unblock/model.rs
pub fn input_events_from_web(t: u8, e: web_sys::Event, ratiox: f32, ratioy: f32) -> Option<Event> {
    let sym_width = PIXEL_SYM_WIDTH as f32;
    let sym_height = PIXEL_SYM_HEIGHT as f32;
    let mut mcte: Option<MouseEvent> = None;

    if let Some(key_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::KeyboardEvent>(&e) {
        assert!(t == 0);
        let kcc = (key_e.key_code(), key_e.char_code());
        match kcc.0 {
            32 | 48..=57 | 97..=122 => {
                let cte = KeyEvent::new(
                    KeyCode::Char(char::from_u32(kcc.0).unwrap()),
                    KeyModifiers::NONE,
                );
                return Some(Event::Key(cte));
            }
            _ => {
                return None;
            }
        }
    }

    if let Some(mouse_e) = wasm_bindgen::JsCast::dyn_ref::<web_sys::MouseEvent>(&e) {
        let medat = (
            mouse_e.buttons(),
            mouse_e.screen_x(),
            mouse_e.screen_y(),
            mouse_e.client_x(),
            mouse_e.client_y(),
            mouse_e.x(),
            mouse_e.y(),
        );
        match t {
            1 => {
                mcte = web_event!(Up, medat, Left);
            }
            2 => {
                mcte = web_event!(Down, medat, Left);
            }
            3 => {
                if medat.0 == 1 {
                    mcte = web_event!(Drag, medat, Left);
                } else {
                    mcte = web_event!(Moved, medat,);
                }
            }
            _ => {}
        }
    }
    if let Some(mut mc) = mcte {
        mc.column /= (sym_width / ratiox) as u16;
        mc.row /= (sym_height / ratioy) as u16;
        if mc.column >= 1 {
            mc.column -= 1;
        }
        if mc.row >= 1 {
            mc.row -= 1;
        }
        return Some(Event::Mouse(mc));
    }
    None
}
