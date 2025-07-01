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
        let vertices = self.generate_vertices_from_render_cells(render_cells);
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

    /// Generate vertices from processed render cells
    fn generate_vertices_from_render_cells(&self, render_cells: &[crate::render::adapter::RenderCell]) -> Vec<WgpuVertex> {
        let mut vertices = Vec::new();
        
        // Constants for texture atlas layout
        // symbols.png is 1024x1024 pixels with 128x128 symbol positions
        // Each symbol occupies 8x8 pixels (1024÷128=8)
        const ATLAS_WIDTH_SYMBOLS: f32 = 128.0; // 128 symbols wide
        const ATLAS_HEIGHT_SYMBOLS: f32 = 128.0; // 128 symbols tall
        const PIXELS_PER_SYMBOL: f32 = 8.0; // Each symbol is 8x8 pixels
        const TEXTURE_SIZE: f32 = 1024.0; // Total texture size in pixels
        

        
        // Convert render cells to vertices
        for render_cell in render_cells {
            // Convert screen coordinates to normalized device coordinates
            // Use actual canvas dimensions from base renderer
            let window_width = self.base.canvas_width as f32;
            let window_height = self.base.canvas_height as f32;
            
            let left = render_cell.x;
            let right = render_cell.x + render_cell.w as f32;
            let top = render_cell.y;
            let bottom = render_cell.y + render_cell.h as f32;
            

            
            // Apply rotation if angle is not zero
            // Simplified approach: rotate the quad corners directly around the rotation center
            let (left_ndc, right_ndc, top_ndc, bottom_ndc) = if render_cell.angle != 0.0 {
                let cx = render_cell.cx as f32;
                let cy = render_cell.cy as f32;
                
                // Calculate the actual rotation center in screen coordinates
                // The rotation center is relative to the render_cell position
                let center_x = render_cell.x + cx;
                let center_y = render_cell.y + cy;
                
                // Apply rotation to each corner around the center
                let cos_a = render_cell.angle.cos();
                let sin_a = render_cell.angle.sin();
                
                let corners = [
                    (left, bottom),   // bottom-left
                    (right, bottom),  // bottom-right
                    (right, top),     // top-right
                    (left, top),      // top-left
                ];
                
                let rotated_corners: Vec<(f32, f32)> = corners.iter().map(|(x, y)| {
                    // Translate to origin (relative to rotation center)
                    let dx = x - center_x;
                    let dy = y - center_y;
                    
                    // Apply rotation
                    let rotated_x = dx * cos_a - dy * sin_a;
                    let rotated_y = dx * sin_a + dy * cos_a;
                    
                    // Translate back
                    (rotated_x + center_x, rotated_y + center_y)
                }).collect();
                
                // Convert to NDC
                let ndc_corners: Vec<(f32, f32)> = rotated_corners.iter().map(|(x, y)| {
                    let x_ndc = (x / window_width) * 2.0 - 1.0;
                    let y_ndc = 1.0 - (y / window_height) * 2.0;
                    (x_ndc, y_ndc)
                }).collect();
                
                (ndc_corners[0], ndc_corners[1], ndc_corners[2], ndc_corners[3])
            } else {
                // No rotation - simple conversion to NDC
                let left_ndc = (left / window_width) * 2.0 - 1.0;
                let right_ndc = (right / window_width) * 2.0 - 1.0;
                let top_ndc = 1.0 - (top / window_height) * 2.0;
                let bottom_ndc = 1.0 - (bottom / window_height) * 2.0;
                
                ((left_ndc, bottom_ndc), (right_ndc, bottom_ndc), (right_ndc, top_ndc), (left_ndc, top_ndc))
            };
            
            // Use foreground color from render cell
            let color = [render_cell.fcolor.0, render_cell.fcolor.1, render_cell.fcolor.2, render_cell.fcolor.3];
            
            // Calculate texture coordinates from texsym field
            // texsym = y * 128 + x, where x,y are symbol positions (0-127)
            let texsym = render_cell.texsym;
            let symbol_x = (texsym % 128) as f32; // X position in symbol grid (0-127)
            let symbol_y = (texsym / 128) as f32; // Y position in symbol grid (0-127)
            
            // Convert symbol positions to pixel coordinates
            let pixel_x = symbol_x * PIXELS_PER_SYMBOL; // X in pixels (0-1016)
            let pixel_y = symbol_y * PIXELS_PER_SYMBOL; // Y in pixels (0-1016)
            
            // Convert pixel coordinates to texture coordinates (0.0-1.0)
            let tex_left = pixel_x / TEXTURE_SIZE;
            let tex_right = (pixel_x + PIXELS_PER_SYMBOL) / TEXTURE_SIZE;
            let tex_top = pixel_y / TEXTURE_SIZE;
            let tex_bottom = (pixel_y + PIXELS_PER_SYMBOL) / TEXTURE_SIZE;
            
            // Debug output removed for performance
            
            // Create quad vertices (using triangle list, so need 6 vertices per quad)
            // Use the calculated (potentially rotated) corner positions
            // Corners are: [bottom-left, bottom-right, top-right, top-left]
            vertices.push(WgpuVertex {
                position: [left_ndc.0, left_ndc.1],  // bottom-left
                tex_coords: [tex_left, tex_bottom],
                color,
            });
            vertices.push(WgpuVertex {
                position: [right_ndc.0, right_ndc.1],  // bottom-right
                tex_coords: [tex_right, tex_bottom],
                color,
            });
            vertices.push(WgpuVertex {
                position: [top_ndc.0, top_ndc.1],  // top-right
                tex_coords: [tex_right, tex_top],
                color,
            });
            
            // Second triangle
            vertices.push(WgpuVertex {
                position: [left_ndc.0, left_ndc.1],  // bottom-left
                tex_coords: [tex_left, tex_bottom],
                color,
            });
            vertices.push(WgpuVertex {
                position: [top_ndc.0, top_ndc.1],  // top-right
                tex_coords: [tex_right, tex_top],
                color,
            });
            vertices.push(WgpuVertex {
                position: [bottom_ndc.0, bottom_ndc.1],  // top-left
                tex_coords: [tex_left, tex_top],
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
                
                // Debug output removed for performance
                
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
        // Create shader modules from custom WGSL source (non-instanced version)
        let vertex_shader_source = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct Uniforms {
    transform: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Apply uniform transformation
    output.clip_position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    
    return output;
}
"#;

        let fragment_shader_source = r#"
@group(0) @binding(1)
var t_symbols: texture_2d<f32>;
@group(0) @binding(2)
var s_symbols: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_symbols, s_symbols, input.tex_coords);
    if (tex_color.a < 0.1) {
        discard;
    }
    return tex_color * input.color;
}
"#;

        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Symbol Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(vertex_shader_source.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Symbol Fragment Shader"), 
            source: wgpu::ShaderSource::Wgsl(fragment_shader_source.into()),
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

