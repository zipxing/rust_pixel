use crate::render::adapter::sdl::color::Color;
use crate::render::adapter::sdl::transform::Transform;
use crate::render::adapter::sdl::pix::Pix;
use crate::render::adapter::sdl::pix::RenderMode;
use glow::HasContext;

#[derive(Clone, Copy, PartialEq)]
pub struct Texture {
    pub gl_texture: glow::NativeTexture,
    pub framebuffer: Option<glow::NativeFramebuffer>,
    pub width: u32,
    pub height: u32,
    pub clear_color: Color,
    pub ready: bool,
    pub frames: Vec<Frame>,
}

pub struct Frame {
    pub texture: glow::NativeTexture,
    pub width: f32,
    pub height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub uv_left: f32,
    pub uv_top: f32,
    pub uv_right: f32,
    pub uv_bottom: f32,
    pub time: f32,
}

pub struct Cell {
    pub frames: Vec<Frame>,
    pub current_frame: usize,
}

impl Cell {
    pub fn draw(
        &mut self,
        pix: &mut Pix,
        transform: &Transform,
        color: &Color,
    ) {
        let frame = &self.frames[self.current_frame];
        pix.bind_texture_atlas(frame.texture);
        pix.prepare_draw(RenderMode::PixCells, 16, None);

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

