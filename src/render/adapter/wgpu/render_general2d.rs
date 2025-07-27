// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # WGPU General2D Renderer Module
//! 
//! Handles rendering textures to screen or other render targets with
//! transformation and color modulation support. Similar to OpenGL GlRenderGeneral2d.

use super::shader_source;
use crate::render::graph::{UnifiedColor, UnifiedTransform};
use super::{WgpuRender, WgpuRenderBase};
use std::mem;
use wgpu::util::DeviceExt;

/// Vertex data for General2D rendering (fullscreen quad)
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct General2dVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for General2dVertex {}
unsafe impl bytemuck::Zeroable for General2dVertex {}

/// Uniform data for General2D shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct General2dUniforms {
    pub transform: [[f32; 4]; 4],  // 4x4 transformation matrix
    pub area: [f32; 4],            // [x, y, width, height]
    pub color: [f32; 4],           // [r, g, b, a]
}

/// WGPU General2D renderer for texture rendering
/// 
/// This renderer can draw textures (especially render textures) to screen
/// or other render targets with transformations and color modulation.
pub struct WgpuGeneral2dRender {
    /// Base renderer data
    base: WgpuRenderBase,

    /// Current rendering area
    area: [f32; 4],

    /// Current transformation matrix
    transform: UnifiedTransform,

    /// Current color modulation
    color: UnifiedColor,

    /// Vertex buffer for quad geometry
    vertex_buffer: Option<wgpu::Buffer>,

    /// Index buffer for quad indices
    index_buffer: Option<wgpu::Buffer>,

    /// Uniform buffer for shader parameters
    uniform_buffer: Option<wgpu::Buffer>,

    /// Render pipeline for drawing
    render_pipeline: Option<wgpu::RenderPipeline>,

    /// Bind group layout for resources
    bind_group_layout: Option<wgpu::BindGroupLayout>,

    /// Current bind group (updated per draw call)
    current_bind_group: Option<wgpu::BindGroup>,

    /// Current render texture source (set via set_render_texture)
    current_render_texture_index: Option<usize>,
}

impl WgpuRender for WgpuGeneral2dRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            base: WgpuRenderBase::new(0, canvas_width, canvas_height),
            area: [0.0, 0.0, 1.0, 1.0],  // Default to full texture
            transform: UnifiedTransform::new(),
            color: UnifiedColor::white(),
            vertex_buffer: None,
            index_buffer: None,
            uniform_buffer: None,
            render_pipeline: None,
            bind_group_layout: None,
            current_bind_group: None,
            current_render_texture_index: None,
        }
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device) {
        // Default to Rgba8Unorm format if not specified
        self.create_shader_with_format(device, wgpu::TextureFormat::Rgba8Unorm);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Define fullscreen quad vertices
        let vertices = [
            General2dVertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] }, // Bottom-left
            General2dVertex { position: [ 1.0, -1.0], tex_coords: [1.0, 1.0] }, // Bottom-right
            General2dVertex { position: [ 1.0,  1.0], tex_coords: [1.0, 0.0] }, // Top-right
            General2dVertex { position: [-1.0,  1.0], tex_coords: [0.0, 0.0] }, // Top-left
        ];

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("General2D Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("General2D Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Uniform buffer will be created per draw call in prepare_draw() to avoid state conflicts

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.uniform_buffer = None; // Will be created per draw call
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Create uniform data with current transform, area, and color
        let uniforms = General2dUniforms {
            transform: self.transform.to_matrix4(),
            area: self.area,
            color: self.color.to_array(),
        };

        // Create a NEW uniform buffer for each draw call to avoid state conflicts
        // This ensures each render_texture_to_screen call has independent uniform data
        self.uniform_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("General2D Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM,
        }));
    }

    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if let (Some(pipeline), Some(vertex_buffer), Some(index_buffer), Some(bind_group)) = (
            &self.render_pipeline,
            &self.vertex_buffer,
            &self.index_buffer,
            &self.current_bind_group,
        ) {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("General2D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear, blend with existing content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
    }

    fn cleanup(&mut self, _device: &wgpu::Device) {
        // WGPU handles resource cleanup automatically
    }
}

impl General2dVertex {
    /// Vertex buffer layout descriptor for WGPU
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<General2dVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Texture coordinates attribute
                wgpu::VertexAttribute {
                    offset: mem::offset_of!(General2dVertex, tex_coords) as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl WgpuGeneral2dRender {
    /// Create shader with a specific texture format
    pub fn create_shader_with_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        // Create shader module using the complete shader source
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("General2D Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source::GENERAL2D_SHADER)),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("General2D Bind Group Layout"),
            entries: &[
                // Uniform buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("General2D Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("General2D Render Pipeline"),
            layout: Some(&pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[General2dVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: format, // Use the specified format
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for 2D quads
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        self.bind_group_layout = Some(bind_group_layout);
        self.render_pipeline = Some(render_pipeline);
    }

    /// Prepare draw with specific render texture references
    /// 
    /// This method creates the bind group with actual texture references.
    /// Called from WgpuPixelRender.render_texture_to_screen_impl() to avoid reference issues.
    pub fn prepare_draw_with_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        // First update uniforms
        self.prepare_draw(device, queue);

        // Create bind group with provided texture references
        if let (Some(bind_group_layout), Some(uniform_buffer)) = (
            &self.bind_group_layout,
            &self.uniform_buffer,
        ) {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
                label: Some("General2D Bind Group"),
            });

            self.current_bind_group = Some(bind_group);
        }
    }

    /// Set the render texture index
    /// 
    /// # Parameters
    /// - `rtidx`: Render texture index to use as source
    pub fn set_render_texture_index(&mut self, rtidx: usize) -> &mut Self {
        self.current_render_texture_index = Some(rtidx);
        self
    }

    /// Set the rendering area (texture coordinates mapping)
    /// 
    /// # Parameters
    /// - `area`: [x, y, width, height] in texture space (0.0-1.0)
    pub fn set_area(&mut self, area: &[f32; 4]) -> &mut Self {
        self.area = *area;
        self
    }

    /// Set the transformation matrix
    /// 
    /// # Parameters
    /// - `transform`: Transform to apply to the quad
    pub fn set_transform(&mut self, transform: &UnifiedTransform) -> &mut Self {
        self.transform = *transform;
        self
    }

    /// Set the color modulation
    /// 
    /// # Parameters
    /// - `color`: Color to multiply with texture
    pub fn set_color(&mut self, color: &UnifiedColor) -> &mut Self {
        self.color = *color;
        self
    }
} 