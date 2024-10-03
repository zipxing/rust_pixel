pub mod color;
pub mod transform;
pub mod texture;
pub mod shader;
pub mod pixel;
pub mod shader_source;
pub mod render_symbols;
pub mod render_transition;
pub mod render_general2d;

// use crate::render::adapter::gl::shader::GlShader;
use shader::GlShader;

pub trait GlRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self
    where
        Self: Sized;

    fn get_base(&mut self) -> &mut GlRenderBase;

    fn create_shader(
        &mut self,
        gl: &glow::Context,
        ver: &str,
    );

    unsafe fn create_buffer(&mut self, gl: &glow::Context);

    fn init(&mut self, gl: &glow::Context, ver: &str) {
        self.create_shader(gl, ver);
        unsafe { self.create_buffer(gl) };
    }

    fn prepare_draw(&mut self, gl: &glow::Context);

    fn draw(&mut self, gl: &glow::Context);

    fn cleanup(&mut self, gl: &glow::Context);
}

pub struct GlRenderBase {
    pub id: usize,
    pub shader: Vec<GlShader>,
    pub shader_binded: bool,
    pub vao: Option<glow::VertexArray>,
    pub gl_buffers: Vec<glow::Buffer>,
    pub textures: Vec<glow::Texture>,
    pub textures_binded: bool,
    pub canvas_width: u32,
    pub canvas_height: u32,
}


