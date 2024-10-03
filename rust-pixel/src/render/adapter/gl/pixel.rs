// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::{
    gl::{
        color::GlColor, render_general2d::GlRenderGeneral2d, render_symbols::GlRenderSymbols,
        render_transition::GlRenderTransition, texture::GlRenderTexture, transform::GlTransform,
        GlRender, GlRenderMode,
    },
    RenderCell,
};
use glow::HasContext;
use log::info;

pub struct GlPixel {
    pub render_mode: GlRenderMode,

    pub r_sym: GlRenderSymbols,
    pub r_g2d: GlRenderGeneral2d,
    pub r_trans: GlRenderTransition,

    pub render_textures: Vec<GlRenderTexture>,

    pub current_texture_atlas: Option<glow::Texture>,

    pub canvas_width: u32,
    pub canvas_height: u32,

    pub clear_color: GlColor,
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
        let mut r_sym = GlRenderSymbols::new(canvas_width as u32, canvas_height as u32);
        r_sym.init(gl, "#version 330 core");
        r_sym.load_texture(gl, texw, texh, texdata);

        let mut r_g2d = GlRenderGeneral2d::new(canvas_width as u32, canvas_height as u32);
        r_g2d.init(gl, "#version 330 core");

        let mut r_trans = GlRenderTransition::new(canvas_width as u32, canvas_height as u32);
        r_trans.init(gl, "#version 330 core");

        // 初始化缓冲区
        unsafe {
            info!("GL EDB....");
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
        for _i in 0..4 {
            let rt = GlRenderTexture::new(gl, canvas_width as u32, canvas_height as u32).unwrap();
            info!("rt...{:?}", rt.texture);
            render_textures.push(rt);
        }

        Self {
            render_mode: GlRenderMode::None,
            canvas_width: canvas_width as u32,
            canvas_height: canvas_height as u32,
            r_sym,
            r_g2d,
            r_trans,
            render_textures,
            current_texture_atlas: None,
            clear_color: GlColor::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn bind(&mut self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        }
    }

    // idx 0 - 3 : render to GlRenderTexture 0 - 3
    pub fn bind_target(&mut self, gl: &glow::Context, idx: usize) {
        unsafe {
            gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                Some(self.render_textures[idx].framebuffer),
            );
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        }
    }

    pub fn set_clear_color(&mut self, color: GlColor) {
        self.clear_color = color;
    }

    pub fn clear(&mut self, gl: &glow::Context) {
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
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
            .set_transform(&transform)
            .set_color(&color);
        self.r_g2d.prepare_draw(gl, GlRenderMode::General2D, 0);
        self.r_g2d.draw(gl);
        self.r_sym.base.textures_binded = false;
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
