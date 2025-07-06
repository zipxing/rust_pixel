// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # WGPU Pixel Renderer Module
//!
//! Main pixel rendering implementation for WGPU pipeline.
//! Handles texture-based character and symbol rendering with
//! instanced drawing for high performance.

use super::render_symbols::WgpuSymbolRenderer;
use super::render_general2d::WgpuGeneral2dRender;
use super::render_transition::WgpuTransitionRender;
use super::shader_source;
use super::*;
use super::texture::WgpuRenderTexture;
use super::transform::WgpuTransform;
use super::color::WgpuColor;

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

/// WGPU uniform data structure
///
/// This structure defines the uniform buffer layout that matches the
/// WGSL shader uniform structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WgpuUniforms {
    /// 4x4 transformation matrix (column-major order)
    pub transform: [[f32; 4]; 4],
    /// Color filter like GL mode (r, g, b, a) - 暂时保留结构但不使用
    pub color_filter: [f32; 4],
}

/// Main pixel renderer for WGPU
///
/// Manages the complete pixel rendering pipeline including shaders,
/// textures, buffers, and render state for character-based graphics.
pub struct WgpuPixelRender {
    /// Base renderer data (shared resources)
    base: WgpuRenderBase,

    /// Symbol renderer for vertex generation
    symbol_renderer: WgpuSymbolRenderer,

    /// General2D renderer for texture-to-texture rendering
    general2d_renderer: WgpuGeneral2dRender,

    /// Transition renderer for transition effects between textures
    transition_renderer: WgpuTransitionRender,

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

    /// Render textures for transition effects (0-3, matching OpenGL mode)
    render_textures: Vec<WgpuRenderTexture>,

    /// Canvas dimensions for render texture compatibility
    pub canvas_width: u32,
    pub canvas_height: u32,
}

