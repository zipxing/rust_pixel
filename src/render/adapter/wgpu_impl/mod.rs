// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! WGPU rendering implementation modules
//! Similar to gl/ modules but using WGPU instead of OpenGL

pub mod color;
pub mod transform;
pub mod texture;
pub mod shader;
pub mod pixel;
pub mod shader_source;
pub mod render_symbols;
pub mod render_transition;
pub mod render_general2d;

use shader::WgpuShader;

/// Base trait for WGPU renderers, similar to GlRender
pub trait WgpuRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self
    where
        Self: Sized;

    fn get_base(&mut self) -> &mut WgpuRenderBase;

    fn create_shader(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    );

    fn create_buffer(&mut self, device: &wgpu::Device);

    fn init(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.create_shader(device, format);
        self.create_buffer(device);
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);

    fn draw(&mut self, render_pass: &mut wgpu::RenderPass);

    fn cleanup(&mut self, device: &wgpu::Device);
}

/// Base structure for WGPU renderers, similar to GlRenderBase
pub struct WgpuRenderBase {
    pub id: usize,
    pub shader: Option<WgpuShader>,
    pub shader_binded: bool,
    pub buffers: Vec<wgpu::Buffer>,
    pub textures: Vec<wgpu::Texture>,
    pub texture_views: Vec<wgpu::TextureView>,
    pub samplers: Vec<wgpu::Sampler>,
    pub bind_groups: Vec<wgpu::BindGroup>,
    pub render_pipeline: Option<wgpu::RenderPipeline>,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

impl WgpuRenderBase {
    pub fn new(id: usize, canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            id,
            shader: None,
            shader_binded: false,
            buffers: Vec::new(),
            textures: Vec::new(),
            texture_views: Vec::new(),
            samplers: Vec::new(),
            bind_groups: Vec::new(),
            render_pipeline: None,
            canvas_width,
            canvas_height,
        }
    }
} 