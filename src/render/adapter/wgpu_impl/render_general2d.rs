// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use crate::render::adapter::wgpu_impl::{
    color::WgpuColor,
    shader::WgpuShader,
    shader_source::{GENERAL2D_VERTEX_SRC, GENERAL2D_FRAGMENT_SRC},
    texture::WgpuRenderTexture,
    transform::WgpuTransform,
    WgpuRender, WgpuRenderBase,
};
use wgpu::util::DeviceExt;

pub struct WgpuRenderGeneral2d {
    pub base: WgpuRenderBase,
    area: [f32; 4],
    transform: WgpuTransform,
    color: WgpuColor,
    texture: Option<WgpuRenderTexture>,
    
    // WGPU specific
    vertex_buffer: Option<wgpu::Buffer>,
    uniform_buffer: Option<wgpu::Buffer>,
    color_uniform_buffer: Option<wgpu::Buffer>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    color_bind_group_layout: Option<wgpu::BindGroupLayout>,
    uniform_bind_group: Option<wgpu::BindGroup>,
    texture_bind_group: Option<wgpu::BindGroup>,
    color_bind_group: Option<wgpu::BindGroup>,
}

impl WgpuRender for WgpuRenderGeneral2d {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let base = WgpuRenderBase::new(1, canvas_width, canvas_height);
        
        Self {
            base,
            area: [0.0, 0.0, 1.0, 1.0],
            transform: WgpuTransform::orthographic(
                0.0,
                canvas_width as f32,
                canvas_height as f32,
                0.0,
            ),
            color: WgpuColor::new(1.0, 1.0, 1.0, 1.0),
            texture: None,
            
            vertex_buffer: None,
            uniform_buffer: None,
            color_uniform_buffer: None,
            bind_group_layout: None,
            texture_bind_group_layout: None,
            color_bind_group_layout: None,
            uniform_bind_group: None,
            texture_bind_group: None,
            color_bind_group: None,
        }
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let shader = WgpuShader::new(
            device,
            "general2d_shader",
            GENERAL2D_VERTEX_SRC,
            GENERAL2D_FRAGMENT_SRC,
        );

        // Create bind group layouts
        self.bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("general2d_uniform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        }));

        self.texture_bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("general2d_texture_bind_group_layout"),
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
            ],
        }));

        self.color_bind_group_layout = Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("general2d_color_bind_group_layout"),
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
            self.bind_group_layout.as_ref().unwrap(),
            self.texture_bind_group_layout.as_ref().unwrap(),
            self.color_bind_group_layout.as_ref().unwrap(),
        ];

        let pipeline = shader.create_render_pipeline(
            device,
            format,
            vertex_layout,
            bind_group_layouts,
            Some("general2d_render_pipeline"),
        );

        self.base.render_pipeline = Some(pipeline);
        self.base.shader = Some(shader);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Create vertex buffer for quad
        let quad_vertices: [f32; 16] = [
            // position, tex_coords
            -1.0, -1.0,   0.0, 0.0,
            -1.0,  1.0,   0.0, 1.0,
             1.0,  1.0,   1.0, 1.0,
             1.0, -1.0,   1.0, 0.0,
        ];

        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("general2d_vertex_buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        // Create uniform buffers
        self.uniform_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("general2d_uniform_buffer"),
            size: 80, // 4x4 matrix + 4 floats for area
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.color_uniform_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("general2d_color_uniform_buffer"),
            size: 16, // 4 floats for color
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        self.base.buffers.push(self.vertex_buffer.as_ref().unwrap().clone());
        self.base.buffers.push(self.uniform_buffer.as_ref().unwrap().clone());
        self.base.buffers.push(self.color_uniform_buffer.as_ref().unwrap().clone());
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Update uniform buffer
        let mut uniform_data = Vec::new();
        uniform_data.extend_from_slice(&self.transform.to_array());
        uniform_data.extend_from_slice(&self.area);

        if let Some(uniform_buffer) = &self.uniform_buffer {
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&uniform_data));
        }

        // Update color uniform buffer
        let color_data = self.color.to_array();
        if let Some(color_buffer) = &self.color_uniform_buffer {
            queue.write_buffer(color_buffer, 0, bytemuck::cast_slice(&color_data));
        }

        // Create bind groups if needed
        self.create_bind_groups(device);
    }

    fn draw(&mut self, render_pass: &mut wgpu::RenderPass) {
        if let Some(pipeline) = &self.base.render_pipeline {
            render_pass.set_pipeline(pipeline);
            
            if let Some(uniform_bind_group) = &self.uniform_bind_group {
                render_pass.set_bind_group(0, uniform_bind_group, &[]);
            }
            
            if let Some(texture_bind_group) = &self.texture_bind_group {
                render_pass.set_bind_group(1, texture_bind_group, &[]);
            }
            
            if let Some(color_bind_group) = &self.color_bind_group {
                render_pass.set_bind_group(2, color_bind_group, &[]);
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

impl WgpuRenderGeneral2d {
    pub fn set_area(&mut self, area: &[f32; 4]) {
        self.area = *area;
    }

    pub fn set_transform(&mut self, transform: &WgpuTransform) {
        self.transform = *transform;
    }

    pub fn set_color(&mut self, color: &WgpuColor) {
        self.color = *color;
    }

    pub fn set_texture(&mut self, texture: &WgpuRenderTexture) {
        // Store reference to texture for bind group creation
        // In a real implementation, you might want to store this differently
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
                    label: Some("general2d_uniform_bind_group"),
                }));
            }
        }

        // Create color bind group
        if self.color_bind_group.is_none() {
            if let (Some(layout), Some(buffer)) = (&self.color_bind_group_layout, &self.color_uniform_buffer) {
                self.color_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                    label: Some("general2d_color_bind_group"),
                }));
            }
        }

        // TODO: Create texture bind group when texture is set
    }
} 