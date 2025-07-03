// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Pixel Renderer Module
//! 
//! Main pixel rendering implementation for WGPU pipeline.
//! Handles texture-based character and symbol rendering with
//! instanced drawing for high performance.

use super::*;
use super::render_symbols::WgpuSymbolRenderer;
use super::shader_source;


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
    
    /// Symbol renderer for vertex generation
    symbol_renderer: WgpuSymbolRenderer,
    
    /// Surface format for render target compatibility
    surface_format: wgpu::TextureFormat,
    
    /// Current vertex count for drawing
    vertex_count: u32,
    
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
    /// Create new WgpuPixelRender with specific surface format
    pub fn new_with_format(canvas_width: u32, canvas_height: u32, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            base: WgpuRenderBase::new(0, canvas_width, canvas_height),
            symbol_renderer: WgpuSymbolRenderer::new(canvas_width, canvas_height),
            surface_format,
            vertex_count: 0,
            vertex_buffer: None,  
            index_buffer: None,
            uniform_buffer: None,
            symbol_texture: None,
            bind_group_layout: None,
            bind_group: None,
        }
    }
    
    /// Load the symbol texture from the specified path
    pub fn load_symbol_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texture_path: &str) -> Result<(), String> {
        // Load the texture file
        let texture_bytes = std::fs::read(texture_path)
            .map_err(|e| format!("Failed to read texture file {}: {}", texture_path, e))?;
        
        let texture_image = image::load_from_memory(&texture_bytes)
            .map_err(|e| format!("Failed to load texture image: {}", e))?
            .to_rgba8();
        
        let texture_width = texture_image.width();
        let texture_height = texture_image.height();
        
        println!("WGPU Debug: Loaded symbol texture {}x{} from {}", texture_width, texture_height, texture_path);
        
        // Create WGPU texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Symbol Texture"),
            size: wgpu::Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Write texture data
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_width),
                rows_per_image: Some(texture_height),
            },
            wgpu::Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
        );
        
        // Create texture view
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel art should use nearest neighbor
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            border_color: None,
            anisotropy_clamp: 1,
            label: Some("Symbol Sampler"),
        });
        
        // Store texture in WgpuTexture wrapper
        self.symbol_texture = Some(texture::WgpuTexture {
            texture,
            view: texture_view,
            sampler: Some(sampler),
            width: texture_width,
            height: texture_height,
        });
        
        Ok(())
    }
    
    /// Create bind group for texture and uniform buffer
    pub fn create_bind_group(&mut self, device: &wgpu::Device) {
        if let (Some(bind_group_layout), Some(symbol_texture), Some(uniform_buffer)) = 
            (&self.bind_group_layout, &self.symbol_texture, &self.uniform_buffer) {
            
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&symbol_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(symbol_texture.sampler.as_ref().unwrap()),
                    },
                ],
                label: Some("Symbol Bind Group"),
            });
            
            self.bind_group = Some(bind_group);
        }
    }

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

        /// Prepare drawing with actual game buffer content
    pub fn prepare_draw_with_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, buffer: &crate::render::buffer::Buffer) {
        // For now, generate vertices based on buffer content
        let vertices = self.symbol_renderer.generate_vertices_from_buffer(buffer);
        self.vertex_count = vertices.len() as u32;
        
        // Debug: Print rendering information (only first frame)
        static mut FIRST_FRAME: bool = true;
        unsafe {
            if FIRST_FRAME && vertices.len() > 0 {
                println!("WGPU Debug: Generated {} vertices, first 3 colors:", vertices.len());
                for i in 0..(3.min(vertices.len())) {
                    let v = &vertices[i];
                    println!("  Vertex {}: pos=[{:.2}, {:.2}], color=[{:.2}, {:.2}, {:.2}, {:.2}]",
                             i, v.position[0], v.position[1], v.color[0], v.color[1], v.color[2], v.color[3]);
                }
                FIRST_FRAME = false;
            }
        }
        
        // Upload generated vertex data
        if let Some(vertex_buffer) = &self.vertex_buffer {
            queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        // Upload index data
        if let Some(index_buffer) = &self.index_buffer {
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(Self::INDICES));
        }
        
        // Upload uniform data (identity matrix for now)
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let uniforms = WgpuUniforms {
                transform: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            };
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Prepare drawing with processed render cells (preferred method)
    /// 
    /// This method receives RenderCell data that has already been processed 
    /// through the complete game rendering pipeline, including sprites, 
    /// borders, logo, and other game elements.
    pub fn prepare_draw_with_render_cells(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, render_cells: &[crate::render::adapter::RenderCell]) {
        let vertices = self.symbol_renderer.generate_vertices_from_render_cells(render_cells);
        self.vertex_count = vertices.len() as u32;
        
        // Debug output removed for performance
        
        // Upload generated vertex data
        if let Some(vertex_buffer) = &self.vertex_buffer {
            queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        // Upload index data
        if let Some(index_buffer) = &self.index_buffer {
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(Self::INDICES));
        }
        
        // Upload uniform data (identity matrix for now)
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let uniforms = WgpuUniforms {
                transform: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            };
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }




}

impl WgpuRender for WgpuPixelRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        // Use a default format, will be set properly in new_with_format
        Self::new_with_format(canvas_width, canvas_height, wgpu::TextureFormat::Bgra8UnormSrgb)
    }

    fn get_base(&mut self) -> &mut WgpuRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, device: &wgpu::Device) {
        // Create shader modules from shader_source module
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Symbol Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source::PIXEL_UNIFORM_VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Symbol Fragment Shader"), 
            source: wgpu::ShaderSource::Wgsl(shader_source::PIXEL_TEXTURE_FRAGMENT_SHADER.into()),
        });

        // Create bind group layout for texture and sampler
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Symbol Bind Group Layout"),
            entries: &[
                // Uniform buffer (transformation matrix)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        // Create render pipeline layout with bind group
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Symbol Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Symbol Render Pipeline"),
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
                    format: self.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Don't cull for 2D sprites
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

        // Store resources
        self.base.render_pipelines.push(render_pipeline);
        self.bind_group_layout = Some(bind_group_layout);
    }

    fn create_buffer(&mut self, device: &wgpu::Device) {
        // Create vertex buffer with enough space for many vertices
        // Estimate max vertices based on canvas size: each cell could be 6 vertices (2 triangles)
        let max_vertices = self.base.canvas_width * self.base.canvas_height * 6;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pixel Vertex Buffer"),
            size: (max_vertices as usize * std::mem::size_of::<WgpuVertex>()) as u64,
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
        // Upload test vertex data
        self.vertex_count = Self::VERTICES.len() as u32;
        if let Some(vertex_buffer) = &self.vertex_buffer {
            queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(Self::VERTICES));
        }

        // Upload index data
        if let Some(index_buffer) = &self.index_buffer {
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(Self::INDICES));
        }
        
        // Upload uniform data (identity matrix for now)
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let uniforms = WgpuUniforms {
                transform: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            };
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }



    fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // Begin render pass with black background for final version
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pixel Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,  // Black background for game
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

            if let Some(vertex_buffer) = &self.vertex_buffer {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

                // Set bind group with texture and uniform buffer
                if let Some(bind_group) = &self.bind_group {
                    render_pass.set_bind_group(0, bind_group, &[]);
                }

                // Draw vertices directly (triangle list mode)
                // Use the actual vertex count from the last prepare_draw_with_buffer call
                render_pass.draw(0..self.vertex_count, 0..1);
            }
        }
    }

    fn cleanup(&mut self, _device: &wgpu::Device) {
        // Cleanup per-frame state if needed
        // For now, no cleanup required
    }
}

