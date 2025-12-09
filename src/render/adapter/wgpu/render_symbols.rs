// RustPixel
// copyright zipxing@hotmail.com 2022～2025

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

use crate::render::adapter::{RenderCell, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use crate::render::graph::UnifiedTransform;

/// Symbol instance data for WGPU (matches OpenGL layout exactly)
///
/// This is the per-instance attribute payload consumed by the instanced
/// vertex shader. The layout mirrors the OpenGL instance buffer:
/// - `a1`: origin and UV top-left
/// - `a2`: UV size and first column of the local 2x2 matrix multiplied by frame size
/// - `a3`: second column of the local 2x2 matrix multiplied by frame size, and translation
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

/// Symbol frame data (equivalent to OpenGL GlCell)
#[derive(Clone, Debug)]
pub struct WgpuSymbolFrame {
    pub width: f32,
    pub height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub uv_left: f32,
    pub uv_top: f32,
    pub uv_width: f32,
    pub uv_height: f32,
}

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
    
    /// Symbol frames (equivalent to OpenGL symbols)
    symbols: Vec<WgpuSymbolFrame>,
    
    /// Current instance buffer data
    instance_buffer: Vec<WgpuSymbolInstance>,
    
    /// Instance count for current frame
    instance_count: usize,
    
    /// Max instances capacity
    max_instances: usize,
    
    /// Ratio parameters for coordinate transformation
    pub ratio_x: f32,
    pub ratio_y: f32,
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
            symbols: Vec::new(),
            instance_buffer: Vec::with_capacity(max_instances),
            instance_count: 0,
            max_instances,
            ratio_x: 1.0,
            ratio_y: 1.0,
        }
    }
    
    /// Load symbol texture with support for TUI (8x16) and Emoji (16x16)
    ///
    /// Maintains original row-major order for backward compatibility:
    /// - Indices 0-12287: Rows 0-95, all 8x8 Sprites (original layout preserved)
    /// - Indices 12288+: Rows 96-127, TUI (8x16) and Emoji (16x16) mixed
    pub fn load_texture(&mut self, texw: i32, texh: i32, _texdata: &[u8]) {
        self.symbols.clear();
        
        let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");
        
        let th = (texh as f32 / sym_height) as usize;  // Grid rows (e.g., 128 for 2048/16)
        let tw = (texw as f32 / sym_width) as usize;   // Grid cols (e.g., 128 for 2048/16)
        
        // Layout constants (based on 1024x1024 texture with 128x128 grid)
        const SPRITE_ROWS: usize = 96;  // Rows 0-95 for sprites
        const TUI_COLS: usize = 80;     // Cols 0-79 for TUI in rows 96+
        
        // Traverse in row-major order (same as original code)
        for i in 0..th {
            for j in 0..tw {
                let pixel_x = j as f32 * sym_width;
                let pixel_y = i as f32 * sym_height;
                
                // Determine symbol size based on row position
                let (width, height) = if i < SPRITE_ROWS {
                    // Rows 0-95: Standard 8x8 sprites (indices 0-12287)
                    (sym_width, sym_height)
                } else if j < TUI_COLS {
                    // Rows 96-127, Cols 0-79: TUI 8x16 characters
                    (sym_width, sym_height * 2.0)
                } else {
                    // Rows 96-127, Cols 80-127: Emoji 16x16
                    (sym_width * 2.0, sym_height * 2.0)
                };
                
                let frame = self.make_symbols_frame_custom(
                    texw as f32, texh as f32,
                    pixel_x, pixel_y,
                    width, height,
                );
                self.symbols.push(frame);
            }
        }
    }
    
    /// Create a symbol frame (equivalent to OpenGL `make_symbols_frame`)
    ///
    /// Packs local width/height, origin, and UV rectangle for a single symbol.
    // fn make_symbols_frame(&self, tex_width: f32, tex_height: f32, x: f32, y: f32) -> WgpuSymbolFrame {
    //     let sym_width = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
    //     let sym_height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");
        
    //     self.make_symbols_frame_custom(tex_width, tex_height, x, y, sym_width, sym_height)
    // }
    
    /// Create a symbol frame with custom dimensions
    ///
    /// Used for TUI (8x16), Emoji (16x16), and Sprite (8x8) regions.
    fn make_symbols_frame_custom(&self, tex_width: f32, tex_height: f32, x: f32, y: f32, width: f32, height: f32) -> WgpuSymbolFrame {
        let origin_x = 1.0;
        let origin_y = 1.0;
        let uv_left = x / tex_width;
        let uv_top = y / tex_height;
        let uv_width = width / tex_width;
        let uv_height = height / tex_height;
        
        WgpuSymbolFrame {
            width,
            height,
            origin_x,
            origin_y,
            uv_left,
            uv_top,
            uv_width,
            uv_height,
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
        // Modifier bit flags (matching Modifier enum in style.rs)
        // 样式修饰符位标志（与 style.rs 中的 Modifier 枚举匹配）
        const MOD_BOLD: u16 = 0x0001;
        const MOD_DIM: u16 = 0x0002;
        const MOD_ITALIC: u16 = 0x0004;
        const MOD_UNDERLINED: u16 = 0x0008;
        const MOD_REVERSED: u16 = 0x0040;
        const MOD_HIDDEN: u16 = 0x0080;
        const MOD_CROSSED_OUT: u16 = 0x0100;
        
        // ITALIC slant factor: tan(12°) ≈ 0.21
        // ITALIC 倾斜因子：tan(12°) ≈ 0.21
        const ITALIC_SKEW: f32 = 0.21;
        
        self.instance_buffer.clear();
        self.instance_count = 0;
        
        for r in render_cells {
            let cell_width = r.w as f32;
            let cell_height = r.h as f32;
            
            // Apply modifier effects to colors
            // 应用样式修饰符效果到颜色
            let modifier = r.modifier;
            
            // Get base colors (may be swapped if REVERSED)
            // 获取基础颜色（如果设置了 REVERSED 则交换）
            let (mut fg_color, bg_color) = if modifier & MOD_REVERSED != 0 {
                // REVERSED: swap foreground and background colors
                // REVERSED: 交换前景色和背景色
                let bg = r.bcolor.unwrap_or((0.0, 0.0, 0.0, 0.0));
                let fg = r.fcolor;
                (bg, Some(fg))
            } else {
                (r.fcolor, r.bcolor)
            };
            
            // Apply BOLD effect: multiply RGB by 1.3, clamp to 1.0
            // BOLD 效果：RGB 值乘以 1.3，限制在 1.0 以内
            if modifier & MOD_BOLD != 0 {
                fg_color.0 = (fg_color.0 * 1.3).min(1.0);
                fg_color.1 = (fg_color.1 * 1.3).min(1.0);
                fg_color.2 = (fg_color.2 * 1.3).min(1.0);
            }
            
            // Apply DIM effect: multiply alpha by 0.6
            // DIM 效果：Alpha 值乘以 0.6
            if modifier & MOD_DIM != 0 {
                fg_color.3 *= 0.6;
            }
            
            // Apply HIDDEN effect: set alpha to 0.0
            // HIDDEN 效果：Alpha 值设为 0.0
            if modifier & MOD_HIDDEN != 0 {
                fg_color.3 = 0.0;
            }
            
            // Background rendering - needs separate transform calculation
            // Background使用独立的transform，因为background symbol (1280) 的frame尺寸
            // 与foreground symbol可能不同（如TUI字符是16x32，而填充符号是16x16）
            if let Some(b) = bg_color {
                let mut bg_transform = self.transform_stack;
                bg_transform.translate(
                    r.x + r.cx - r.w as f32,
                    r.y + r.cy - r.h as f32,
                );
                if r.angle != 0.0 {
                    bg_transform.rotate(r.angle);
                }
                bg_transform.translate(
                    -r.cx + r.w as f32,
                    -r.cy + r.h as f32,
                );
                
                // Background symbol 1280 has its own frame size, scale to match cell size
                // 背景符号1280有自己的frame尺寸，需要缩放以匹配cell尺寸
                let bg_frame = &self.symbols[1280];
                let bg_frame_width = bg_frame.width / ratio_x;
                let bg_frame_height = bg_frame.height / ratio_y;
                let bg_scale_x = cell_width / bg_frame_width / ratio_x;
                let bg_scale_y = cell_height / bg_frame_height / ratio_y;
                bg_transform.scale(bg_scale_x, bg_scale_y);
                
                self.draw_symbol_instance(1280, &bg_transform, [b.0, b.1, b.2, b.3]);
            }
            
            // Foreground rendering
            let mut transform = self.transform_stack;
            transform.translate(
                r.x + r.cx - r.w as f32,
                r.y + r.cy - r.h as f32,
            );
            if r.angle != 0.0 {
                transform.rotate(r.angle);
            }
            transform.translate(
                -r.cx + r.w as f32,
                -r.cy + r.h as f32,
            );
            
            // Apply ITALIC effect: horizontal skew transformation
            // ITALIC 效果：水平倾斜变换
            if modifier & MOD_ITALIC != 0 {
                transform.skew_x(ITALIC_SKEW);
            }
            
            // Apply scaling based on RenderCell dimensions vs actual frame size.
            // This preserves per-sprite scaling beyond DPI ratio adjustments.
            // IMPORTANT: Use frame dimensions (not PIXEL_SYM_WIDTH/HEIGHT) because
            // TUI (16x32) and Emoji (32x32) have different sizes than Sprite (16x16).
            let frame = &self.symbols[r.texsym];
            let frame_width = frame.width / ratio_x;
            let frame_height = frame.height / ratio_y;
            
            let scale_x = cell_width / frame_width / ratio_x;
            let scale_y = cell_height / frame_height / ratio_y;
            
            transform.scale(scale_x, scale_y);
            
            // Draw foreground symbol with modified color
            self.draw_symbol_instance(r.texsym, &transform, [fg_color.0, fg_color.1, fg_color.2, fg_color.3]);
            
            // Draw UNDERLINED effect: a line at the bottom of the cell
            // UNDERLINED 效果：在单元格底部绘制线条
            // Uses symbol 1280 (background fill) scaled to line thickness
            if modifier & MOD_UNDERLINED != 0 {
                let mut line_transform = self.transform_stack;
                // Position at bottom of cell (90% down)
                let line_y = r.y + r.cy - r.h as f32 + cell_height * 0.9;
                line_transform.translate(r.x + r.cx - r.w as f32, line_y);
                if r.angle != 0.0 {
                    line_transform.rotate(r.angle);
                }
                line_transform.translate(-r.cx + r.w as f32, -r.cy + r.h as f32);
                
                // Scale to full width, thin height (10% of cell height)
                let line_frame = &self.symbols[1280];
                let line_scale_x = cell_width / (line_frame.width / ratio_x) / ratio_x;
                let line_scale_y = (cell_height * 0.08) / (line_frame.height / ratio_y) / ratio_y;
                line_transform.scale(line_scale_x, line_scale_y);
                
                self.draw_symbol_instance(1280, &line_transform, [fg_color.0, fg_color.1, fg_color.2, fg_color.3]);
            }
            
            // Draw CROSSED_OUT effect: a line through the middle of the cell
            // CROSSED_OUT 效果：在单元格中间绘制删除线
            if modifier & MOD_CROSSED_OUT != 0 {
                let mut line_transform = self.transform_stack;
                // Position at middle of cell (50% down, adjusted for line thickness)
                let line_y = r.y + r.cy - r.h as f32 + cell_height * 0.46;
                line_transform.translate(r.x + r.cx - r.w as f32, line_y);
                if r.angle != 0.0 {
                    line_transform.rotate(r.angle);
                }
                line_transform.translate(-r.cx + r.w as f32, -r.cy + r.h as f32);
                
                // Scale to full width, thin height (8% of cell height)
                let line_frame = &self.symbols[1280];
                let line_scale_x = cell_width / (line_frame.width / ratio_x) / ratio_x;
                let line_scale_y = (cell_height * 0.08) / (line_frame.height / ratio_y) / ratio_y;
                line_transform.scale(line_scale_x, line_scale_y);
                
                self.draw_symbol_instance(1280, &line_transform, [fg_color.0, fg_color.1, fg_color.2, fg_color.3]);
            }
        }
    }
    
    /// Add a symbol instance (equivalent to OpenGL `draw_symbol`)
    ///
    /// Packs per-instance attributes into the instance buffer:
    /// - a1: origin, UV top-left
    /// - a2: UV size, first column of local 2x2 scaled by frame size
    /// - a3: second column of local 2x2 scaled by frame size, then translation
    fn draw_symbol_instance(&mut self, sym: usize, transform: &UnifiedTransform, color: [f32; 4]) {
        if self.instance_count >= self.max_instances {
            // Instance limit reached (debug output removed for performance)
            return;
        }
        if sym >= self.symbols.len() {
            // Symbol index out of bounds (debug output removed for performance)
            return;
        }
        
        let frame = &self.symbols[sym];
        
        // Create instance data matching OpenGL layout exactly
        let instance = WgpuSymbolInstance {
            // a1: [origin_x, origin_y, uv_left, uv_top]
            a1: [frame.origin_x, frame.origin_y, frame.uv_left, frame.uv_top],
            
            // a2: [uv_width, uv_height, m00*width, m10*width]
            // Matrix column 1: both m00 and m10 multiply by width
            a2: [
                frame.uv_width, 
                frame.uv_height,
                transform.m00 * frame.width,
                transform.m10 * frame.width  // Fixed: was * frame.height
            ],
            
            // a3: [m01*height, m11*height, m20, m21]
            // Matrix column 2: both m01 and m11 multiply by height
            a3: [
                transform.m01 * frame.height,  // Fixed: was * frame.width
                transform.m11 * frame.height,
                transform.m20,
                transform.m21
            ],
            
            // color: [r, g, b, a]
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
                // color: vec4<f32> at location 4
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
