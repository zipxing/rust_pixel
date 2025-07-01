// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Pixel Renderer Module
//! 
//! Main pixel rendering implementation for WGPU pipeline.
//! Handles texture-based character and symbol rendering with
//! instanced drawing for high performance.

use super::*;
use super::shader_source::*;

/// Vertex data structure for WGPU rendering
/// 
/// Represents a single vertex with position, texture coordinates, and color.
/// Uses `#[repr(C)]` to ensure consistent memory layout for GPU upload.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuVertex {
    /// 2D position in normalized device coordinates
    pub position: [f32; 2],
    /// Texture coordinates (0.0-1.0 range)
    pub tex_coords: [f32; 2],
    /// Vertex color as RGBA components (0.0-1.0 range)
    pub color: [f32; 4],
}

impl WgpuVertex {
    /// Vertex buffer layout descriptor for WGPU
    /// 
    /// Defines how vertex data is structured in GPU memory,
    /// including attribute locations, formats, and offsets.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute (@location(0))
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Texture coordinates attribute (@location(1))
                wgpu::VertexAttribute {
                    offset: std::mem::offset_of!(WgpuVertex, tex_coords) as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Color attribute (@location(2))
                wgpu::VertexAttribute {
                    offset: std::mem::offset_of!(WgpuVertex, color) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// Safe for GPU upload
unsafe impl bytemuck::Pod for WgpuVertex {}
unsafe impl bytemuck::Zeroable for WgpuVertex {}

/// Uniform data structure for shader transformations
/// 
/// Contains transformation matrix for converting screen coordinates
/// to normalized device coordinates in shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuUniforms {
    /// 4x4 transformation matrix (column-major order)
    pub transform: [[f32; 4]; 4],
}

unsafe impl bytemuck::Pod for WgpuUniforms {}
unsafe impl bytemuck::Zeroable for WgpuUniforms {}

/// Main pixel renderer for WGPU
/// 
/// Manages the complete pixel rendering pipeline including shaders,
/// textures, buffers, and render state for character-based graphics.
pub struct WgpuPixelRender {
    /// Base renderer data (shared resources)
    base: WgpuRenderBase,
    
    /// Vertex buffer for quad geometry
    vertex_buffer: Option<wgpu::Buffer>,
    
    /// Index buffer for quad indices
    index_buffer: Option<wgpu::Buffer>,
    
    /// Uniform buffer for transformation matrices
    uniform_buffer: Option<wgpu::Buffer>,
    
    /// Main texture for symbols and characters
    symbol_texture: Option<texture::WgpuTexture>,
    
    /// Bind group layout for shader resources
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    
    /// Bind group for current frame resources
    bind_group: Option<wgpu::BindGroup>,
}

impl WgpuPixelRender {
    /// Vertex data for a fullscreen quad
    /// 
    /// Two triangles forming a rectangle covering the entire screen.
    /// Used as base geometry for all symbol rendering.
    const VERTICES: &'static [WgpuVertex] = &[
        // Triangle 1
        WgpuVertex {
            position: [-1.0, -1.0],  // Bottom left
            tex_coords: [0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],  // Red
        },
        WgpuVertex {
            position: [1.0, -1.0],   // Bottom right
            tex_coords: [1.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],  // Green
        },
        WgpuVertex {
            position: [1.0, 1.0],    // Top right
            tex_coords: [1.0, 0.0],
            color: [0.0, 0.0, 1.0, 1.0],  // Blue
        },
        WgpuVertex {
            position: [-1.0, 1.0],   // Top left
            tex_coords: [0.0, 0.0],
            color: [1.0, 1.0, 0.0, 1.0],  // Yellow
        },
    ];

    /// Index data for quad triangulation
    /// 
    /// Two triangles: (0,1,2) and (2,3,0) forming a rectangle.
    const INDICES: &'static [u16] = &[
        0, 1, 2,  // First triangle
        2, 3, 0,  // Second triangle
    ];
}

impl WgpuRender for WgpuPixelRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            base: WgpuRenderBase::new(0, canvas_width, canvas_height),
            vertex_buffer: None,  
            index_buffer: None,
            uniform_buffer: None,
            symbol_texture: None,
            bind_group_layout: None,
            bind_group: None,
        }
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device) {
        // Create shader modules from WGSL source
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pixel Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(PIXEL_VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pixel Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(PIXEL_FRAGMENT_SHADER.into()),
        });

        // Create render pipeline with no bind groups for simplicity
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pixel Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pixel Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[WgpuVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb, // Common surface format
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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

        // Store resources in base
        self.base.render_pipelines.push(render_pipeline);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Create vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pixel Vertex Buffer"),
            size: (Self::VERTICES.len() * std::mem::size_of::<WgpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create index buffer
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pixel Index Buffer"),
            size: (Self::INDICES.len() * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pixel Uniform Buffer"),
            size: std::mem::size_of::<WgpuUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Store buffers
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.uniform_buffer = Some(uniform_buffer);
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Upload vertex data
        if let Some(vertex_buffer) = &self.vertex_buffer {
            queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(Self::VERTICES));
        }

        // Upload index data
        if let Some(index_buffer) = &self.index_buffer {
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(Self::INDICES));
        }

        // No uniform buffer needed for simplified pipeline

        // Skip bind group creation for simplified pipeline
    }

    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pixel Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Set pipeline and buffers
        if let Some(pipeline) = self.base.render_pipelines.get(0) {
            render_pass.set_pipeline(pipeline);

            if let (Some(vertex_buffer), Some(index_buffer)) = 
                (&self.vertex_buffer, &self.index_buffer) {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                // No bind group needed for simplified pipeline

                // Draw indexed quad
                render_pass.draw_indexed(0..Self::INDICES.len() as u32, 0, 0..1);
            }
        }
    }

    fn cleanup(&mut self, _device: &wgpu::Device) {
        // Cleanup per-frame state if needed
        // For now, no cleanup required
    }
}

