// RustPixel
// copyright zipxing@hotmail.com 2022~2024

#![allow(unused_variables)]
use crate::{
    event::Event,
    render::{buffer::Buffer, sprite::Sprites},
    util::{Rand, Rect},
};
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use crate::{
    render::adapter::gl::{color::GlColor, pixel::GlPixel, transform::GlTransform},
    render::style::Color,
    util::{ARect, PointF32, PointI32, PointU16},
    LOGO_FRAME,
};
use std::any::Any;
use std::time::Duration;
// use log::info;

// opengl codes...
pub mod gl;

// merge l, u, ext1, ext2 to a single image
// c64l.png  c64u.png    -->  c64.png
// c64e1.png c64e2.png
// Add more files to this list when needed,max 255 textures...
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const PIXEL_TEXTURE_FILES: [&str; 1] = ["assets/pix/c64.png"];

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

// pre-render cell...
// this struct used for opengl render and webgl render...
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderCell {
    pub fcolor: (f32, f32, f32, f32),
    pub bcolor: Option<(f32, f32, f32, f32)>,
    pub texsym: usize,
    pub x: f32,
    pub y: f32,
    pub w: u32,
    pub h: u32,
    pub angle: f32,
    pub cx: f32,
    pub cy: f32,
}

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
    pub rd: Rand,
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    pub gl: Option<glow::Context>,
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    pub gl_pixel: Option<GlPixel>,
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
            rd: Rand::new(),
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            gl: None,
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            gl_pixel: None,
        }
    }
}