impl WgpuPixelRender {
    /// Create new WgpuPixelRender with specific surface format
    pub fn new_with_format(
        canvas_width: u32,
        canvas_height: u32,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            base: WgpuRenderBase::new(0, canvas_width, canvas_height),
            symbol_renderer: WgpuSymbolRenderer::new(canvas_width, canvas_height),
            general2d_renderer: WgpuGeneral2dRender::new(canvas_width, canvas_height),
            transition_renderer: WgpuTransitionRender::new(canvas_width, canvas_height),
            surface_format,
            vertex_count: 0,
            vertex_buffer: None,
            index_buffer: None,
            uniform_buffer: None,
            symbol_texture: None,
            bind_group_layout: None,
            bind_group: None,
            render_textures: Vec::new(),
            canvas_width,
            canvas_height,
        }
    }

    /// Initialize render textures (similar to OpenGL GlPixel)
    /// 
    /// Creates 4 render textures for transition effects:
    /// - 0: transition texture 1 (hidden by default)
    /// - 1: transition texture 2 (hidden by default)
    /// - 2: main buffer (visible by default)
    /// - 3: transition buffer (visible by default)
    pub fn init_render_textures(&mut self, device: &wgpu::Device) -> Result<(), String> {
        // Clear existing render textures
        self.render_textures.clear();
        
        // Create 4 render textures with appropriate hidden states
        let rt_hidden = [true, true, false, false];
        
        for i in 0..4 {
            let render_texture = WgpuRenderTexture::new_with_format(
                device,
                self.canvas_width,
                self.canvas_height,
                self.surface_format,  // Use surface format to match pipelines
                rt_hidden[i],
            )?;
            
            self.render_textures.push(render_texture);
            
            log::info!("WGPU render texture {} created ({}x{}, format: {:?}, hidden: {})", 
                i, self.canvas_width, self.canvas_height, self.surface_format, rt_hidden[i]);
        }
        
        Ok(())
    }

    /// Initialize General2D renderer for texture-to-texture rendering
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    pub fn init_general2d_renderer(&mut self, device: &wgpu::Device) {
        self.general2d_renderer.init(device);
    }

    /// Initialize Transition renderer for transition effects
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    pub fn init_transition_renderer(&mut self, device: &wgpu::Device) {
        self.transition_renderer.init(device);
    }

    /// Get render texture hidden state (matches OpenGL GlPixel interface)
    /// 
    /// # Parameters
    /// - `rtidx`: Render texture index (0-3)
    /// 
    /// # Returns
    /// True if hidden, false if visible
    pub fn get_render_texture_hidden(&self, rtidx: usize) -> bool {
        if rtidx < self.render_textures.len() {
            self.render_textures[rtidx].is_hidden()
        } else {
            true // Default to hidden if index is out of bounds
        }
    }

    /// Set render texture hidden state (matches OpenGL GlPixel interface)
    /// 
    /// # Parameters
    /// - `rtidx`: Render texture index (0-3)
    /// - `hidden`: New hidden state
    pub fn set_render_texture_hidden(&mut self, rtidx: usize, hidden: bool) {
        if rtidx < self.render_textures.len() {
            self.render_textures[rtidx].set_hidden(hidden);
        }
    }

    /// Get a reference to a specific render texture
    /// 
    /// # Parameters
    /// - `rtidx`: Render texture index (0-3)
    /// 
    /// # Returns
    /// Optional reference to the render texture
    pub fn get_render_texture(&self, rtidx: usize) -> Option<&WgpuRenderTexture> {
        self.render_textures.get(rtidx)
    }

    /// Get a mutable reference to a specific render texture
    /// 
    /// # Parameters
    /// - `rtidx`: Render texture index (0-3)
    /// 
    /// # Returns
    /// Optional mutable reference to the render texture
    pub fn get_render_texture_mut(&mut self, rtidx: usize) -> Option<&mut WgpuRenderTexture> {
        self.render_textures.get_mut(rtidx)
    }

    /// Render to a specific render texture (similar to OpenGL bind_target)
    /// 
    /// This method sets up the render pass to render to a specific render texture
    /// instead of the screen, enabling off-screen rendering for transition effects.
    /// 
    /// # Parameters
    /// - `encoder`: Command encoder for the render pass
    /// - `rtidx`: Render texture index (0-3)
    /// 
    /// # Returns
    /// Result containing the render pass or error
    pub fn begin_render_to_texture<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        rtidx: usize,
    ) -> Result<wgpu::RenderPass<'a>, String> {
        if rtidx >= self.render_textures.len() {
            return Err(format!("Render texture index {} out of bounds", rtidx));
        }

        let render_texture = &self.render_textures[rtidx];
        
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("Render to Texture {}", rtidx)),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_texture.get_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        Ok(render_pass)
    }

    /// Draw a render texture to screen or another render target using General2D renderer
    /// 
    /// This method provides the same interface as OpenGL GlPixel.draw_general2d(),
    /// allowing WGPU mode to render render textures with transformations and color modulation.
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `encoder`: Command encoder for render commands
    /// - `target_view`: Target render view (screen or render texture)
    /// - `rtidx`: Render texture index (0-3)
    /// - `area`: Texture area [x, y, width, height] in texture coordinates (0.0-1.0)
    /// - `transform`: Transformation matrix
    /// - `color`: Color modulation
    pub fn draw_general2d(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        rtidx: usize,
        area: [f32; 4],
        transform: &WgpuTransform,
        color: &WgpuColor,
    ) -> Result<(), String> {
        if rtidx >= self.render_textures.len() {
            return Err(format!("Render texture index {} out of bounds", rtidx));
        }

        let render_texture = &self.render_textures[rtidx];

        // Configure General2D renderer
        self.general2d_renderer
            .set_render_texture_index(rtidx)
            .set_area(&area)
            .set_transform(transform)
            .set_color(color);

        // Prepare and draw with texture references
        if let Some(sampler) = render_texture.get_sampler() {
            self.general2d_renderer.prepare_draw_with_texture(
                device,
                queue,
                render_texture.get_view(),
                sampler,
            );
            self.general2d_renderer.draw(encoder, target_view);
        } else {
            return Err("Render texture missing sampler".to_string());
        }

        Ok(())
    }

    /// Render transition frame (matches OpenGL GlPixel::render_trans_frame interface)
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `encoder`: Command encoder for rendering
    /// - `target_texture_idx`: Target render texture index for rendering
    /// - `shader_idx`: Transition shader index (0-6)
    /// - `progress`: Transition progress (0.0 to 1.0)
    pub fn render_trans_frame_to_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_texture_idx: usize,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        // Ensure we have enough render textures
        if self.render_textures.len() < 2 {
            return Err("Not enough render textures for transition effect".to_string());
        }
        
        if target_texture_idx >= self.render_textures.len() {
            return Err(format!("Target texture index {} out of range", target_texture_idx));
        }

        // Get texture views for render textures 0 and 1 (source textures)
        let texture1_view = &self.render_textures[0].view;
        let texture2_view = &self.render_textures[1].view;
        
        // Get target texture view
        let target_view = &self.render_textures[target_texture_idx].view;

        // Set textures on transition renderer
        self.transition_renderer.set_textures(device, texture1_view, texture2_view);

        // Draw transition effect
        self.transition_renderer.draw_transition(
            device,
            queue,
            encoder,
            target_view,
            shader_idx,
            progress,
        );

        Ok(())
    }
    
    /// Render transition frame (matches OpenGL GlPixel::render_trans_frame interface)
    /// 
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `encoder`: Command encoder for rendering
    /// - `target_view`: Target view for rendering
    /// - `shader_idx`: Transition shader index (0-6)
    /// - `progress`: Transition progress (0.0 to 1.0)
    pub fn render_trans_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        // Ensure we have at least 2 render textures for transition
        if self.render_textures.len() < 2 {
            return Err("Not enough render textures for transition effect".to_string());
        }

        // Get texture views for render textures 0 and 1
        let texture1_view = &self.render_textures[0].view;
        let texture2_view = &self.render_textures[1].view;

        // Set textures on transition renderer
        self.transition_renderer.set_textures(device, texture1_view, texture2_view);

        // Draw transition effect
        self.transition_renderer.draw_transition(
            device,
            queue,
            encoder,
            target_view,
            shader_idx,
            progress,
        );

        Ok(())
    }

    /// Load the symbol texture from the specified path
    pub fn load_symbol_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_path: &str,
    ) -> Result<(), String> {
        // Load the texture file
        let texture_bytes = std::fs::read(texture_path)
            .map_err(|e| format!("Failed to read texture file {}: {}", texture_path, e))?;

        let texture_image = image::load_from_memory(&texture_bytes)
            .map_err(|e| format!("Failed to load texture image: {}", e))?
            .to_rgba8();

        let texture_width = texture_image.width();
        let texture_height = texture_image.height();

        println!(
            "WGPU Debug: Loaded symbol texture {}x{} from {}",
            texture_width, texture_height, texture_path
        );

        // Create WGPU texture (use linear format to exactly match GL mode)
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
            format: wgpu::TextureFormat::Rgba8Unorm, // Linear format like GL mode
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
        if let (Some(bind_group_layout), Some(symbol_texture), Some(uniform_buffer)) = (
            &self.bind_group_layout,
            &self.symbol_texture,
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
                        resource: wgpu::BindingResource::TextureView(&symbol_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(
                            symbol_texture.sampler.as_ref().unwrap(),
                        ),
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
            position: [-1.0, -1.0], // Bottom left
            tex_coords: [0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0], // Red
        },
        WgpuVertex {
            position: [1.0, -1.0], // Bottom right
            tex_coords: [1.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0], // Green
        },
        WgpuVertex {
            position: [1.0, 1.0], // Top right
            tex_coords: [1.0, 0.0],
            color: [0.0, 0.0, 1.0, 1.0], // Blue
        },
        WgpuVertex {
            position: [-1.0, 1.0], // Top left
            tex_coords: [0.0, 0.0],
            color: [1.0, 1.0, 0.0, 1.0], // Yellow
        },
    ];

    /// Index data for quad triangulation
    ///
    /// Two triangles: (0,1,2) and (2,3,0) forming a rectangle.
    const INDICES: &'static [u16] = &[
        0, 1, 2, // First triangle
        2, 3, 0, // Second triangle
    ];

    /// Prepare drawing with actual game buffer content
    pub fn prepare_draw_with_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &crate::render::buffer::Buffer,
    ) {
        // For now, generate vertices based on buffer content
        let vertices = self.symbol_renderer.generate_vertices_from_buffer(buffer);
        self.vertex_count = vertices.len() as u32;

        // Debug: Print rendering information (only first frame)
        static mut FIRST_FRAME: bool = true;
        unsafe {
            if FIRST_FRAME && vertices.len() > 0 {
                println!(
                    "WGPU Debug: Generated {} vertices, first 3 colors:",
                    vertices.len()
                );
                for i in 0..(3.min(vertices.len())) {
                    let v = &vertices[i];
                    println!(
                        "  Vertex {}: pos=[{:.2}, {:.2}], color=[{:.2}, {:.2}, {:.2}, {:.2}]",
                        i,
                        v.position[0],
                        v.position[1],
                        v.color[0],
                        v.color[1],
                        v.color[2],
                        v.color[3]
                    );
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
                color_filter: [1.0, 1.0, 1.0, 1.0],
            };
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Prepare drawing with processed render cells (preferred method)
    ///
    /// This method receives RenderCell data that has already been processed
    /// through the complete game rendering pipeline, including sprites,
    /// borders, logo, and other game elements.
    pub fn prepare_draw_with_render_cells(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_cells: &[crate::render::adapter::RenderCell],
    ) {
        let vertices = self
            .symbol_renderer
            .generate_vertices_from_render_cells(render_cells);
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
                color_filter: [1.0, 1.0, 1.0, 1.0],
            };
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Get the render pipeline (for internal access)
    pub fn get_render_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.base.render_pipelines.get(0)
    }

    /// Get the vertex buffer (for internal access)
    pub fn get_vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        self.vertex_buffer.as_ref()
    }

    /// Get the bind group (for internal access)
    pub fn get_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    /// Get the vertex count (for internal access)
    pub fn get_vertex_count(&self) -> u32 {
        self.vertex_count
    }

    /// Get mutable reference to the General2D renderer (for internal access)
    pub fn get_general2d_render_mut(&mut self) -> Option<&mut WgpuGeneral2dRender> {
        Some(&mut self.general2d_renderer)
    }

    /// Set the ratio parameters for coordinate transformation
    /// 
    /// This method configures the ratio parameters that are used for coordinate
    /// transformation to match the OpenGL version's behavior exactly.
    pub fn set_ratio(&mut self, ratio_x: f32, ratio_y: f32) {
        self.symbol_renderer.set_ratio(ratio_x, ratio_y);
    }
}

impl WgpuRender for WgpuPixelRender {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        // Use linear surface format to exactly match GL mode (no gamma correction)
        Self::new_with_format(canvas_width, canvas_height, wgpu::TextureFormat::Bgra8Unorm)
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
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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
                color_filter: [1.0, 1.0, 1.0, 1.0],
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
                        r: 0.0, // Black background for game
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
