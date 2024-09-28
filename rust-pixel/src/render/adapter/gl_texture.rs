// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::gl_color::GlColor;
use glow::HasContext;
// use log::info;

// render target texture...
pub struct GlRenderTexture {
    pub framebuffer: glow::Framebuffer,
    pub texture: glow::Texture,
    pub width: u32,
    pub height: u32,
}

impl GlRenderTexture {
    pub fn new(gl: &glow::Context, width: u32, height: u32) -> Result<Self, String> {
        unsafe {
            let framebuffer = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
                // glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
                // glow::LINEAR as i32,
            );

            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );

            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err("Framebuffer is not complete".to_string());
            }

            // 解绑帧缓冲和纹理
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.bind_texture(glow::TEXTURE_2D, None);

            Ok(Self {
                framebuffer,
                texture,
                width,
                height,
            })
        }
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    pub fn unbind(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn get_texture(&self) -> glow::Texture {
        self.texture
    }

    pub fn free(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.framebuffer);
            gl.delete_texture(self.texture);
        }
    }
}

pub struct GlTexture {
    pub texture: glow::Texture,
    pub width: u32,
    pub height: u32,
    clear_color: GlColor,
    framebuffer: glow::Framebuffer,
}

#[derive(Clone)]
pub struct GlCell {
    pub texture: glow::Texture,
    pub width: f32,
    pub height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub uv_left: f32,
    pub uv_top: f32,
    pub uv_width: f32,
    pub uv_height: f32,
}

impl GlTexture {
    pub fn new(gl: &glow::Context, w:i32, h: i32, data: &[u8]) -> Result<Self, String> {
        let texture = unsafe { gl.create_texture().map_err(|e| e.to_string())? };
        let framebuffer = unsafe { gl.create_framebuffer().map_err(|e| e.to_string())? };

        let clear_color = GlColor::new(1.0, 1.0, 1.0, 1.0);

        // let img = image::open(source).map_err(|e| e.to_string())?.to_rgba8();
        // let width = img.width();
        // let height = img.height();
        // info!("opengl texture...(width{} height{})", width, height);

        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                w,
                h,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&data),
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
                // glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
                // glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        Ok(Self {
            texture,
            framebuffer,
            width: w as u32,
            height: h as u32,
            clear_color,
        })
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    pub fn free(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.texture);
            gl.delete_framebuffer(self.framebuffer);
        }
    }

    pub fn get_texture(&self) -> glow::Texture {
        self.texture
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn set_clear_color(&mut self, color: GlColor) {
        self.clear_color = color;
    }
}

