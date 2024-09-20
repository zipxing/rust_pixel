use crate::render::adapter::sdl::gl_color::GlColor;
use crate::render::adapter::sdl::gl_transform::GlTransform;
use crate::render::adapter::sdl::gl_pix::GlPix;
use crate::render::adapter::sdl::gl_pix::GlRenderMode;
use glow::HasContext;
use std::rc::Rc;

pub struct GlTexture {
    pub texture: glow::NativeTexture,
    framebuffer: glow::NativeFramebuffer,
    pub width: u32,
    pub height: u32,
    clear_color: GlColor,
    pub frames: Vec<GlFrame>,
}

#[derive(Clone)]
pub struct GlFrame {
    pub texture: glow::NativeTexture,
    pub width: f32,
    pub height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub uv_left: f32,
    pub uv_top: f32,
    pub uv_right: f32,
    pub uv_bottom: f32,
}

pub struct GlCell {
    pub frame: GlFrame,
}

impl GlTexture {
    pub fn new(gl: &glow::Context, source: &str) -> Result<Self, String> {
        let texture = unsafe { gl.create_texture().map_err(|e| e.to_string())? };
        let framebuffer = unsafe { gl.create_framebuffer().map_err(|e| e.to_string())? };

        let mut width = 0;
        let mut height = 0;
        let clear_color = GlColor::new(1.0, 1.0, 1.0, 0.0);
        let frames = Vec::new();

        // 加载图像
        let img = image::open(source).map_err(|e| e.to_string())?.to_rgba8();
        width = img.width();
        height = img.height();

        unsafe {
            gl.active_texture(glow::TEXTURE0);
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
                Some(&img),
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
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

            // 绑定帧缓冲
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
            width,
            height,
            clear_color,
            frames,
        })
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl
                .viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    pub fn free(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.texture);
            gl.delete_framebuffer(self.framebuffer);
        }
    }

    pub fn add_frame(&mut self, frame: GlFrame) {
        self.frames.push(frame);
    }

    pub fn get_texture(&self) -> glow::NativeTexture {
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

impl GlCell {
    pub fn new(frame: GlFrame) -> Self {
        Self { frame }
    }

    pub fn draw(
        &mut self,
        gl: &glow::Context,
        pix: &mut GlPix,
        transform: &GlTransform,
        color: &GlColor,
    ) {
        pix.bind_texture_atlas(gl, self.frame.texture);
        pix.prepare_draw(gl, GlRenderMode::PixCells, 16);

        let frame = &self.frame;
        let instance_buffer = &mut pix.instance_buffer;

        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = frame.origin_x;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = frame.origin_y;

        // UV attributes
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = frame.uv_left;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = frame.uv_top;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = frame.uv_right;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = frame.uv_bottom;

        // Transform attributes
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = transform.m00 * frame.width;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = transform.m10 * frame.height;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = transform.m01 * frame.width;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = transform.m11 * frame.height;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = transform.m20;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = transform.m21;

        // Color
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = color.r;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = color.g;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = color.b;
        pix.instance_buffer_at += 1;
        instance_buffer[pix.instance_buffer_at as usize] = color.a;
    }
}

pub fn load_texture(gl: &glow::Context, image_path: &str) -> Result<glow::NativeTexture, String> {
    let img = image::open(image_path)
        .map_err(|e| format!("Failed to load image: {}", e))?
        .flipv()
        .to_rgba8();
    let (width, height) = img.dimensions();
    unsafe {
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
            Some(&img),
        );
        gl.generate_mipmap(glow::TEXTURE_2D);
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
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        Ok(texture)
    }
}