pub trait Adapter {
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String);
    fn reset(&mut self);
    fn get_base(&mut self) -> &mut AdapterBase;
    fn poll_event(&mut self, timeout: Duration, ev: &mut Vec<Event>) -> bool;

    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
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

    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    fn main_render_pass(&mut self) {
        let bs = self.get_base();

        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            pix.bind_screen(gl);

            // draw render_texture 2 ( main buffer )
            let t = GlTransform::new();
            let c = GlColor::new(1.0, 1.0, 1.0, 1.0);
            pix.draw_general2d(gl, 2, [0.0, 0.0, 1.0, 1.0], &t, &c);

            // draw render_texture 3 ( gl transition )
            let pcw = pix.canvas_width as f32;
            let pch = pix.canvas_height as f32;
            let pw = 40.0 * PIXEL_SYM_WIDTH;
            let ph = 25.0 * PIXEL_SYM_HEIGHT;

            let mut t2 = GlTransform::new();
            t2.scale(pw / pcw, ph / pch);
            pix.draw_general2d(
                gl,
                3,
                [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch],
                &t2,
                &c,
            );
        }
    }

    // draw buffer to render texture...
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    fn draw_buffer_to_texture(&mut self, buf: &Buffer, rtidx: usize) {
        let rbuf = self.buffer_to_render_buffer(buf);
        // For debug...
        // self.draw_render_buffer(&rbuf, rtidx, true);
        self.draw_render_buffer_to_texture(&rbuf, rtidx, false);
    }

    // draw render buffer to render texture...
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool) {
        let bs = self.get_base();
        let rx = bs.ratio_x;
        let ry = bs.ratio_y;
        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            pix.bind_target(gl, rtidx);
            if debug {
                // set red background for debug...
                pix.set_clear_color(GlColor::new(1.0, 0.0, 0.0, 1.0));
            } else {
                pix.set_clear_color(GlColor::new(0.0, 0.0, 0.0, 1.0));
            }
            pix.clear(gl);
            pix.render_rbuf(gl, rbuf, rx, ry);
        }
    }

    // buffer to render buffer...
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    fn buffer_to_render_buffer(&mut self, cb: &Buffer) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let rx = self.get_base().ratio_x;
        let ry = self.get_base().ratio_y;
        let pz = PointI32 { x: 0, y: 0 };
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         _s0: ARect,
                         _s1: ARect,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };
        render_main_buffer(cb, cb.area.width, rx, ry, true, &mut rfunc);
        rbuf
    }

    // draw main buffer & pixel sprites to render buffer...
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    fn draw_all_to_render_buffer(
        &mut self,
        cb: &Buffer,
        _pb: &Buffer,
        ps: &mut Vec<Sprites>,
        stage: u32,
    ) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let width = cb.area.width;
        let pz = PointI32 { x: 0, y: 0 };

        // render logo...
        if stage <= LOGO_FRAME {
            render_logo(
                self.get_base().ratio_x,
                self.get_base().ratio_y,
                self.get_base().pixel_w,
                self.get_base().pixel_h,
                &mut self.get_base().rd,
                stage,
                |fc, _s1, s2, texidx, symidx| {
                    push_render_buffer(&mut rbuf, fc, &None, texidx, symidx, s2, 0.0, &pz);
                },
            );
            return rbuf;
        }

        let cw = self.get_base().cell_w;
        let ch = self.get_base().cell_h;
        let rx = self.get_base().ratio_x;
        let ry = self.get_base().ratio_y;
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         _s0: ARect,
                         _s1: ARect,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };

        // render windows border, only at sdl mode
        #[cfg(feature = "sdl")]
        render_border(cw, ch, rx, ry, &mut rfunc);

        // render main buffer...
        if stage > LOGO_FRAME {
            render_main_buffer(cb, width, rx, ry, false, &mut rfunc);
        }

        // render pixel_sprites...
        if stage > LOGO_FRAME {
            for item in ps {
                if item.is_pixel && !item.is_hidden {
                    render_pixel_sprites(
                        item,
                        rx,
                        ry,
                        |fc, bc, _s0, _s1, s2, texidx, symidx, angle, ccp| {
                            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, angle, &ccp);
                        },
                    );
                }
            }
        }
        rbuf
    }

    fn as_any(&mut self) -> &mut dyn Any;
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
fn push_render_buffer(
    rbuf: &mut Vec<RenderCell>,
    fc: &(u8, u8, u8, u8),
    bgc: &Option<(u8, u8, u8, u8)>,
    texidx: usize,
    symidx: usize,
    s: ARect,
    angle: f64,
    ccp: &PointI32,
) {
    let mut wc = RenderCell {
        fcolor: (
            fc.0 as f32 / 255.0,
            fc.1 as f32 / 255.0,
            fc.2 as f32 / 255.0,
            fc.3 as f32 / 255.0,
        ),
        ..Default::default()
    };
    if let Some(bc) = bgc {
        wc.bcolor = Some((
            bc.0 as f32 / 255.0,
            bc.1 as f32 / 255.0,
            bc.2 as f32 / 255.0,
            bc.3 as f32 / 255.0,
        ));
    } else {
        wc.bcolor = None;
    }
    let x = symidx as u32 % 16u32 + (texidx as u32 % 2u32) * 16u32;
    let y = symidx as u32 / 16u32 + (texidx as u32 / 2u32) * 16u32;
    wc.texsym = (y * 32u32 + x) as usize;
    wc.x = s.x as f32 + PIXEL_SYM_WIDTH;
    wc.y = s.y as f32 + PIXEL_SYM_HEIGHT;
    wc.w = s.w;
    wc.h = s.h;
    if angle == 0.0 {
        wc.angle = angle as f32;
    } else {
        let mut aa = (1.0 - angle / 180.0) * std::f64::consts::PI;
        let pi2 = std::f64::consts::PI * 2.0;
        while aa < 0.0 {
            aa += pi2;
        }
        while aa > pi2 {
            aa -= pi2;
        }
        wc.angle = aa as f32;
    }
    wc.cx = ccp.x as f32;
    wc.cy = ccp.y as f32;
    rbuf.push(wc);
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
fn render_helper(
    cell_w: u16,
    r: PointF32,
    // rx: f32,
    // ry: f32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    // px: u16,
    // py: u16,
    is_border: bool,
) -> (ARect, ARect, ARect, usize, usize) {
    let w = PIXEL_SYM_WIDTH as i32;
    let h = PIXEL_SYM_HEIGHT as i32;
    let dstx = i as u16 % cell_w;
    let dsty = i as u16 / cell_w;
    let tex_count = PIXEL_TEXTURE_FILES.len() as u8 * 4;
    let tx = if sh.1 < tex_count { sh.1 as usize } else { 1 };
    let srcy = sh.0 as u32 / w as u32 + (tx as u32 / 2u32) * w as u32;
    let srcx = sh.0 as u32 % w as u32 + (tx as u32 % 2u32) * w as u32;
    // let bsrcy = 160u32 / w as u32 + (1u32 / 2u32) * w as u32;
    // let bsrcx = 160u32 % w as u32 + (1u32 % 2u32) * w as u32;
    let bsrcy = 160u32 / w as u32;
    let bsrcx = 160u32 % w as u32 + w as u32;

    (
        // background sym rect in texture(sym=160 tex=1)
        ARect {
            x: (w + 1) * bsrcx as i32,
            y: (h + 1) * bsrcy as i32,
            w: w as u32,
            h: h as u32,
        },
        // sym rect in texture
        ARect {
            x: (w + 1) * srcx as i32,
            y: (h + 1) * srcy as i32,
            w: w as u32,
            h: h as u32,
        },
        // dst rect in render texture
        ARect {
            x: (dstx + if is_border { 0 } else { 1 }) as i32 * (w as f32 / r.x) as i32 + p.x as i32,
            y: (dsty + if is_border { 0 } else { 1 }) as i32 * (h as f32 / r.y) as i32 + p.y as i32,
            w: (w as f32 / r.x) as u32,
            h: (h as f32 / r.y) as u32,
        },
        // texture id
        tx,
        // sym id
        sh.0 as usize,
    )
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_pixel_sprites<F>(pixel_spt: &mut Sprites, rx: f32, ry: f32, mut f: F)
where
    // rgba, back rgba, back rect, sym rect, dst rect, tex, sym, angle, center point
    F: FnMut(
        &(u8, u8, u8, u8),
        &Option<(u8, u8, u8, u8)>,
        ARect,
        ARect,
        ARect,
        usize,
        usize,
        f64,
        PointI32,
    ),
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
            let (s0, s1, s2, texidx, symidx) = render_helper(
                pw,
                PointF32 { x: rx, y: ry },
                i,
                sh,
                PointU16 { x: px, y: py },
                false,
            );
            let x = i % pw as usize;
            let y = i / pw as usize;
            // center point ...
            let ccp = PointI32 {
                x: ((pw as f32 / 2.0 - x as f32) * PIXEL_SYM_WIDTH / rx) as i32,
                y: ((ph as f32 / 2.0 - y as f32) * PIXEL_SYM_HEIGHT / ry) as i32,
            };
            let mut fc = sh.2.get_rgba();
            fc.3 = s.alpha;
            let bc;
            if sh.3 != Color::Reset {
                let mut brgba = sh.3.get_rgba();
                brgba.3 = s.alpha;
                bc = Some(brgba);
            } else {
                bc = None;
            }
            f(&fc, &bc, s0, s1, s2, texidx, symidx, s.angle, ccp);
        }
    }
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_main_buffer<F>(buf: &Buffer, width: u16, rx: f32, ry: f32, border: bool, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    for (i, cell) in buf.content.iter().enumerate() {
        // symidx, texidx, fg, bg
        let sh = cell.get_cell_info();
        let (s0, s1, s2, texidx, symidx) = render_helper(
            width,
            PointF32 { x: rx, y: ry },
            i,
            &sh,
            PointU16 { x: 0, y: 0 },
            border,
        );
        let fc = sh.2.get_rgba();
        let bc = if sh.3 != Color::Reset {
            Some(sh.3.get_rgba())
        } else {
            None
        };
        f(&fc, &bc, s0, s1, s2, texidx, symidx);
    }
}

#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    let sh_top = (102u8, 1u8, Color::Indexed(7), Color::Reset);
    let sh_other = (24u8, 2u8, Color::Indexed(7), Color::Reset);
    let sh_close = (214u8, 1u8, Color::Indexed(7), Color::Reset);

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
            let (s0, s1, s2, texidx, symidx) = render_helper(
                cell_w + 2,
                PointF32 { x: rx, y: ry },
                n * (cell_w as usize + 2) + m,
                rsh,
                PointU16 { x: 0, y: 0 },
                true,
            );
            let fc = rsh.2.get_rgba();
            let bc = None;
            f(&fc, &bc, s0, s1, s2, texidx, symidx);
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

            let (_s0, s1, mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                PointF32 { x: rx, y: ry },
                sci,
                &(
                    PIXEL_LOGO[sci * 3],
                    PIXEL_LOGO[sci * 3 + 2],
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]),
                    Color::Reset,
                ),
                PointU16 {
                    x: spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                    y: sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                },
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
                s2.x += randadj;
            } else if stage <= sg as u32 * 2 {
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255;
            } else {
                let cc = (stage as u8 - sg * 2).saturating_mul(10);
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
