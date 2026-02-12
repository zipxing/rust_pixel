// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # WGPU Render Core
//!
//! Shared rendering logic for both native (WinitWgpu) and web (WgpuWeb) adapters.
//! This module eliminates code duplication by providing a unified rendering core.

use crate::render::adapter::wgpu::pixel::WgpuPixelRender;
use crate::render::adapter::wgpu::WgpuRender;
use crate::render::adapter::{RenderCell, RtComposite};
use crate::render::graph::{UnifiedColor, UnifiedTransform};

/// Core WGPU rendering functionality shared between adapters
///
/// Contains the essential WGPU resources and provides unified rendering methods
/// that work identically on both native and web platforms.
pub struct WgpuRenderCore {
    /// WGPU device for creating resources
    pub device: wgpu::Device,
    /// WGPU queue for submitting commands
    pub queue: wgpu::Queue,
    /// Main pixel renderer
    pub pixel_renderer: WgpuPixelRender,
    /// X-axis scaling ratio
    pub ratio_x: f32,
    /// Y-axis scaling ratio
    pub ratio_y: f32,
}

impl WgpuRenderCore {
    /// Create a new render core with the given WGPU resources
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        pixel_renderer: WgpuPixelRender,
        ratio_x: f32,
        ratio_y: f32,
    ) -> Self {
        Self {
            device,
            queue,
            pixel_renderer,
            ratio_x,
            ratio_y,
        }
    }

    /// Set the scaling ratios
    pub fn set_ratio(&mut self, rx: f32, ry: f32) {
        self.ratio_x = rx;
        self.ratio_y = ry;
        self.pixel_renderer.set_ratio(rx, ry);
    }

    /// Get canvas dimensions
    pub fn canvas_size(&self) -> (u32, u32) {
        (self.pixel_renderer.canvas_width, self.pixel_renderer.canvas_height)
    }

    /// Render buffer to render texture
    ///
    /// Converts RenderCell data and renders it to the specified render texture.
    ///
    /// # Parameters
    /// - `rbuf`: RenderCell data array
    /// - `rtidx`: Target render texture index (0-3)
    /// - `debug`: Enable debug mode (red background for debugging)
    pub fn rbuf2rt(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool) {
        // Bind target render texture
        self.pixel_renderer.bind_target(rtidx);

        // Set clear color
        if debug {
            self.pixel_renderer.set_clear_color(UnifiedColor::new(1.0, 0.0, 0.0, 1.0));
        } else {
            self.pixel_renderer.set_clear_color(UnifiedColor::new(0.0, 0.0, 0.0, 1.0));
        }

        // Clear target
        self.pixel_renderer.clear();

        // Render RenderCell data to the currently bound target
        self.pixel_renderer.render_rbuf(&self.device, &self.queue, rbuf, self.ratio_x, self.ratio_y);

        // Create command encoder for rendering to texture
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!("Render to RT{} Encoder", rtidx)),
        });

        // Execute rendering to the currently bound target
        if let Err(e) = self.pixel_renderer.render_to_current_target(&mut encoder, None) {
            log::error!("WgpuRenderCore: render error: {}", e);
            return;
        }

        // Submit rendering commands
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Set render texture visibility
    pub fn set_rt_visible(&mut self, texture_index: usize, visible: bool) {
        self.pixel_renderer.set_render_texture_hidden(texture_index, !visible);
    }

    /// Get render texture hidden state
    pub fn get_rt_hidden(&self, texture_index: usize) -> bool {
        self.pixel_renderer.get_render_texture_hidden(texture_index)
    }

    /// Blend two render textures with transition effect
    ///
    /// # Parameters
    /// - `src1`: Source RT 1 index
    /// - `src2`: Source RT 2 index
    /// - `target`: Target RT index
    /// - `effect`: Effect type (0=Mosaic, 1=Heart, etc.)
    /// - `progress`: Transition progress (0.0-1.0)
    pub fn blend_rts(&mut self, src1: usize, src2: usize, target: usize, effect: usize, progress: f32) {
        // Make destination RT visible
        self.pixel_renderer.set_render_texture_hidden(target, false);

        // Create command encoder for transition
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Transition Encoder"),
        });

        // Render transition effect to target texture
        if let Err(e) = self.pixel_renderer.render_trans_frame_to_texture(
            &self.device,
            &self.queue,
            &mut encoder,
            src1,
            src2,
            target,
            effect,
            progress,
        ) {
            log::error!("WgpuRenderCore: transition error: {}", e);
            return;
        }

        // Submit transition commands
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Copy one render texture to another
    pub fn copy_rt(&mut self, src_index: usize, dst_index: usize) {
        self.pixel_renderer.copy_rt(&self.device, &self.queue, src_index, dst_index);
    }

    /// Present render textures to screen
    ///
    /// # Parameters
    /// - `surface_view`: The surface texture view to render to
    /// - `composites`: Array of RtComposite items to render in order
    pub fn present(&mut self, surface_view: &wgpu::TextureView, composites: &[RtComposite]) {
        // Bind screen as render target
        self.pixel_renderer.bind_screen();

        // Create command encoder for screen composition
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Present Encoder"),
        });

        // Clear screen
        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Screen Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        let pcw = self.pixel_renderer.canvas_width as f32;
        let pch = self.pixel_renderer.canvas_height as f32;

        // Render each composite in order
        for composite in composites {
            let rtidx = composite.rt;

            // Skip hidden RTs
            if self.pixel_renderer.get_render_texture_hidden(rtidx) {
                continue;
            }

            // Calculate area and transform based on viewport
            let (area, transform) = if let Some(ref vp) = composite.viewport {
                let vp_x = vp.x as f32;
                let vp_y = vp.y as f32;
                let pw = vp.w as f32;
                let ph = vp.h as f32;

                // Get content size for texture sampling
                let (content_w, content_h) = composite.content_size
                    .map(|(w, h)| (w as f32, h as f32))
                    .unwrap_or((pw, ph));

                // area controls TEXTURE SAMPLING
                let area = [0.0, 0.0, content_w / pcw, content_h / pch];

                // transform controls SCREEN POSITION
                let mut base_transform = UnifiedTransform::new();
                base_transform.scale(pw / pcw, ph / pch);

                // NDC coordinates: -1 to 1, center is (0, 0)
                let tx = (2.0 * vp_x + pw - pcw) / pcw;
                let ty = (pch - 2.0 * vp_y - ph) / pch;
                base_transform.translate(tx, ty);

                // Apply composite transform if specified
                let final_transform = if let Some(ref user_transform) = composite.transform {
                    base_transform.compose(user_transform)
                } else {
                    base_transform
                };

                (area, final_transform)
            } else {
                // Fullscreen
                let area = [0.0, 0.0, 1.0, 1.0];
                let transform = composite.transform.clone().unwrap_or_else(UnifiedTransform::new);
                (area, transform)
            };

            // Create color with alpha
            let alpha_f = composite.alpha as f32 / 255.0;
            let color = UnifiedColor::new(1.0, 1.0, 1.0, alpha_f);

            // Render this RT to screen
            if let Err(e) = self.pixel_renderer.render_texture_to_screen_impl(
                &self.device,
                &self.queue,
                &mut encoder,
                surface_view,
                rtidx,
                area,
                &transform,
                &color,
            ) {
                log::error!("WgpuRenderCore: present error: {}", e);
            }
        }

        // Submit screen composition commands
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Present with default settings (RT2 fullscreen)
    pub fn present_default(&mut self, surface_view: &wgpu::TextureView) {
        self.present(surface_view, &[RtComposite::fullscreen(2)]);
    }
}

