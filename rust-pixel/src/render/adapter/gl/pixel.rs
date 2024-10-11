// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::{
    gl::{
        color::GlColor, render_general2d::GlRenderGeneral2d, render_symbols::GlRenderSymbols,
        render_transition::GlRenderTransition, texture::GlRenderTexture, transform::GlTransform,
        GlRender, 
    },
    RenderCell,
};
use glow::HasContext;
use log::info;

pub struct GlPixel {
    r_sym: GlRenderSymbols,
    r_g2d: GlRenderGeneral2d,
    r_trans: GlRenderTransition,

    render_textures: Vec<GlRenderTexture>,

    canvas_width: u32,
    canvas_height: u32,

    clear_color: GlColor,
}

impl GlPixel {
    pub fn new(
        gl: &glow::Context,
        ver: &str,
        canvas_width: i32,
        canvas_height: i32,
        texw: i32,
        texh: i32,
        texdata: &[u8],
    ) -> Self {
        // gl render symbols for draw main buffer
        let mut r_sym = GlRenderSymbols::new(canvas_width as u32, canvas_height as u32);
        r_sym.init(gl, ver);
        r_sym.load_texture(gl, texw, texh, texdata);

        // gl render general2d for draw render texture
        let mut r_g2d = GlRenderGeneral2d::new(canvas_width as u32, canvas_height as u32);
        r_g2d.init(gl, ver);

        // gl render transition for transition effect
        let mut r_trans = GlRenderTransition::new(canvas_width as u32, canvas_height as u32);
        r_trans.init(gl, ver);

        unsafe {
            gl.enable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
        }

        // create 4 render texture for gl transition...
        let mut render_textures = vec![];
        for i in 0..4 {
            let w = if i == 2 { canvas_width as u32 } else { 40 * 16 };
            let h = if i == 2 { canvas_height as u32 } else { 25 * 16 };
            // let w = canvas_width as u32;
            // let h = canvas_height as u32;
            let rt = GlRenderTexture::new(gl, w, h).unwrap();
            info!("rt...{:?}", rt.texture);
            render_textures.push(rt);
        }

        Self {
            canvas_width: canvas_width as u32,
            canvas_height: canvas_height as u32,
            r_sym,
            r_g2d,
            r_trans,
            render_textures,
            clear_color: GlColor::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn bind_screen(&mut self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        }
    }

    // idx 0 - 3 : render to GlRenderTexture 0 - 3
    pub fn bind_target(&mut self, gl: &glow::Context, render_texture_idx: usize) {
        unsafe {
            let tex = &self.render_textures[render_texture_idx];
            gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                Some(tex.framebuffer),
            );
            info!("bind_target...{} {} {}", render_texture_idx, tex.width, tex.height);
            gl.viewport(0, 0, tex.width as i32, tex.height as i32);
        }
    }

    pub fn set_clear_color(&mut self, color: GlColor) {
        self.clear_color = color;
    }

    pub fn clear(&mut self, gl: &glow::Context) {
        unsafe {
            gl.clear_color(
                self.clear_color.r,
                self.clear_color.g,
                self.clear_color.b,
                self.clear_color.a,
            );
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn draw_general2d(
        &mut self,
        gl: &glow::Context,
        rtidx: usize,
        area: [f32; 4],
        transform: &GlTransform,
        color: &GlColor,
    ) {
        self.r_g2d
            .set_texture(gl, self.render_textures[rtidx].texture)
            .set_area(&area)
            .set_transform(transform)
            .set_color(color);
        self.r_g2d.prepare_draw(gl);
        self.r_g2d.draw(gl);
    }

    pub fn render_rbuf(
        &mut self,
        gl: &glow::Context,
        rbuf: &[RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
        self.r_sym.render_rbuf(gl, rbuf, ratio_x, ratio_y);
    }

    pub fn render_trans_frame(
        &mut self,
        gl: &glow::Context,
        sidx: usize,
        width: u32,
        height: u32,
        progress: f32,
    ) {
        self.r_trans.set_texture(
            width,
            height,
            self.render_textures[0].texture,
            self.render_textures[1].texture,
        );
        self.r_trans.draw_trans(gl, sidx, progress);
    }
}
