// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Transition Renderer Module
//! 
//! Handles transition effects between two textures with various algorithms.
//! Similar to OpenGL GlRenderTransition but using WGPU/WGSL.

use super::{shader_source, WgpuRender, WgpuRenderBase};
use std::mem;
use wgpu::util::DeviceExt;

/// Vertex data for transition rendering (fullscreen quad)
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TransitionVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for TransitionVertex {}
unsafe impl bytemuck::Zeroable for TransitionVertex {}

/// Uniform data for transition shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TransitionUniforms {
    pub progress: f32,
    pub _padding1: [f32; 3], // Complete the first 16-byte block: 4 + 12 = 16 bytes
    pub _padding2: [f32; 4], // vec3<f32> padded to 16 bytes in WGSL
    pub _padding3: [f32; 4], // vec4<f32> = 16 bytes
}

/// WGPU Transition renderer for transition effects
/// 
/// This renderer can blend between two textures using various transition algorithms.
/// Supports 7 different transition effects matching the OpenGL version.
pub struct WgpuTransitionRender {
    /// Base renderer data
    base: WgpuRenderBase,

    /// Current transition progress (0.0 to 1.0)
    progress: f32,

    /// Current shader index (0-6 for different effects)
    shader_idx: usize,

    /// Vertex buffer for quad geometry
    vertex_buffer: Option<wgpu::Buffer>,

    /// Index buffer for quad indices
    index_buffer: Option<wgpu::Buffer>,

    /// Uniform buffer for shader parameters
    uniform_buffer: Option<wgpu::Buffer>,

    /// Render pipelines for different transition effects (7 effects)
    render_pipelines: Vec<Option<wgpu::RenderPipeline>>,

    /// Bind group layout for resources
    bind_group_layout: Option<wgpu::BindGroupLayout>,

    /// Current bind group (updated per draw call)
    current_bind_group: Option<wgpu::BindGroup>,

    /// Texture sampler
    texture_sampler: Option<wgpu::Sampler>,
}

impl WgpuRender for WgpuTransitionRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let mut render_pipelines = Vec::new();
        for _ in 0..7 {
            render_pipelines.push(None);
        }

        Self {
            base: WgpuRenderBase::new(0, canvas_width, canvas_height),
            progress: 0.0,
            shader_idx: 0,
            vertex_buffer: None,
            index_buffer: None,
            uniform_buffer: None,
            render_pipelines,
            bind_group_layout: None,
            current_bind_group: None,
            texture_sampler: None,
        }
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device) {
        // Create bind group layout for transition shaders
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transition Bind Group Layout"),
            entries: &[
                // Uniform buffer (progress)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Texture 1 (from texture)
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
                // Texture 2 (to texture)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transition Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create shader modules for all 7 transition effects
        let transition_shaders = shader_source::get_transition_shaders();
        for (i, shader_source) in transition_shaders.iter().enumerate() {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("Transition Shader {}", i)),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_source)),
            });

            // Create render pipeline for this transition effect
            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("Transition Render Pipeline {}", i)),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[TransitionVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm, // Use Bgra8Unorm to match surface format
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
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

            self.render_pipelines[i] = Some(render_pipeline);
        }

        // Create texture sampler
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Transition Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        self.bind_group_layout = Some(bind_group_layout);
        self.texture_sampler = Some(texture_sampler);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Define fullscreen quad vertices
        let vertices = [
            TransitionVertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] }, // Bottom-left
            TransitionVertex { position: [ 1.0, -1.0], tex_coords: [1.0, 1.0] }, // Bottom-right
            TransitionVertex { position: [ 1.0,  1.0], tex_coords: [1.0, 0.0] }, // Top-right
            TransitionVertex { position: [-1.0,  1.0], tex_coords: [0.0, 0.0] }, // Top-left
        ];

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        // Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transition Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transition Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transition Uniform Buffer"),
            size: mem::size_of::<TransitionUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.uniform_buffer = Some(uniform_buffer);
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Update uniform buffer with current progress
        let uniforms = TransitionUniforms {
            progress: self.progress,
            _padding1: [0.0; 3],
            _padding2: [0.0; 4],
            _padding3: [0.0; 4],
        };

        if let Some(uniform_buffer) = &self.uniform_buffer {
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if self.shader_idx >= self.render_pipelines.len() {
            return;
        }

        let pipeline = match &self.render_pipelines[self.shader_idx] {
            Some(pipeline) => pipeline,
            None => return,
        };

        let bind_group = match &self.current_bind_group {
            Some(bind_group) => bind_group,
            None => return,
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Transition Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        
        if let (Some(vertex_buffer), Some(index_buffer)) = (&self.vertex_buffer, &self.index_buffer) {
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
    }

    fn cleanup(&mut self, _device: &wgpu::Device) {
        // Clean up resources
        self.vertex_buffer = None;
        self.index_buffer = None;
        self.uniform_buffer = None;
        self.render_pipelines.clear();
        self.bind_group_layout = None;
        self.current_bind_group = None;
        self.texture_sampler = None;
    }
}

impl TransitionVertex {
    /// Get the vertex buffer layout descriptor
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TransitionVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Texture coordinates
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl WgpuTransitionRender {
    /// Set the two textures for transition
    pub fn set_textures(
        &mut self,
        device: &wgpu::Device,
        texture1: &wgpu::TextureView,
        texture2: &wgpu::TextureView,
    ) {
        if let (Some(bind_group_layout), Some(uniform_buffer), Some(sampler)) = (
            &self.bind_group_layout,
            &self.uniform_buffer,
            &self.texture_sampler,
        ) {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Transition Bind Group"),
                layout: bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(texture1),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(texture2),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });

            self.current_bind_group = Some(bind_group);
        }
    }

    /// Set the transition shader index (0-6)
    pub fn set_shader_index(&mut self, shader_idx: usize) -> &mut Self {
        self.shader_idx = shader_idx.min(6); // Clamp to valid range
        self
    }

    /// Set the transition progress (0.0 to 1.0)
    pub fn set_progress(&mut self, progress: f32) -> &mut Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Draw transition effect - main API matching OpenGL version
    pub fn draw_transition(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        shader_idx: usize,
        progress: f32,
    ) {
        self.set_shader_index(shader_idx);
        self.set_progress(progress);
        self.prepare_draw(device, queue);
        self.draw(encoder, view);
    }
} 