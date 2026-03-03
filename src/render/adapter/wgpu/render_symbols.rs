// RustPixel
// copyright zipxing@hotmail.com 2022～2026

//! WGPU Symbol Renderer (Instanced Rendering)
//!
//! This module implements instanced rendering for symbols and mirrors the
//! OpenGL renderer's behavior. A single base quad is reused, while per-symbol
//! data (UVs, color, local transform and world transform) is supplied via
//! per-instance attributes and a small uniform block.
//!
//! Key points:
//! - Texture atlas: Symbols come from a grid-based atlas defined by
//!   `PIXEL_SYM_WIDTH/HEIGHT`.
//! - Transform chain parity: The exact same translate/rotate/translate/scale
//!   chain is applied here as in the OpenGL backend to ensure pixel-perfect
//!   parity.
//! - Ratio handling: Display scaling (`ratio_x`, `ratio_y`) is handled by a
//!   combination of upstream sizing and per-instance scaling here, matching
//!   the contract in `graph.rs`.

use crate::render::adapter::RenderCell;
use crate::render::cell::cellsym_block;
use crate::render::graph::UnifiedTransform;
use crate::render::symbol_map::{Tile, MipUV, get_layered_symbol_map};

/// Symbol instance data for WGPU (matches OpenGL layout exactly)
///
/// This is the per-instance attribute payload consumed by the instanced
/// vertex shader. The layout mirrors the OpenGL instance buffer:
/// - `a1`: origin and UV top-left
/// - `a2`: UV size and first column of the local 2x2 matrix multiplied by frame size
/// - `a3`: second column of the local 2x2 matrix multiplied by frame size, and translation
/// - `a4`: layer index (for Texture2DArray), reserved fields
/// - `color`: per-instance color modulation
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuSymbolInstance {
    /// a1: [origin_x, origin_y, uv_left, uv_top]
    pub a1: [f32; 4],
    /// a2: [uv_width, uv_height, m00*width, m10*height]
    pub a2: [f32; 4],
    /// a3: [m01*width, m11*height, m20, m21]
    pub a3: [f32; 4],
    /// a4: [layer_index, reserved, reserved, reserved]
    pub a4: [f32; 4],
    /// color: [r, g, b, a]
    pub color: [f32; 4],
}

unsafe impl bytemuck::Pod for WgpuSymbolInstance {}
unsafe impl bytemuck::Zeroable for WgpuSymbolInstance {}

/// Transform uniform data (matches OpenGL UBO layout)
///
/// The two vec4 values encode a 2x2 matrix and translation (tw/th) along with
/// canvas size in the w components. This block is kept aligned with the OpenGL
/// UBO for consistent math in the shader.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuTransformUniforms {
    /// tw: [m00, m10, m20, canvas_width] 
    pub tw: [f32; 4],
    /// th: [m01, m11, m21, canvas_height]
    pub th: [f32; 4],
    /// colorFilter: [r, g, b, a]
    pub color_filter: [f32; 4],
}

unsafe impl bytemuck::Pod for WgpuTransformUniforms {}
unsafe impl bytemuck::Zeroable for WgpuTransformUniforms {}

/// Base quad vertex for instanced rendering
///
/// The base quad spans unit coordinates [0,1] x [0,1]. Local transforms map
/// this unit quad into screen-space according to the per-instance attributes.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WgpuQuadVertex {
    /// Vertex position in unit quad coordinates (0,0) to (1,1)
    pub position: [f32; 2],
}

unsafe impl bytemuck::Pod for WgpuQuadVertex {}
unsafe impl bytemuck::Zeroable for WgpuQuadVertex {}

/// WGPU Symbol Renderer using instanced rendering
///
/// This renderer exactly matches the OpenGL `GlRenderSymbols` behavior:
/// - Uses a single base quad geometry
/// - Each symbol is an instance with its own transform and UV data
/// - Performs the same complex transformation chain in the vertex shader
pub struct WgpuSymbolRenderer {
    canvas_width: f32,
    canvas_height: f32,

    /// Transform stack (equivalent to OpenGL transform_stack)
    transform_stack: UnifiedTransform,

    /// Current instance buffer data
    instance_buffer: Vec<WgpuSymbolInstance>,

    /// Instance count for current frame
    instance_count: usize,

    /// Max instances capacity
    max_instances: usize,

    /// Ratio parameters for coordinate transformation
    pub ratio_x: f32,
    pub ratio_y: f32,

