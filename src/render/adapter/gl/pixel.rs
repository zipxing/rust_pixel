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

// open gl shader wrapper...
pub struct GlPixel {
    r_sym: GlRenderSymbols,
    r_g2d: GlRenderGeneral2d,
    r_trans: GlRenderTransition,

    render_textures: Vec<GlRenderTexture>,

    pub canvas_width: u32,
    pub canvas_height: u32,

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
        let rt_hidden = [true, true, false, true];
        for i in 0..4 {
            let w = canvas_width as u32;
            let h = canvas_height as u32;
            let rt = GlRenderTexture::new(gl, w, h, rt_hidden[i]).unwrap();
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

    // bind none for render to screen...
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
            // info!("bind_target...{} {} {}", render_texture_idx, tex.width, tex.height);
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

    pub fn get_render_texture_hidden(&mut self, rtidx: usize) -> bool {
        self.render_textures[rtidx].is_hidden
    }

    pub fn set_render_texture_hidden(&mut self, rtidx: usize, h: bool) {
        self.render_textures[rtidx].is_hidden = h;
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
        progress: f32,
    ) {
        self.r_trans.set_texture(
            self.canvas_width,
            self.canvas_height,
            self.render_textures[0].texture,
            self.render_textures[1].texture,
        );
        self.r_trans.draw_trans(gl, sidx, progress);
    }
}

// Implementation of the unified PixelRenderer trait for OpenGL backend
impl crate::render::pixel_renderer::PixelRenderer for GlPixel {
    fn get_canvas_size(&self) -> (u32, u32) {
        (self.canvas_width, self.canvas_height)
    }
    
    fn draw_general2d(
        &mut self,
        context: &mut crate::render::pixel_renderer::RenderContext,
        rtidx: usize,
        area: [f32; 4],
        transform: &crate::render::pixel_renderer::UnifiedTransform,
        color: &crate::render::pixel_renderer::UnifiedColor,
    ) -> Result<(), String> {
        if let crate::render::pixel_renderer::RenderContext::OpenGL { gl } = context {
            // Convert unified types to OpenGL-specific types
            let gl_transform = transform.to_gl_transform();
            let gl_color = color.to_gl_color();
            
            // Use existing OpenGL implementation
            self.draw_general2d(*gl, rtidx, area, &gl_transform, &gl_color);
            Ok(())
        } else {
            Err("Invalid context type for OpenGL renderer".to_string())
        }
    }
    
    fn render_transition_frame(
        &mut self,
        context: &mut crate::render::pixel_renderer::RenderContext,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        if let crate::render::pixel_renderer::RenderContext::OpenGL { gl } = context {
            // Use existing OpenGL implementation
            self.render_trans_frame(*gl, shader_idx, progress);
            Ok(())
        } else {
            Err("Invalid context type for OpenGL renderer".to_string())
        }
    }
    
    fn get_render_texture_hidden(&self, rtidx: usize) -> bool {
        if rtidx < self.render_textures.len() {
            self.render_textures[rtidx].is_hidden
        } else {
            true // Out of bounds textures are considered hidden
        }
    }
    
    fn set_render_texture_hidden(&mut self, rtidx: usize, hidden: bool) {
        if rtidx < self.render_textures.len() {
            self.render_textures[rtidx].is_hidden = hidden;
        }
    }
    
    fn render_symbols_to_texture(
        &mut self,
        context: &mut crate::render::pixel_renderer::RenderContext,
        rbuf: &[crate::render::adapter::RenderCell],
        rtidx: usize,
        ratio_x: f32,
        ratio_y: f32,
    ) -> Result<(), String> {
        if let crate::render::pixel_renderer::RenderContext::OpenGL { gl } = context {
            // Bind the target render texture
            self.bind_target(*gl, rtidx);
            
            // Clear the target
            self.clear(*gl);
            
            // Render symbols to the bound target
            self.render_rbuf(*gl, rbuf, ratio_x, ratio_y);
            
            Ok(())
        } else {
            Err("Invalid context type for OpenGL renderer".to_string())
        }
    }
    
    fn set_clear_color(&mut self, color: &crate::render::pixel_renderer::UnifiedColor) {
        let gl_color = color.to_gl_color();
        self.set_clear_color(gl_color);
    }
    
    fn clear(&mut self, context: &mut crate::render::pixel_renderer::RenderContext) {
        if let crate::render::pixel_renderer::RenderContext::OpenGL { gl } = context {
            self.clear(*gl);
        }
    }
    
    fn bind_render_target(&mut self, rtidx: Option<usize>) {
        // This method would need to store the GL context to use later
        // For now, we can't implement it without changing the GL context handling
        // The existing bind_target and bind_screen methods require a GL context parameter
    }
    
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
