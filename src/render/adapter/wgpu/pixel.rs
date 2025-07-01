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
        let vertices = self.generate_vertices_from_buffer(buffer);
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
    }

    /// Prepare drawing with processed render cells (preferred method)
    /// 
    /// This method receives RenderCell data that has already been processed 
    /// through the complete game rendering pipeline, including sprites, 
    /// borders, logo, and other game elements.
    pub fn prepare_draw_with_render_cells(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, render_cells: &[crate::render::adapter::RenderCell]) {
        let vertices = self.generate_vertices_from_render_cells(render_cells);
        self.vertex_count = vertices.len() as u32;
        
        // Debug render cells statistics (only first frame)
        static mut FIRST_RENDER_CELLS_STATS: bool = true;
        unsafe {
            if FIRST_RENDER_CELLS_STATS {
                println!("WGPU Debug: Generated {} vertices from {} render cells", 
                         vertices.len(), render_cells.len());
                if !render_cells.is_empty() {
                    let rc = &render_cells[0];
                    println!("WGPU Debug: First render cell: pos=({}, {}), size={}x{}, texsym={}, fcolor=({:.2},{:.2},{:.2},{:.2})", 
                             rc.x, rc.y, rc.w, rc.h, rc.texsym, rc.fcolor.0, rc.fcolor.1, rc.fcolor.2, rc.fcolor.3);
                }
                FIRST_RENDER_CELLS_STATS = false;
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
    }

    /// Generate vertices from processed render cells
    fn generate_vertices_from_render_cells(&self, render_cells: &[crate::render::adapter::RenderCell]) -> Vec<WgpuVertex> {
        let mut vertices = Vec::new();
        
        // Convert render cells to vertices
        for render_cell in render_cells {
            // Convert screen coordinates to normalized device coordinates
            // Assume window size is 800x600 for now (we should get this from context)
            let window_width = 800.0;
            let window_height = 600.0;
            
            let left_ndc = (render_cell.x / window_width) * 2.0 - 1.0;
            let right_ndc = ((render_cell.x + render_cell.w as f32) / window_width) * 2.0 - 1.0;
            let top_ndc = 1.0 - (render_cell.y / window_height) * 2.0;
            let bottom_ndc = 1.0 - ((render_cell.y + render_cell.h as f32) / window_height) * 2.0;
            
            // Use foreground color from render cell
            let color = [render_cell.fcolor.0, render_cell.fcolor.1, render_cell.fcolor.2, render_cell.fcolor.3];
            
            // Create quad vertices (using triangle list, so need 6 vertices per quad)
            vertices.push(WgpuVertex {
                position: [left_ndc, bottom_ndc],
                tex_coords: [0.0, 1.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right_ndc, bottom_ndc],
                tex_coords: [1.0, 1.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right_ndc, top_ndc],
                tex_coords: [1.0, 0.0],
                color,
            });
            
            // Second triangle
            vertices.push(WgpuVertex {
                position: [left_ndc, bottom_ndc],
                tex_coords: [0.0, 1.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right_ndc, top_ndc],
                tex_coords: [1.0, 0.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [left_ndc, top_ndc],
                tex_coords: [0.0, 0.0],
                color,
            });
        }
        
        vertices
    }

    /// Generate vertices from buffer content
    fn generate_vertices_from_buffer(&self, buffer: &crate::render::buffer::Buffer) -> Vec<WgpuVertex> {
        // Get buffer dimensions
        let buffer_width = buffer.area.width as f32;
        let buffer_height = buffer.area.height as f32;
        
        let mut vertices = Vec::new();
        let mut debug_skipped = 0;
        let mut debug_rendered = 0;
        
        // Generate quads for each cell in the buffer
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                
                // Get cell info: (symidx, texidx, fg_color, bg_color)
                let cell_info = cell.get_cell_info();
                let symidx = cell_info.0;
                let fg_color = cell_info.2;
                let bg_color = cell_info.3;
                
                // Debug first few cells AND check for any non-empty cells
                static mut FIRST_BUFFER_DEBUG: bool = true;
                static mut NON_EMPTY_FOUND: bool = false;
                unsafe {
                    if FIRST_BUFFER_DEBUG && debug_rendered + debug_skipped < 5 {
                        println!("WGPU Debug Cell({},{}): symbol='{}', fg={:?}, bg={:?}", 
                                 x, y, cell.symbol, fg_color, bg_color);
                        if debug_rendered + debug_skipped >= 4 {
                            FIRST_BUFFER_DEBUG = false;
                        }
                    }
                    
                    // Check for any non-empty content
                    if !NON_EMPTY_FOUND && (cell.symbol != " " || bg_color != crate::render::style::Color::Reset) {
                        println!("WGPU Debug: Found non-empty cell at ({},{}) symbol='{}', fg={:?}, bg={:?}", 
                                 x, y, cell.symbol, fg_color, bg_color);
                        NON_EMPTY_FOUND = true;
                    }
                }
                
                // Convert buffer coordinates to normalized device coordinates
                // NDC: (-1,-1) = bottom-left, (1,1) = top-right
                // Buffer: (0,0) = top-left, (width,height) = bottom-right
                let left = (x as f32 / buffer_width) * 2.0 - 1.0;
                let right = ((x + 1) as f32 / buffer_width) * 2.0 - 1.0;
                // Flip Y axis: buffer y=0 should map to NDC y=1 (top)
                let top = 1.0 - (y as f32 / buffer_height) * 2.0;
                let bottom = 1.0 - ((y + 1) as f32 / buffer_height) * 2.0;
                
                // Only render cells with actual content - skip empty cells
                let render_color = if cell.symbol != " " && cell.symbol != "" {
                    // Non-empty symbol: use foreground color
                    debug_rendered += 1;
                    fg_color
                } else if bg_color != crate::render::style::Color::Reset {
                    // Empty symbol but has background: use background color
                    debug_rendered += 1;
                    bg_color
                } else {
                    // Skip empty cells with Reset background - no fallback gray
                    debug_skipped += 1;
                    continue;
                };
                
                // Convert color to RGBA float array
                let (r, g, b, a) = render_color.get_rgba();
                if a == 0 {
                    debug_skipped += 1;
                    continue; // Skip fully transparent cells
                }
                let color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                
                // Create quad vertices (using triangle list, so need 6 vertices per quad)
                vertices.push(WgpuVertex {
                    position: [left, bottom],
                    tex_coords: [0.0, 1.0],
                    color,
                });
                vertices.push(WgpuVertex {
                    position: [right, bottom],
                    tex_coords: [1.0, 1.0],
                    color,
                });
                vertices.push(WgpuVertex {
                    position: [right, top],
                    tex_coords: [1.0, 0.0],
                    color,
                });
                
                // Second triangle
                vertices.push(WgpuVertex {
                    position: [left, bottom],
                    tex_coords: [0.0, 1.0],
                    color,
                });
                vertices.push(WgpuVertex {
                    position: [right, top],
                    tex_coords: [1.0, 0.0],
                    color,
                });
                vertices.push(WgpuVertex {
                    position: [left, top],
                    tex_coords: [0.0, 0.0],
                    color,
                });
            }
        }
        
        // Debug buffer statistics (only first frame)
        static mut FIRST_STATS: bool = true;
        unsafe {
            if FIRST_STATS {
                println!("WGPU Debug: Buffer {}x{}, rendered {} cells, skipped {} cells", 
                         buffer_width, buffer_height, debug_rendered, debug_skipped);
                FIRST_STATS = false;
            }
        }
        
        // If no vertices generated, show a simple test pattern to confirm WGPU works
        if vertices.is_empty() {
            println!("WGPU Debug: No game content found, showing test pattern");
            
            // Create a simple colored square in the center of screen
            let size = 0.2; // 20% of screen
            let left = -size;
            let right = size;
            let top = size;
            let bottom = -size;
            let color = [1.0, 0.0, 0.0, 1.0]; // Red
            
            // Create quad vertices
            vertices.push(WgpuVertex {
                position: [left, bottom],
                tex_coords: [0.0, 1.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right, bottom],
                tex_coords: [1.0, 1.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right, top],
                tex_coords: [1.0, 0.0],
                color,
            });
            
            // Second triangle
            vertices.push(WgpuVertex {
                position: [left, bottom],
                tex_coords: [0.0, 1.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right, top],
                tex_coords: [1.0, 0.0],
                color,
            });
            vertices.push(WgpuVertex {
                position: [left, top],
                tex_coords: [0.0, 0.0],
                color,
            });
        }
        
        vertices
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
                    format: self.surface_format, // Use actual surface format
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

                // No bind group needed for simplified pipeline

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