    /// Viewport scale factor: physical_window_height / canvas_height.
    /// Used to convert render-space cell heights to physical screen pixel heights
    /// for accurate mipmap level selection. Updated on window resize.
    viewport_scale: f32,

    /// Render scale for HiDPI/Retina: physical_size / logical_size.
    /// Used to scale render coordinates so content renders at correct physical size.
    render_scale: f32,
}




impl WgpuSymbolRenderer {
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        // Initialize transform stack to match OpenGL version exactly:
        // Y is flipped (m11 = -1) and translated by canvas height.
        // Equivalent to: UnifiedTransform::new_with_values(1, 0, 0, -1, 0, canvas_h)
        let transform_stack = UnifiedTransform::new_with_values(
            1.0, 0.0,                    // m00, m01
            0.0, -1.0,                   // m10, m11  
            0.0, canvas_height as f32    // m20, m21
        );
        
        let max_instances = (canvas_width * canvas_height) as usize; // Conservative estimate
        
        Self {
            canvas_width: canvas_width as f32,
            canvas_height: canvas_height as f32,
            transform_stack,
            instance_buffer: Vec::with_capacity(max_instances),
            instance_count: 0,
            max_instances,
            ratio_x: 1.0,
            ratio_y: 1.0,
            viewport_scale: 1.0,
            render_scale: 1.0,
        }
    }
    
    /// Generate instance data from render cells (equivalent to OpenGL `render_rbuf`)
    ///
    /// Applies the same transform chain as OpenGL to ensure parity:
    /// 1) translate(r.x + r.cx - r.w, r.y + r.cy - r.h)
    /// 2) if angle != 0 → rotate(angle)
    /// 3) translate(-r.cx + r.w, -r.cy + r.h)
    /// 4) scale(cell_size_compensation × ratio_compensation)
    ///
    /// Notes:
    /// - `r.w`/`r.h` (from `RenderCell`) are destination pixel sizes already
    ///   adjusted by upstream ratio/sprite scaling; here we compute a relative
    ///   scaling against the default symbol size so that custom sprite scaling
    ///   is preserved.
    /// - `ratio_x`/`ratio_y` are applied to keep DPI scaling parity with GL.
    pub fn generate_instances_from_render_cells(&mut self, render_cells: &[RenderCell], ratio_x: f32, ratio_y: f32) {
        self.instance_buffer.clear();
        self.instance_count = 0;
        self.generate_instances_layered(render_cells, ratio_x, ratio_y);
    }

    /// Layered instance generation (Texture2DArray mode, bitmap only, no MSDF)
    /// Select mipmap level based on screen pixel height.
    ///
    /// Thresholds are per base unit (PIXEL_SYMBOL_SIZE = 16px):
    /// - >= 48px/unit → mip0 (×4, highest resolution)
    /// - >= 24px/unit → mip1 (×2, mid resolution)
    /// - < 24px/unit  → mip2 (×1, base resolution)
    #[inline]
    pub(crate) fn select_mip_level(screen_pixel_h: f32, cell_h: u8) -> usize {
        let per_unit = screen_pixel_h / cell_h.max(1) as f32;
        if per_unit >= 48.0 { 0 }
        else if per_unit >= 24.0 { 1 }
        else { 2 }
    }

    fn generate_instances_layered(&mut self, render_cells: &[RenderCell], ratio_x: f32, ratio_y: f32) {
        const MOD_BOLD: u16 = 0x0001;
        const MOD_DIM: u16 = 0x0002;
        const MOD_ITALIC: u16 = 0x0004;
        const MOD_UNDERLINED: u16 = 0x0008;
        const MOD_REVERSED: u16 = 0x0040;
        const MOD_HIDDEN: u16 = 0x0080;
        const MOD_CROSSED_OUT: u16 = 0x0100;
        const MOD_GLOW: u16 = 0x0400;
        const ITALIC_SKEW: f32 = 0.21;

        let base_w = crate::render::PIXEL_SYM_WIDTH.get().copied().unwrap_or(32.0);
        let base_h = crate::render::PIXEL_SYM_HEIGHT.get().copied().unwrap_or(32.0);

        // BG fill tile (always use mip1 — it's a solid block, mip level irrelevant)
        // BG_FILL_SYMBOL linear index 160 = block 0, idx 160
        let bg_pua = cellsym_block(0, 160);
        let bg_tile = get_layered_symbol_map()
            .map(|m| *m.resolve(&bg_pua))
            .unwrap_or_default();
        let bg_mip = bg_tile.mips[1];
        let bg_frame_w = bg_tile.cell_w.max(1) as f32 * base_w;
        let bg_frame_h = bg_tile.cell_h.max(1) as f32 * base_h;

        let rs = self.render_scale;  // HiDPI scale factor

        for r in render_cells {
            // Scale coordinates by render_scale for HiDPI/Retina displays
            let cell_width = r.w as f32 * rs;
            let cell_height = r.h as f32 * rs;
            let rx = r.x * rs;
            let ry = r.y * rs;
            let rcx = r.cx * rs;
            let rcy = r.cy * rs;
            let modifier = r.modifier;

            let (mut fg_color, bg_color) = if modifier & MOD_REVERSED != 0 {
                let bg = r.bcolor.unwrap_or((0.0, 0.0, 0.0, 0.0));
                let fg = r.fcolor;
                (bg, Some(fg))
            } else {
                (r.fcolor, r.bcolor)
            };

            if modifier & MOD_BOLD != 0 {
                fg_color.0 = (fg_color.0 * 1.3).min(1.0);
                fg_color.1 = (fg_color.1 * 1.3).min(1.0);
                fg_color.2 = (fg_color.2 * 1.3).min(1.0);
            }
            if modifier & MOD_DIM != 0 { fg_color.3 *= 0.6; }
            if modifier & MOD_HIDDEN != 0 { fg_color.3 = 0.0; }

            // Background
            if let Some(b) = bg_color {
                let mut bg_transform = self.transform_stack;
                bg_transform.translate(rx + rcx - cell_width, ry + rcy - cell_height);
                if r.angle != 0.0 { bg_transform.rotate(r.angle); }
                bg_transform.translate(-rcx + cell_width, -rcy + cell_height);
                let bg_fw = bg_frame_w / ratio_x;
                let bg_fh = bg_frame_h / ratio_y;
                bg_transform.scale(cell_width / bg_fw / ratio_x, cell_height / bg_fh / ratio_y);
                self.draw_layered_instance(&bg_tile, &bg_mip, bg_frame_w, bg_frame_h, &bg_transform, [b.0, b.1, b.2, b.3], false);
            }

            // Tile is carried directly from Cell — zero lookup
            let tile = r.tile;

            // Dynamic mipmap selection based on physical screen pixel size.
            // viewport_scale converts render-space cell_height to physical pixels.
            let mip_level = Self::select_mip_level(cell_height * self.viewport_scale, tile.cell_h);
            let mip = tile.mips[mip_level];
            let frame_width = tile.cell_w.max(1) as f32 * base_w;
            let frame_height = tile.cell_h.max(1) as f32 * base_h;

            let is_bold = modifier & MOD_BOLD != 0;
            let is_glow = modifier & MOD_GLOW != 0;

            // Glow halo
            if is_glow {
                let glow_color = [fg_color.0, fg_color.1, fg_color.2, fg_color.3 * 0.4];
                let mut glow_transform = self.transform_stack;
                glow_transform.translate(rx + rcx - cell_width, ry + rcy - cell_height);
                if r.angle != 0.0 { glow_transform.rotate(r.angle); }
                glow_transform.translate(-rcx + cell_width, -rcy + cell_height);
                if modifier & MOD_ITALIC != 0 { glow_transform.skew_x(ITALIC_SKEW); }
                let fw = frame_width / ratio_x;
                let fh = frame_height / ratio_y;
                glow_transform.scale(cell_width / fw / ratio_x, cell_height / fh / ratio_y);
                self.draw_layered_instance_with_glow(&tile, &mip, frame_width, frame_height, &glow_transform, glow_color, is_bold, 2.5);
            }

            // Foreground
            let mut transform = self.transform_stack;
            transform.translate(rx + rcx - cell_width, ry + rcy - cell_height);
            if r.angle != 0.0 { transform.rotate(r.angle); }
            transform.translate(-rcx + cell_width, -rcy + cell_height);
            if modifier & MOD_ITALIC != 0 { transform.skew_x(ITALIC_SKEW); }

            let fw = frame_width / ratio_x;
            let fh = frame_height / ratio_y;
            transform.scale(cell_width / fw / ratio_x, cell_height / fh / ratio_y);
            self.draw_layered_instance(&tile, &mip, frame_width, frame_height, &transform, [fg_color.0, fg_color.1, fg_color.2, fg_color.3], is_bold);

            // Underline
            if modifier & MOD_UNDERLINED != 0 {
                let mut lt = self.transform_stack;
                lt.translate(rx + rcx - cell_width, ry + rcy - cell_height + cell_height * 0.9);
                if r.angle != 0.0 { lt.rotate(r.angle); }
                lt.translate(-rcx + cell_width, -rcy + cell_height);
                let bg_fw = bg_frame_w / ratio_x;
                let bg_fh = bg_frame_h / ratio_y;
                lt.scale(cell_width / bg_fw / ratio_x, (cell_height * 0.08) / bg_fh / ratio_y);
                self.draw_layered_instance(&bg_tile, &bg_mip, bg_frame_w, bg_frame_h, &lt, [fg_color.0, fg_color.1, fg_color.2, fg_color.3], false);
            }

            // Strikethrough
            if modifier & MOD_CROSSED_OUT != 0 {
                let mut lt = self.transform_stack;
                lt.translate(rx + rcx - cell_width, ry + rcy - cell_height + cell_height * 0.46);
                if r.angle != 0.0 { lt.rotate(r.angle); }
                lt.translate(-rcx + cell_width, -rcy + cell_height);
                let bg_fw = bg_frame_w / ratio_x;
                let bg_fh = bg_frame_h / ratio_y;
                lt.scale(cell_width / bg_fw / ratio_x, (cell_height * 0.08) / bg_fh / ratio_y);
                self.draw_layered_instance(&bg_tile, &bg_mip, bg_frame_w, bg_frame_h, &lt, [fg_color.0, fg_color.1, fg_color.2, fg_color.3], false);
            }
        }
    }

    /// Add a symbol instance for layered mode (Texture2DArray).
    ///
    /// Uses Tile data from LayeredSymbolMap for UV + layer coordinates.
    /// No MSDF path — all layered rendering is bitmap-only.
    fn draw_layered_instance(
        &mut self,
        tile: &Tile,
        mip: &MipUV,
        frame_width: f32,
        frame_height: f32,
        transform: &UnifiedTransform,
        color: [f32; 4],
        is_bold: bool,
    ) {
        if self.instance_count >= self.max_instances { return; }
        // Skip rendering for missing/unknown symbols (mip size is 0)
        if mip.w <= 0.0 || mip.h <= 0.0 { return; }

        let origin_x = if is_bold { -1.0_f32 } else { 1.0 };
        let origin_y = 1.0_f32; // No MSDF flag in layered mode

        // Half-texel inset to prevent bilinear filtering from sampling adjacent symbols
        let half_texel = 0.5 / 2048.0; // layer_size = 2048
        let uv_left = mip.x + half_texel;
        let uv_top = mip.y + half_texel;
        let uv_width = mip.w - half_texel * 2.0;
        let uv_height = mip.h - half_texel * 2.0;

        let instance = WgpuSymbolInstance {
            a1: [origin_x, origin_y, uv_left, uv_top],
            a2: [
                uv_width,
                uv_height,
                transform.m00 * frame_width,
                transform.m10 * frame_width,
            ],
            a3: [
                transform.m01 * frame_height,
                transform.m11 * frame_height,
                transform.m20,
                transform.m21,
            ],
            a4: [mip.layer as f32, 0.0, 0.0, 0.0],
            color,
        };

        self.instance_buffer.push(instance);
        self.instance_count += 1;
    }

    /// Add a glow halo instance for layered mode.
    fn draw_layered_instance_with_glow(
        &mut self,
        tile: &Tile,
        mip: &MipUV,
        frame_width: f32,
        frame_height: f32,
        transform: &UnifiedTransform,
        color: [f32; 4],
        is_bold: bool,
        glow_scale: f32,
    ) {
        if self.instance_count >= self.max_instances { return; }
        // Skip rendering for missing/unknown symbols (mip size is 0)
        if mip.w <= 0.0 || mip.h <= 0.0 { return; }

        let origin_x = if is_bold { -1.0_f32 } else { 1.0 };
        let origin_y = 1.0_f32;

        let half_texel = 0.5 / 2048.0;
        let uv_left = mip.x + half_texel;
        let uv_top = mip.y + half_texel;
        let uv_width_raw = mip.w - half_texel * 2.0;
        let uv_height_val = mip.h - half_texel * 2.0;

        let gs = glow_scale;
        let mat_00_fw = transform.m00 * frame_width;
        let mat_10_fw = transform.m10 * frame_width;
        let mat_01_fh = transform.m01 * frame_height;
        let mat_11_fh = transform.m11 * frame_height;

        // Center-preserving enlargement (same formula as legacy)
        let cx = 0.5 - 1.0;  // -0.5 when origin is 1.0
        let cy = 0.5 - 1.0;
        let center_x = mat_00_fw * cx + mat_01_fh * cy;
        let center_y = mat_10_fw * cx + mat_11_fh * cy;
        let tx = transform.m20 - (gs - 1.0) * center_x;
        let ty = transform.m21 - (gs - 1.0) * center_y;

        // Encode glow flag in uv_width sign: negative = glow
        let uv_width = -uv_width_raw;

        let instance = WgpuSymbolInstance {
            a1: [origin_x, origin_y, uv_left, uv_top],
            a2: [uv_width, uv_height_val, mat_00_fw * gs, mat_10_fw * gs],
            a3: [mat_01_fh * gs, mat_11_fh * gs, tx, ty],
            a4: [mip.layer as f32, 0.0, 0.0, 0.0],
            color,
        };

        self.instance_buffer.push(instance);
        self.instance_count += 1;
    }

    /// Get base quad vertices (equivalent to OpenGL quad vertices)
    pub fn get_base_quad_vertices() -> &'static [WgpuQuadVertex] {
        // Base quad vertices in unit coordinates (matches OpenGL TRIANGLE_FAN order)
        // OpenGL uses: [0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]
        &[
            WgpuQuadVertex { position: [0.0, 0.0] }, // Bottom-left
            WgpuQuadVertex { position: [0.0, 1.0] }, // Top-left  
            WgpuQuadVertex { position: [1.0, 1.0] }, // Top-right
            WgpuQuadVertex { position: [1.0, 0.0] }, // Bottom-right
        ]
    }
    
    /// Get base quad indices for triangle list rendering
    pub fn get_base_quad_indices() -> &'static [u16] {
        // Convert TRIANGLE_FAN to TRIANGLE_LIST: (0,1,2) and (2,3,0)
        &[0, 1, 2, 2, 3, 0]
    }
    
    /// Get current instance buffer data
    pub fn get_instance_buffer(&self) -> &[WgpuSymbolInstance] {
        &self.instance_buffer[..self.instance_count]
    }
    
    /// Get instance count
    pub fn get_instance_count(&self) -> u32 {
        self.instance_count as u32
    }
    
    /// Get transform uniforms (equivalent to OpenGL UBO)
    pub fn get_transform_uniforms(&self) -> WgpuTransformUniforms {
        WgpuTransformUniforms {
            // tw: [m00, m10, m20, canvas_width]
            tw: [
                self.transform_stack.m00,
                self.transform_stack.m10, 
                self.transform_stack.m20,
                self.canvas_width
            ],
            // th: [m01, m11, m21, canvas_height]
            th: [
                self.transform_stack.m01,
                self.transform_stack.m11,
                self.transform_stack.m21,
                self.canvas_height
            ],
            // colorFilter: [r, g, b, a] - default to white (no filtering)
            color_filter: [1.0, 1.0, 1.0, 1.0],
        }
    }
    
    /// Update canvas dimensions
    pub fn update_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width as f32;
        self.canvas_height = height as f32;
        
        // Update transform stack canvas height (matches OpenGL behavior)
        self.transform_stack.m21 = height as f32;
    }
    
    /// Set ratio parameters for coordinate transformation
    pub fn set_ratio(&mut self, ratio_x: f32, ratio_y: f32) {
        self.ratio_x = ratio_x;
        self.ratio_y = ratio_y;
    }

    /// Set viewport scale factor (physical_window_height / canvas_height).
    /// Called before each frame to ensure mipmap selection accounts for window size.
    pub fn set_viewport_scale(&mut self, scale: f32) {
        self.viewport_scale = scale;
    }

    /// Set render scale for HiDPI/Retina displays.
    /// render_scale = physical_size / logical_size.
    pub fn set_render_scale(&mut self, scale: f32) {
        self.render_scale = scale;
    }

    /// Generate vertices from buffer (legacy method for compatibility)
    /// This method exists for backward compatibility but is no longer used
    /// in the instanced rendering pipeline
    pub fn generate_vertices_from_buffer(&self, _buffer: &crate::render::buffer::Buffer) -> Vec<crate::render::adapter::wgpu::pixel::WgpuVertex> {
        // Return empty vector since we now use instanced rendering
        Vec::new()
    }
}

