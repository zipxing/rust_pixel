// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::wgpu_impl::{
    shader::WgpuShader,
    shader_source::{VERTEX_SRC_TRANS, TRANS_FRAGMENT_SRC},
    texture::WgpuRenderTexture,
    WgpuRender, WgpuRenderBase,
};
use wgpu::util::DeviceExt;

pub struct WgpuRenderTransition {
    pub base: WgpuRenderBase,
    progress: f32,
    texture1: Option<WgpuRenderTexture>,
    texture2: Option<WgpuRenderTexture>,
    
    // WGPU specific
    vertex_buffer: Option<wgpu::Buffer>,
    uniform_buffer: Option<wgpu::Buffer>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    uniform_bind_group: Option<wgpu::BindGroup>,
    texture_bind_group: Option<wgpu::BindGroup>,
}

impl WgpuRender for WgpuRenderTransition {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let base = WgpuRenderBase::new(2, canvas_width, canvas_height);
        
        Self {
            base,
            progress: 0.0,
            texture1: None,
            texture2: None,
            
            vertex_buffer: None,
            uniform_buffer: None,
            bind_group_layout: None,
            texture_bind_group_layout: None,
            uniform_bind_group: None,
            texture_bind_group: None,
        }
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let shader = WgpuShader::new(
            device,
            "transition_shader",
            VERTEX_SRC_TRANS,
            TRANS_FRAGMENT_SRC,
        );

        // Create bind group layouts
        self.texture_bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transition_texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        }));

        self.bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transition_uniform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }));

        // Create render pipeline
        let vertex_layout = &[wgpu::VertexBufferLayout {
            array_stride: 4 * 4, // 4 floats * 4 bytes
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2, // position
                },
                wgpu::VertexAttribute {
                    offset: 2 * 4,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2, // tex_coords
                },
            ],
        }];

        let bind_group_layouts = &[
            self.texture_bind_group_layout.as_ref().unwrap(),
            self.bind_group_layout.as_ref().unwrap(),
        ];

        let pipeline = shader.create_render_pipeline(
            device,
            format,
            vertex_layout,
            bind_group_layouts,
            Some("transition_render_pipeline"),
        );

        self.base.render_pipeline = Some(pipeline);
        self.base.shader = Some(shader);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Create vertex buffer for fullscreen quad
        let quad_vertices: [f32; 16] = [
            // position, tex_coords
            -1.0, -1.0,   0.0, 1.0,
            -1.0,  1.0,   0.0, 0.0,
             1.0,  1.0,   1.0, 0.0,
             1.0, -1.0,   1.0, 1.0,
        ];

        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("transition_vertex_buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        // Create uniform buffer for progress
        self.uniform_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transition_uniform_buffer"),
            size: 16, // 4 floats (progress + padding)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.base.buffers.push(self.vertex_buffer.as_ref().unwrap().clone());
        self.base.buffers.push(self.uniform_buffer.as_ref().unwrap().clone());
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Update uniform buffer with progress
        let uniform_data = [self.progress, 0.0, 0.0, 0.0]; // progress + padding
        if let Some(uniform_buffer) = &self.uniform_buffer {
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&uniform_data));
        }

        // Create bind groups if needed
        self.create_bind_groups(device);
    }

    fn draw(&mut self, render_pass: &mut wgpu::RenderPass) {
        if let Some(pipeline) = &self.base.render_pipeline {
            render_pass.set_pipeline(pipeline);
            
            if let Some(texture_bind_group) = &self.texture_bind_group {
                render_pass.set_bind_group(0, texture_bind_group, &[]);
            }
            
            if let Some(uniform_bind_group) = &self.uniform_bind_group {
                render_pass.set_bind_group(1, uniform_bind_group, &[]);
            }
            
            if let Some(vertex_buffer) = &self.vertex_buffer {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            }
            
            render_pass.draw(0..4, 0..1);
        }
    }

    fn cleanup(&mut self, device: &wgpu::Device) {
        // Cleanup resources if needed
    }
}

impl WgpuRenderTransition {
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn set_textures(&mut self, texture1: &WgpuRenderTexture, texture2: &WgpuRenderTexture) {
        // Store references to textures for bind group creation
        // In a real implementation, you might want to store these differently
    }

    pub fn draw_trans(&mut self, render_pass: &mut wgpu::RenderPass, _sidx: usize) {
        // For now, just use the basic transition
        // In the future, sidx could select different transition shaders
        self.draw(render_pass);
    }

    fn create_bind_groups(&mut self, device: &wgpu::Device) {
        // Create uniform bind group
        if self.uniform_bind_group.is_none() {
            if let (Some(layout), Some(buffer)) = (&self.bind_group_layout, &self.uniform_buffer) {
                self.uniform_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                    label: Some("transition_uniform_bind_group"),
                }));
            }
        }

        // TODO: Create texture bind group when textures are set
        // This would require storing the texture references properly
    }
} 