/// Builder for creating WgpuRenderCore with proper initialization
pub struct WgpuRenderCoreBuilder {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub surface_format: wgpu::TextureFormat,
    pub ratio_x: f32,
    pub ratio_y: f32,
}

impl WgpuRenderCoreBuilder {
    pub fn new(canvas_width: u32, canvas_height: u32, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            canvas_width,
            canvas_height,
            surface_format,
            ratio_x: 1.0,
            ratio_y: 1.0,
        }
    }

    pub fn with_ratio(mut self, rx: f32, ry: f32) -> Self {
        self.ratio_x = rx;
        self.ratio_y = ry;
        self
    }

    /// Build the render core with the given device, queue, and texture data
    pub fn build(
        self,
        device: wgpu::Device,
        queue: wgpu::Queue,
        tex_width: u32,
        tex_height: u32,
        tex_data: &[u8],
    ) -> Result<WgpuRenderCore, String> {
        let mut pixel_renderer = WgpuPixelRender::new_with_format(
            self.canvas_width,
            self.canvas_height,
            self.surface_format,
        );

        // Initialize all WGPU components
        pixel_renderer.load_symbol_texture_from_data(
            &device,
            &queue,
            tex_width,
            tex_height,
            tex_data,
        )?;

        pixel_renderer.create_shader(&device);
        pixel_renderer.create_buffer(&device);
        pixel_renderer.create_bind_group(&device);

        pixel_renderer.init_render_textures(&device)?;

        pixel_renderer.init_general2d_renderer(&device);
        pixel_renderer.init_transition_renderer(&device);
        pixel_renderer.set_ratio(self.ratio_x, self.ratio_y);

        Ok(WgpuRenderCore::new(
            device,
            queue,
            pixel_renderer,
            self.ratio_x,
            self.ratio_y,
        ))
    }
}