impl WgpuQuadVertex {
    /// Vertex buffer layout descriptor for base quad
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuQuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl WgpuSymbolInstance {
    /// Instance buffer layout descriptor
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WgpuSymbolInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // a1: vec4<f32> at location 1
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a2: vec4<f32> at location 2
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a3: vec4<f32> at location 3
                wgpu::VertexAttribute {
                    offset: (2 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // a4: vec4<f32> at location 4 (layer_index, reserved...)
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color: vec4<f32> at location 5
                wgpu::VertexAttribute {
                    offset: (4 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====================================================================
    // select_mip_level tests
    // ====================================================================

    #[test]
    fn test_mip_level_high_res() {
        // 96px screen height / 1 cell = 96 per-unit → mip0
        assert_eq!(WgpuSymbolRenderer::select_mip_level(96.0, 1), 0);
        // 48px exactly → mip0
        assert_eq!(WgpuSymbolRenderer::select_mip_level(48.0, 1), 0);
        // 200px → mip0
        assert_eq!(WgpuSymbolRenderer::select_mip_level(200.0, 1), 0);
    }

    #[test]
    fn test_mip_level_mid_res() {
        // 47px → mip1 (just below mip0 threshold)
        assert_eq!(WgpuSymbolRenderer::select_mip_level(47.0, 1), 1);
        // 24px exactly → mip1
        assert_eq!(WgpuSymbolRenderer::select_mip_level(24.0, 1), 1);
        // 36px → mip1
        assert_eq!(WgpuSymbolRenderer::select_mip_level(36.0, 1), 1);
    }

    #[test]
    fn test_mip_level_low_res() {
        // 23px → mip2 (just below mip1 threshold)
        assert_eq!(WgpuSymbolRenderer::select_mip_level(23.0, 1), 2);
        // 16px → mip2
        assert_eq!(WgpuSymbolRenderer::select_mip_level(16.0, 1), 2);
        // 1px → mip2
        assert_eq!(WgpuSymbolRenderer::select_mip_level(1.0, 1), 2);
    }

    #[test]
    fn test_mip_level_multi_cell() {
        // 96px / 2 cells = 48 per-unit → mip0
        assert_eq!(WgpuSymbolRenderer::select_mip_level(96.0, 2), 0);
        // 48px / 2 cells = 24 per-unit → mip1
        assert_eq!(WgpuSymbolRenderer::select_mip_level(48.0, 2), 1);
        // 24px / 2 cells = 12 per-unit → mip2
        assert_eq!(WgpuSymbolRenderer::select_mip_level(24.0, 2), 2);
    }

    #[test]
    fn test_mip_level_zero_cell_h() {
        // cell_h=0 should not panic (clamped to 1)
        assert_eq!(WgpuSymbolRenderer::select_mip_level(48.0, 0), 0);
        assert_eq!(WgpuSymbolRenderer::select_mip_level(24.0, 0), 1);
        assert_eq!(WgpuSymbolRenderer::select_mip_level(10.0, 0), 2);
    }

    #[test]
    fn test_mip_level_boundary_values() {
        // Test exact boundary at 48.0
        assert_eq!(WgpuSymbolRenderer::select_mip_level(48.0, 1), 0);
        assert_eq!(WgpuSymbolRenderer::select_mip_level(47.999, 1), 1);
        // Test exact boundary at 24.0
        assert_eq!(WgpuSymbolRenderer::select_mip_level(24.0, 1), 1);
        assert_eq!(WgpuSymbolRenderer::select_mip_level(23.999, 1), 2);
    }

    #[test]
    fn test_mip_level_return_range() {
        // Ensure return value is always 0, 1, or 2
        for h in [0.1, 1.0, 10.0, 23.0, 24.0, 47.0, 48.0, 100.0, 1000.0] {
            for cell_h in [1, 2, 4] {
                let level = WgpuSymbolRenderer::select_mip_level(h, cell_h);
                assert!(level <= 2, "mip level {} out of range for h={}, cell_h={}", level, h, cell_h);
            }
        }
    }
}
