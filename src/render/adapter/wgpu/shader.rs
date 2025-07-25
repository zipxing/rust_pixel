// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # WGPU Shader Management Module
//! 
//! Handles WGSL shader compilation, pipeline creation, and shader resource management.

/// WGPU Shader compilation and management utilities
pub struct WgpuShader {
    /// Compiled vertex shader module
    pub vertex_module: Option<wgpu::ShaderModule>,
    /// Compiled fragment shader module
    pub fragment_module: Option<wgpu::ShaderModule>,
    /// Render pipeline layout
    pub pipeline_layout: Option<wgpu::PipelineLayout>,
    /// Bind group layout for uniforms and textures
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl WgpuShader {
    /// Create new shader manager
    pub fn new() -> Self {
        Self {
            vertex_module: None,
            fragment_module: None,
            pipeline_layout: None,
            bind_group_layout: None,
        }
    }

    /// Compile WGSL shader from source
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `source`: WGSL shader source code
    /// - `label`: Optional label for debugging
    /// 
    /// # Returns
    /// Compiled shader module
    pub fn compile(
        device: &wgpu::Device,
        source: &str,
        label: Option<&str>,
    ) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    }

    /// Create render pipeline
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `vertex_module`: Compiled vertex shader
    /// - `fragment_module`: Compiled fragment shader
    /// - `vertex_layout`: Vertex buffer layout
    /// 
    /// # Returns
    /// Created render pipeline
    pub fn create_pipeline(
        device: &wgpu::Device,
        vertex_module: &wgpu::ShaderModule,
        fragment_module: &wgpu::ShaderModule,
        vertex_layout: &wgpu::VertexBufferLayout,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        surface_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RustPixel Pipeline Layout"),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RustPixel Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vertex_module,
                entry_point: "vs_main",
                buffers: &[vertex_layout.clone()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: fragment_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
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
        })
    }
} 