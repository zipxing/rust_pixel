// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # WGPU Pixel Renderer Module
//!
//! Main pixel rendering implementation for WGPU pipeline.
//! Handles texture-based character and symbol rendering with
//! instanced drawing for high performance.

use super::render_general2d::WgpuGeneral2dRender;
use super::render_symbols::{
    WgpuQuadVertex, WgpuSymbolInstance, WgpuSymbolRenderer, WgpuTransformUniforms,
};
use super::render_transition::WgpuTransitionRender;
use super::shader_source;
use super::texture::WgpuRenderTexture;
use super::*;
use crate::render::graph::{UnifiedColor, UnifiedTransform};

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
    /// Color filter like GL mode (r, g, b, a) - Structure temporarily retained but not used
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

    /// Current instance count for drawing
    instance_count: u32,

    /// Base quad vertex buffer (shared for all symbols)
    quad_vertex_buffer: Option<wgpu::Buffer>,

    /// Instance buffer for per-symbol data
    instance_buffer: Option<wgpu::Buffer>,

    /// Index buffer for quad triangulation
    index_buffer: Option<wgpu::Buffer>,

    /// Uniform buffer for transform data
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

    /// Current render target (None = screen, Some(idx) = render texture)
    current_render_target: Option<usize>,

    /// Clear color for render operations
    clear_color: UnifiedColor,
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
            instance_count: 0,
            quad_vertex_buffer: None,
            instance_buffer: None,
            index_buffer: None,
            uniform_buffer: None,
            symbol_texture: None,
            bind_group_layout: None,
            bind_group: None,
            render_textures: Vec::new(),
            canvas_width,
            canvas_height,
            current_render_target: None,
            clear_color: UnifiedColor::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Initialize render textures (similar to OpenGL GlPixel)
    ///
    /// Creates 4 render textures for transition effects:
    /// - 0: transition texture 1 (hidden by default)
    /// - 1: transition texture 2 (hidden by default)
    /// - 2: main buffer (visible by default)
    /// - 3: transition buffer (visible by default)
    ///
    /// On Windows, uninitialized GPU memory may contain garbage (white patches),
    /// so we explicitly clear all render textures to black after creation.
    pub fn init_render_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        // Clear existing render textures
        self.render_textures.clear();

        // Create 4 render textures with appropriate hidden states
        // RT0, RT1: hidden (for transition effects)
        // RT2: visible (main buffer)
        // RT3: hidden (only shown during transitions)
        let rt_hidden = [true, true, false, true];

        for i in 0..4 {
            let render_texture = WgpuRenderTexture::new_with_format(
                device,
                self.canvas_width,
                self.canvas_height,
                self.surface_format, // Use surface format to match pipelines
                rt_hidden[i],
            )?;

            self.render_textures.push(render_texture);
        }

        // Immediately clear all render textures to avoid garbage on Windows Vulkan
        // RT0, RT1, RT3: Clear to TRANSPARENT (alpha=0) so they won't cover content if accidentally rendered
        // RT2: Clear to BLACK (the main content area)
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RT Init Clear Encoder"),
        });

        for (i, rt) in self.render_textures.iter().enumerate() {
            // RT2 (main buffer) clears to black, others clear to transparent
            let clear_color = if i == 2 {
                wgpu::Color::BLACK
            } else {
                wgpu::Color::TRANSPARENT  // alpha=0, won't cover anything if rendered
            };

            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RT{} Init Clear Pass", i)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: rt.get_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            // RenderPass drops here, ending the pass
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// Initialize General2D renderer for texture-to-texture rendering
    ///
    /// # Parameters
    /// - `device`: WGPU device handle
    pub fn init_general2d_renderer(&mut self, device: &wgpu::Device) {
        // Ensure General2D renderer uses the same format as surface
        self.general2d_renderer
            .create_shader_with_format(device, self.surface_format);
        self.general2d_renderer.create_buffer(device);
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

    /// Copy one render texture to another
    ///
    /// Uses wgpu's copy_texture_to_texture for efficient GPU-side copy.
    /// Much faster than rendering through a shader for static copies.
    ///
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `src_index`: Source render texture index (0-3)
    /// - `dst_index`: Destination render texture index (0-3)
    pub fn copy_rt(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_index: usize,
        dst_index: usize,
    ) {
        if src_index >= self.render_textures.len() || dst_index >= self.render_textures.len() {
            return;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Texture Copy Encoder"),
        });

        let src_texture = &self.render_textures[src_index].texture;
        let dst_texture = &self.render_textures[dst_index].texture;

        encoder.copy_texture_to_texture(
            src_texture.as_image_copy(),
            dst_texture.as_image_copy(),
            wgpu::Extent3d {
                width: self.canvas_width,
                height: self.canvas_height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        // Make destination texture visible
        self.render_textures[dst_index].set_hidden(false);
    }

    /// Get canvas size (matches OpenGL GlPixel interface)
    ///
    /// # Returns
    /// Tuple of (width, height) in pixels
    pub fn get_canvas_size(&self) -> (u32, u32) {
        (self.canvas_width, self.canvas_height)
    }

    /// Get render texture by index (for debugging and general access)
    ///
    /// This method provides access to render textures for debugging purposes,
    /// such as saving them to image files, and general render texture access.
    ///
    /// # Parameters
    /// - `index`: Render texture index (0-3)
    ///
    /// # Returns
    /// Reference to the render texture if it exists
    pub fn get_render_texture(&self, index: usize) -> Option<&WgpuRenderTexture> {
        self.render_textures.get(index)
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
    /// The returned render pass is fully configured with the instanced rendering pipeline.
    ///
    /// # Parameters
    /// - `encoder`: Command encoder for the render pass
    /// - `rtidx`: Render texture index (0-3)
    ///
    /// # Returns
    /// Result containing the fully configured render pass or error
    pub fn begin_render_to_texture<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        rtidx: usize,
    ) -> Result<wgpu::RenderPass<'a>, String> {
        if rtidx >= self.render_textures.len() {
            return Err(format!("Render texture index {} out of bounds", rtidx));
        }

        let render_texture = &self.render_textures[rtidx];

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("Render to Texture {}", rtidx)),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_texture.get_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.clear_color.r as f64,
                        g: self.clear_color.g as f64,
                        b: self.clear_color.b as f64,
                        a: self.clear_color.a as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Set up the instanced rendering pipeline automatically
        if let Some(pipeline) = self.base.render_pipelines.get(0) {
            render_pass.set_pipeline(pipeline);

            // Set quad vertex buffer (buffer 0)
            if let Some(quad_vertex_buffer) = &self.quad_vertex_buffer {
                render_pass.set_vertex_buffer(0, quad_vertex_buffer.slice(..));
            }

            // Set instance buffer (buffer 1)
            if let Some(instance_buffer) = &self.instance_buffer {
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            }

            // Set index buffer
            if let Some(index_buffer) = &self.index_buffer {
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            }

            // Set bind group with texture and uniform buffer
            if let Some(bind_group) = &self.bind_group {
                render_pass.set_bind_group(0, bind_group, &[]);
            }
        }

        Ok(render_pass)
    }

    /// Render texture to screen or another render target using General2D renderer
    ///
    /// This method provides the same interface as OpenGL GlPixel.render_texture_to_screen_impl(),
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
    pub fn render_texture_to_screen_impl(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        rtidx: usize,
        area: [f32; 4],
        transform: &UnifiedTransform,
        color: &UnifiedColor,
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

    /// Render transition frame to a specific render texture
    ///
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `encoder`: Command encoder for rendering
    /// - `src_texture1_idx`: First source render texture index
    /// - `src_texture2_idx`: Second source render texture index
    /// - `target_texture_idx`: Target render texture index for rendering
    /// - `shader_idx`: Transition shader index (0-6)
    /// - `progress`: Transition progress (0.0 to 1.0)
    pub fn render_trans_frame_to_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src_texture1_idx: usize,
        src_texture2_idx: usize,
        target_texture_idx: usize,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        // Ensure we have enough render textures
        if self.render_textures.len() < 2 {
            return Err("Not enough render textures for transition effect".to_string());
        }

        if target_texture_idx >= self.render_textures.len() {
            return Err(format!(
                "Target texture index {} out of range",
                target_texture_idx
            ));
        }

        if src_texture1_idx >= self.render_textures.len() || src_texture2_idx >= self.render_textures.len() {
            return Err("Source texture index out of range".to_string());
        }

        // Get texture views for specified source textures
        let texture1_view = &self.render_textures[src_texture1_idx].view;
        let texture2_view = &self.render_textures[src_texture2_idx].view;

        // Get target texture view
        let target_view = &self.render_textures[target_texture_idx].view;

        // Set textures on transition renderer
        self.transition_renderer
            .set_textures(device, texture1_view, texture2_view);

        // Draw transition effect with the correct target format
        self.transition_renderer.draw_transition_with_format(
            device,
            queue,
            encoder,
            target_view,
            self.surface_format, // Use the same format as render textures
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
    /// - `src_texture1_idx`: First source render texture index
    /// - `src_texture2_idx`: Second source render texture index
    /// - `shader_idx`: Transition shader index (0-6)
    /// - `progress`: Transition progress (0.0 to 1.0)
    pub fn render_trans_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        src_texture1_idx: usize,
        src_texture2_idx: usize,
        shader_idx: usize,
        progress: f32,
    ) -> Result<(), String> {
        // Ensure we have at least 2 render textures for transition
        if self.render_textures.len() < 2 {
            return Err("Not enough render textures for transition effect".to_string());
        }

        if src_texture1_idx >= self.render_textures.len() || src_texture2_idx >= self.render_textures.len() {
            return Err("Source texture index out of range".to_string());
        }

        // Get texture views for specified source textures
        let texture1_view = &self.render_textures[src_texture1_idx].view;
        let texture2_view = &self.render_textures[src_texture2_idx].view;

        // Set textures on transition renderer
        self.transition_renderer
            .set_textures(device, texture1_view, texture2_view);

        // Draw transition effect with the correct target format
        self.transition_renderer.draw_transition_with_format(
            device,
            queue,
            encoder,
            target_view,
            self.surface_format, // Use the same format as render textures
            shader_idx,
            progress,
        );

        Ok(())
    }

    /// Load the symbol texture from pre-loaded data
    ///
    /// This is the preferred method when using init_pixel_assets() which pre-loads
    /// the texture data into memory.
    pub fn load_symbol_texture_from_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_width: u32,
        texture_height: u32,
        texture_data: &[u8],
    ) -> Result<(), String> {
        self.load_symbol_texture_internal(device, queue, texture_width, texture_height, texture_data)
    }

    /// Load the symbol texture from the specified path (legacy method)
    ///
    /// Prefer using load_symbol_texture_from_data() with pre-loaded texture data.
    /// Note: This method is not available on wasm32 targets as it uses std::fs.
    #[cfg(not(target_arch = "wasm32"))]
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

        self.load_symbol_texture_internal(device, queue, texture_width, texture_height, &texture_image)
    }

    /// Internal method for loading symbol texture
    fn load_symbol_texture_internal(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_width: u32,
        texture_height: u32,
        texture_data: &[u8],
    ) -> Result<(), String> {

        // Symbol texture loaded successfully (debug output removed for performance)

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
            texture.as_image_copy(),
            texture_data,
            wgpu::TexelCopyBufferLayout {
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
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
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

        // Load texture data into symbol renderer
        self.symbol_renderer.load_texture(
            texture_width as i32,
            texture_height as i32,
            texture_data,
        );

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

    // Removed unused VERTICES and INDICES constants

    /// Prepare drawing with actual game buffer content
    pub fn prepare_draw_with_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &crate::render::buffer::Buffer,
    ) {
        // Note: This method exists for compatibility but is simplified
        // In practice, prepare_draw_with_render_cells is used directly
        // since the adapter already converts buffers to render cells

        // For now, create an empty render cells array
        // This will be properly implemented when the full adapter pipeline is connected
        let render_cells = vec![];
        self.prepare_draw_with_render_cells(device, queue, &render_cells);
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
        // Generate instance data using symbol renderer
        self.symbol_renderer.generate_instances_from_render_cells(
            render_cells,
            self.symbol_renderer.ratio_x,
            self.symbol_renderer.ratio_y,
        );

        // Get instance data from symbol renderer
        let instances = self.symbol_renderer.get_instance_buffer();
        self.instance_count = self.symbol_renderer.get_instance_count();

        // Upload instance data
        if let Some(instance_buffer) = &self.instance_buffer {
            queue.write_buffer(instance_buffer, 0, bytemuck::cast_slice(instances));
        }

        // Upload transform uniform data
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let uniforms = self.symbol_renderer.get_transform_uniforms();
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Get the render pipeline (for internal access)
    pub fn get_render_pipeline(&self) -> Option<&wgpu::RenderPipeline> {
        self.base.render_pipelines.get(0)
    }

    /// Get the quad vertex buffer (for internal access)
    pub fn get_vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        self.quad_vertex_buffer.as_ref()
    }

    /// Get the instance buffer (for internal access)
    pub fn get_instance_buffer(&self) -> Option<&wgpu::Buffer> {
        self.instance_buffer.as_ref()
    }

    /// Get the index buffer (for internal access)
    pub fn get_index_buffer(&self) -> Option<&wgpu::Buffer> {
        self.index_buffer.as_ref()
    }

    /// Get the bind group (for internal access)
    pub fn get_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    /// Get the instance count (for internal access)
    pub fn get_vertex_count(&self) -> u32 {
        self.instance_count
    }

    /// Get mutable reference to the General2D renderer (for internal access)
    pub fn get_general2d_render_mut(&mut self) -> Option<&mut WgpuGeneral2dRender> {
        Some(&mut self.general2d_renderer)
    }

    /// Get current instance count for drawing operations
    pub fn get_instance_count(&self) -> u32 {
        self.instance_count
    }

    /// Set the ratio parameters for coordinate transformation
    ///
    /// This method configures the ratio parameters that are used for coordinate
    /// transformation to match the OpenGL version's behavior exactly.
    pub fn set_ratio(&mut self, ratio_x: f32, ratio_y: f32) {
        self.symbol_renderer.set_ratio(ratio_x, ratio_y);
    }

    /// Set CAS (Contrast Adaptive Sharpening) intensity for the General2D renderer
    ///
    /// Applied during the final RT-to-screen composition (Stage 4).
    /// 0.0 = off, 0.5 = moderate, 1.0 = maximum.
    pub fn set_sharpness(&mut self, sharpness: f32) {
        self.general2d_renderer.set_sharpness(sharpness);
    }

    /// Set whether MSDF/SDF rendering is enabled for TUI/CJK regions.
    pub fn set_msdf_enabled(&mut self, enabled: bool) {
        self.symbol_renderer.set_msdf_enabled(enabled);
    }

    /// Get whether MSDF/SDF rendering is currently enabled.
    pub fn get_msdf_enabled(&self) -> bool {
        self.symbol_renderer.get_msdf_enabled()
    }

    /// Bind screen as render target (matches OpenGL GlPixel interface)
    ///
    /// This method sets the render target to screen, equivalent to OpenGL's
    /// glBindFramebuffer(GL_FRAMEBUFFER, 0).
    pub fn bind_screen(&mut self) {
        self.current_render_target = None;
    }

    /// Bind render texture as render target (matches OpenGL GlPixel interface)
    ///
    /// This method sets the render target to a specific render texture,
    /// equivalent to OpenGL's glBindFramebuffer(GL_FRAMEBUFFER, texture_fbo).
    ///
    /// # Parameters
    /// - `render_texture_idx`: Index of render texture to bind (0-3)
    pub fn bind_target(&mut self, render_texture_idx: usize) {
        self.current_render_target = Some(render_texture_idx);
    }

    /// Set clear color for render operations (matches OpenGL GlPixel interface)
    ///
    /// # Parameters
    /// - `color`: Clear color to use for subsequent clear operations
    pub fn set_clear_color(&mut self, color: UnifiedColor) {
        self.clear_color = color;
    }

    /// Clear current render target (matches OpenGL GlPixel interface)
    ///
    /// This method clears the current render target with the configured clear color.
    /// Note: In WGPU, clearing happens when beginning a render pass, so this method
    /// stores the clear request to be applied during the next render operation.
    pub fn clear(&mut self) {
        // In WGPU, clearing is handled when beginning render pass
        // The clear_color is already stored and will be used automatically
    }

    /// Render RenderCell buffer to current render target (matches OpenGL GlPixel interface)
    ///
    /// This method provides the same interface as OpenGL GlPixel.render_rbuf(),
    /// allowing WGPU mode to render RenderCell data with the same API.
    ///
    /// # Parameters
    /// - `device`: WGPU device handle
    /// - `queue`: WGPU queue handle
    /// - `rbuf`: RenderCell data array
    /// - `ratio_x`: X-axis ratio for coordinate transformation
    /// - `ratio_y`: Y-axis ratio for coordinate transformation
    pub fn render_rbuf(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rbuf: &[crate::render::adapter::RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
        // Set ratios for coordinate transformation
        self.symbol_renderer.set_ratio(ratio_x, ratio_y);
        
        // Prepare draw data
        self.prepare_draw(device, queue);
        self.prepare_draw_with_render_cells(device, queue, rbuf);
    }

    /// Render to current bound target (used after render_rbuf)
    ///
    /// This method executes the actual rendering to the currently bound target,
    /// either screen or render texture, matching the OpenGL workflow.
    ///
    /// # Parameters
    /// - `encoder`: Command encoder for render commands
    /// - `screen_view`: Optional screen view (required if rendering to screen)
    ///
    /// # Returns
    /// Result indicating success or error
    pub fn render_to_current_target(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        screen_view: Option<&wgpu::TextureView>,
    ) -> Result<(), String> {
        match self.current_render_target {
            None => {
                // Render to screen
                if let Some(view) = screen_view {
                    self.render_to_screen(encoder, view);
                } else {
                    return Err("Screen view required for screen rendering".to_string());
                }
            }
            Some(rtidx) => {
                // Render to render texture
                let render_pass_result = self.begin_render_to_texture(encoder, rtidx);
                if let Ok(mut render_pass) = render_pass_result {
                    render_pass.draw_indexed(0..6, 0, 0..self.instance_count);
                } else {
                    return Err(format!("Failed to begin render to texture {}", rtidx));
                }
            }
        }
        Ok(())
    }

    /// Render to screen with current clear color
    ///
    /// # Parameters
    /// - `encoder`: Command encoder for render commands
    /// - `view`: Target screen view
    fn render_to_screen(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pixel Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.clear_color.r as f64,
                        g: self.clear_color.g as f64,
                        b: self.clear_color.b as f64,
                        a: self.clear_color.a as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Set pipeline and buffers for instanced rendering
        if let Some(pipeline) = self.base.render_pipelines.get(0) {
            render_pass.set_pipeline(pipeline);

            // Set quad vertex buffer (buffer 0)
            if let Some(quad_vertex_buffer) = &self.quad_vertex_buffer {
                render_pass.set_vertex_buffer(0, quad_vertex_buffer.slice(..));
            }

            // Set instance buffer (buffer 1)
            if let Some(instance_buffer) = &self.instance_buffer {
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            }

            // Set index buffer
            if let Some(index_buffer) = &self.index_buffer {
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            }

            // Set bind group with texture and uniform buffer
            if let Some(bind_group) = &self.bind_group {
                render_pass.set_bind_group(0, bind_group, &[]);
            }

            // Draw using instanced rendering
            if self.instance_count > 0 {
                render_pass.draw_indexed(0..6, 0, 0..self.instance_count);
            }
        }
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
        // Create shader modules for instanced rendering
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Symbol Instanced Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source::SYMBOLS_INSTANCED_VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Symbol Instanced Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(
                shader_source::SYMBOLS_INSTANCED_FRAGMENT_SHADER.into(),
            ),
        });

        // Create bind group layout for instanced rendering
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Symbol Instanced Bind Group Layout"),
            entries: &[
                // Uniform buffer (transform data)
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
                label: Some("Symbol Instanced Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Symbol Instanced Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[WgpuQuadVertex::desc(), WgpuSymbolInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: Some("fs_main"),
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
        // Create base quad vertex buffer (shared by all instances)
        let quad_vertices = WgpuSymbolRenderer::get_base_quad_vertices();
        let quad_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Vertex Buffer"),
            size: (quad_vertices.len() * std::mem::size_of::<WgpuQuadVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create instance buffer sized by cell count (not pixel count)
        let sym_w = crate::render::PIXEL_SYM_WIDTH.get().copied().unwrap_or(16.0);
        let sym_h = crate::render::PIXEL_SYM_HEIGHT.get().copied().unwrap_or(16.0);
        let cols = (self.base.canvas_width as f32 / sym_w).ceil() as u32;
        let rows = (self.base.canvas_height as f32 / sym_h).ceil() as u32;
        let max_instances = (cols * rows * 2).max(65536); // *2 margin, min 64K
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Symbol Instance Buffer"),
            size: (max_instances as usize * std::mem::size_of::<WgpuSymbolInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create index buffer for triangulated quad
        let quad_indices = WgpuSymbolRenderer::get_base_quad_indices();
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Index Buffer"),
            size: (quad_indices.len() * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create uniform buffer for transform data
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transform Uniform Buffer"),
            size: std::mem::size_of::<WgpuTransformUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Store buffers
        self.quad_vertex_buffer = Some(quad_vertex_buffer);
        self.instance_buffer = Some(instance_buffer);
        self.index_buffer = Some(index_buffer);
        self.uniform_buffer = Some(uniform_buffer);
    }

    fn prepare_draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // Upload base quad vertex data
        let quad_vertices = WgpuSymbolRenderer::get_base_quad_vertices();
        if let Some(quad_vertex_buffer) = &self.quad_vertex_buffer {
            queue.write_buffer(quad_vertex_buffer, 0, bytemuck::cast_slice(quad_vertices));
        }

        // Upload quad index data
        let quad_indices = WgpuSymbolRenderer::get_base_quad_indices();
        if let Some(index_buffer) = &self.index_buffer {
            queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(quad_indices));
        }

        // Upload transform uniform data
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let uniforms = self.symbol_renderer.get_transform_uniforms();
            queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }

        // Initialize instance count to 0 (will be set in prepare_draw_with_render_cells)
        self.instance_count = 0;
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
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Set pipeline and buffers for instanced rendering
        if let Some(pipeline) = self.base.render_pipelines.get(0) {
            render_pass.set_pipeline(pipeline);

            // Set quad vertex buffer (buffer 0)
            if let Some(quad_vertex_buffer) = &self.quad_vertex_buffer {
                render_pass.set_vertex_buffer(0, quad_vertex_buffer.slice(..));
            }

            // Set instance buffer (buffer 1)
            if let Some(instance_buffer) = &self.instance_buffer {
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
            }

            // Set index buffer
            if let Some(index_buffer) = &self.index_buffer {
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            }

            // Set bind group with texture and uniform buffer
            if let Some(bind_group) = &self.bind_group {
                render_pass.set_bind_group(0, bind_group, &[]);
            }

            // Draw using instanced rendering
            if self.instance_count > 0 {
                let quad_indices = WgpuSymbolRenderer::get_base_quad_indices();
                render_pass.draw_indexed(0..quad_indices.len() as u32, 0, 0..self.instance_count);
            }
        }
    }

    fn cleanup(&mut self, _device: &wgpu::Device) {
        // Cleanup per-frame state if needed
        // For now, no cleanup required
    }
}


