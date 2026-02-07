// RustPixel
// copyright zipxing@hotmail.com 2022～2025
// 
// OpenGL Symbol Renderer (Instanced Rendering)
//
// This module renders symbols using instanced drawing and mirrors the WGPU
// renderer's behavior. A single base quad is reused while per-instance data
// (UVs, color, local transform and world transform) are streamed each frame.
//
// Key points:
// - Texture atlas: Symbols come from a grid-based atlas defined by
//   `PIXEL_SYM_WIDTH/HEIGHT`.
// - Transform chain parity: The translate/rotate/translate/scale chain is kept
//   identical to the WGPU backend for pixel-perfect parity.
// - Ratio handling: Display scaling (`ratio_x`, `ratio_y`) is compensated by
//   computing a relative scale against the default symbol size and applying the
//   same ratio terms as WGPU.
use crate::render::adapter::gl::{
    shader::GlShader,
    shader_source::{FRAGMENT_SRC_SYMBOLS, VERTEX_SRC_SYMBOLS},
    texture::{GlCell, GlTexture, GlTextureArray},
    GlRender, GlRenderBase,
};
use crate::render::graph::{UnifiedColor, UnifiedTransform};
use crate::render::adapter::RenderCell;
use crate::render::symbol_map::{iter_symbol_frames, layout};
use glow::HasContext;
use log::info;

/// Background fill symbol - use centralized constant from symbol_map
const BG_FILL_SYMBOL: usize = layout::BG_FILL_SYMBOL;

pub struct GlRenderSymbols {
    pub base: GlRenderBase,
    instance_buffer: Vec<f32>,
    instance_buffer_capacity: usize,
    instance_buffer_at: isize,
    instance_count: usize,
    ubo_contents: [f32; 12],
    pub symbols: Vec<GlCell>,
    pub transform_stack: UnifiedTransform,
    pub transform_dirty: bool,
}

impl GlRender for GlRenderSymbols {
    fn new(canvas_width: u32, canvas_height: u32) -> Self {
        let base = GlRenderBase {
            id: 0,
            shader: vec![],
            shader_binded: false,
            vao: None,
            gl_buffers: vec![],
            textures: vec![],
            textures_binded: false,
            canvas_width,
            canvas_height,
        };
        let mut ubo_contents = [0.0f32; 12];
        ubo_contents[8] = 1.0;
        ubo_contents[9] = 1.0;
        ubo_contents[10] = 1.0;
        ubo_contents[11] = 1.0;

        Self {
            base,
            instance_buffer: vec![0.0; 1024],
            instance_buffer_capacity: 1024,
            instance_buffer_at: -1,
            instance_count: 0,
            ubo_contents,
            symbols: vec![],
            transform_stack: UnifiedTransform::new_with_values(
                1.0, 0.0, // m00, m01
                0.0, -1.0, // m10, m11  
                0.0, canvas_height as f32, // m20, m21
            ),
            transform_dirty: true,
        }
    }

    fn get_base(&mut self) -> &mut GlRenderBase {
        &mut self.base
    }

    fn create_shader(&mut self, gl: &glow::Context, ver: &str) {
        let rbs = self.get_base();
        rbs.shader.push(GlShader::new(
            gl,
            ver,
            VERTEX_SRC_SYMBOLS,
            FRAGMENT_SRC_SYMBOLS,
        ));
    }

    fn create_buffer(&mut self, gl: &glow::Context) {
        unsafe {
            let vao_symbolss = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao_symbolss));

            let instances_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances_vbo));
            let instance_buffer_capacity = 1024;
            gl.buffer_data_size(
                glow::ARRAY_BUFFER,
                (instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                glow::DYNAMIC_DRAW,
            );

            let quad_vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            // Base quad in unit space (TRIANGLE_FAN order):
            // [0,0], [0,1], [1,1], [1,0]
            let quad_vertices: [f32; 8] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0];
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                quad_vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            // Uniform buffer (matches WGPU transform uniform layout):
            // tw: [m00, m10, m20, canvas_width]
            // th: [m01, m11, m21, canvas_height]
            // color: [1,1,1,1]
            let ubo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(ubo));
            gl.buffer_data_size(glow::UNIFORM_BUFFER, 48, glow::DYNAMIC_DRAW);
            gl.bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(ubo));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances_vbo));

            // Per-instance attribute stride: 5 vec4 = 80 bytes (added a5 for tex_layer)
            let stride = 80;

            // Attribute 1 (a1: origin, UV top-left)
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, stride, 0);
            gl.vertex_attrib_divisor(1, 1);

            // Attribute 2 (a2: UV size, transform column 1)
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);
            gl.vertex_attrib_divisor(2, 1);

            // Attribute 3 (a3: transform column 2, translation)
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, stride, 32);
            gl.vertex_attrib_divisor(3, 1);

            // Attribute 4 (color)
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 4, glow::FLOAT, false, stride, 48);
            gl.vertex_attrib_divisor(4, 1);

            // Attribute 5 (a5: tex_layer, padding, padding, padding)
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(5, 4, glow::FLOAT, false, stride, 64);
            gl.vertex_attrib_divisor(5, 1);

            gl.bind_vertex_array(None);

            self.base.vao = Some(vao_symbolss);
            self.base.gl_buffers.clear();
            self.base.gl_buffers = vec![instances_vbo, quad_vbo, ubo];
        }
    }

    fn prepare_draw(&mut self, gl: &glow::Context) {
        // Instance size: 5 vec4 = 20 floats (a1, a2, a3, color, a5)
        let size = 20u32;

        if !self.base.textures_binded {
            unsafe {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.base.textures[0]));
            }
            self.base.textures_binded = true;
        }

        // When the transform stack changes, flush any pending instances,
        // update the UBO and resume batching.
        if self.transform_dirty {
            self.draw(gl);
            self.send_uniform_buffer(gl);
        }

        if !self.base.shader_binded {
            self.draw(gl);
            self.base.shader[0].bind(gl);
            self.base.shader_binded = true;
        }

        // Grow instance buffer when near capacity.
        if (self.instance_buffer_at + size as isize) as usize >= self.instance_buffer_capacity {
            self.instance_buffer_capacity *= 2;
            self.instance_buffer
                .resize(self.instance_buffer_capacity, 0.0);

            unsafe {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.base.gl_buffers[0]));
                gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    (self.instance_buffer_capacity * std::mem::size_of::<f32>()) as i32,
                    glow::DYNAMIC_DRAW,
                );
            }
        }

        self.instance_count += 1;
    }

    fn draw(&mut self, gl: &glow::Context) {
        if self.instance_count == 0 {
            return;
        }

        unsafe {
            // instances_vbo
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.base.gl_buffers[0]));
            gl.buffer_sub_data_u8_slice(
                glow::ARRAY_BUFFER,
                0,
                self.instance_buffer[0..=(self.instance_buffer_at as usize)]
                    .align_to::<u8>()
                    .1,
            );

            gl.bind_vertex_array(self.base.vao);
            gl.draw_arrays_instanced(glow::TRIANGLE_FAN, 0, 4, self.instance_count as i32);

            self.instance_buffer_at = -1;
            self.instance_count = 0;
            self.base.shader_binded = false;
            self.base.textures_binded = false;
        }
    }

    fn cleanup(&mut self, gl: &glow::Context) {}
}

impl GlRenderSymbols {
    /// Load symbol texture using centralized iterator from symbol_map
    ///
    /// Creates a 3-layer texture array:
    /// - Layer 0: Sprite/TUI/Emoji (from symbols.png)
    /// - Layer 1: CJK multi-size (from cjk.png: 16px + 32px + 64px front)
    /// - Layer 2: CJK 64px overflow (from cjk64.png: 64px back)
    ///
    /// Symbol indices and tex_layer assignment:
    /// - [0, 40959]: Sprite -> Layer 0
    /// - [40960, 43519]: TUI -> Layer 0
    /// - [43520, 44287]: Emoji -> Layer 0
    /// - [44288+]: CJK -> Layer 1
    pub fn load_texture(&mut self, gl: &glow::Context, texw: i32, texh: i32, texdata: &[u8]) {
        // Get CJK texture data for Layer 1 and Layer 2
        let cjk_tex_data = crate::get_pixel_cjk_texture_data();
        let cjk64_tex_data = crate::get_pixel_cjk64_texture_data();

        // Create a 3-layer texture array
        let texture_array = GlTextureArray::new(gl, texw, texh, 3).unwrap();

        // Upload symbols.png to Layer 0 (Sprite/TUI/Emoji)
        texture_array.upload_layer(gl, 0, texdata);
        // Upload cjk.png to Layer 1 (CJK multi-size: 16px + 32px + 64px front)
        texture_array.upload_layer(gl, 1, &cjk_tex_data.data);
        // Upload cjk64.png to Layer 2 (CJK 64px overflow)
        texture_array.upload_layer(gl, 2, &cjk64_tex_data.data);

        // Also create a regular GlTexture for UV calculations (same dimensions)
        let mut sprite_sheet = GlTexture::new(gl, texw, texh, texdata).unwrap();
        sprite_sheet.bind(gl);

        // Use centralized iterator from symbol_map
        let mut index = 0usize;
        for frame in iter_symbol_frames() {
            // Determine texture layer: Layer 0 for Sprite/TUI/Emoji, Layer 1 for CJK
            let tex_layer = if index >= layout::CJK_BASE { 1.0 } else { 0.0 };

            let symbol = self.make_symbols_frame_custom(
                &mut sprite_sheet,
                frame.pixel_x as f32,
                frame.pixel_y as f32,
                frame.width as f32,
                frame.height as f32,
                tex_layer,
            );
            self.symbols.push(symbol);
            index += 1;
        }

        info!(
            "symbols loaded: {} (Sprite: {}, TUI: {}, Emoji: {}, CJK: {}, Total: {})",
            self.symbols.len(),
            layout::SPRITE_TOTAL,
            layout::TUI_TOTAL,
            layout::EMOJI_TOTAL,
            layout::CJK_TOTAL,
            layout::SPRITE_TOTAL + layout::TUI_TOTAL + layout::EMOJI_TOTAL + layout::CJK_TOTAL
        );

        // Use the texture array instead of the regular 2D texture
        self.base.textures.clear();
        self.base.textures.push(texture_array.get_texture());
        self.base.textures_binded = false;
    }

    /// Upload the current transform stack and canvas size to the UBO
    fn send_uniform_buffer(&mut self, gl: &glow::Context) {
        let transform = self.transform_stack;
        self.ubo_contents[0] = transform.m00;
        self.ubo_contents[1] = transform.m10;
        self.ubo_contents[2] = transform.m20;
        self.ubo_contents[4] = transform.m01;
        self.ubo_contents[5] = transform.m11;
        self.ubo_contents[6] = transform.m21;
        self.ubo_contents[3] = self.base.canvas_width as f32;
        self.ubo_contents[7] = self.base.canvas_height as f32;

        unsafe {
            // ubo
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.base.gl_buffers[2]));
            gl.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                0,
                self.ubo_contents.align_to::<u8>().1,
            );
        }

        self.transform_dirty = false;
    }

    // Fill the instance buffer with one symbol's attributes
    // Layout: a1 (4), a2 (4), a3 (4), color (4), a5 (4) = 20 floats per instance
    fn draw_symbol(
        &mut self,
        gl: &glow::Context,
        sym: usize,
        transform: &UnifiedTransform,
        color: &UnifiedColor,
    ) {
        self.prepare_draw(gl);
        let frame = &self.symbols[sym];
        let instance_buffer = &mut self.instance_buffer;

        // a1: [origin_x, origin_y, uv_left, uv_top]
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.origin_x;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.origin_y;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_left;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_top;

        // a2: [uv_width, uv_height, m00*width, m10*width]
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_width;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.uv_height;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m00 * frame.width;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m10 * frame.width;

        // a3: [m01*height, m11*height, m20, m21]
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m01 * frame.height;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m11 * frame.height;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m20;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = transform.m21;

        // color: [r, g, b, a]
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.r;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.g;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.b;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = color.a;

        // a5: [tex_layer, padding, padding, padding]
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = frame.tex_layer;
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = 0.0; // padding
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = 0.0; // padding
        self.instance_buffer_at += 1;
        instance_buffer[self.instance_buffer_at as usize] = 0.0; // padding
    }

    /// Render all `RenderCell` instances by converting them into per-instance
    /// attributes and issuing a single instanced draw.
    pub fn render_rbuf(
        &mut self,
        gl: &glow::Context,
        rbuf: &[RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
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
        
        // Transform chain parity with WGPU:
        // 1) translate(r.x + r.cx - r.w, r.y + r.cy - r.h)
        // 2) if angle != 0 → rotate(angle)
        // 3) translate(-r.cx + r.w, -r.cy + r.h)
        // 4) scale(cell_size_compensation × ratio_compensation)
        for r in rbuf {
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
                let mut bg_transform = UnifiedTransform::new();
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
                
                // Background symbol (BG_FILL_SYMBOL, solid block in PETSCII) has its own frame size, scale to match cell size
                // 背景符号160 (PETSCII中的实心方块) 有自己的frame尺寸，需要缩放以匹配cell尺寸
                let bg_frame = &self.symbols[BG_FILL_SYMBOL];
                let bg_frame_width = bg_frame.width / ratio_x;
                let bg_frame_height = bg_frame.height / ratio_y;
                bg_transform.scale(cell_width / bg_frame_width / ratio_x, cell_height / bg_frame_height / ratio_y);

                let back_color = UnifiedColor::new(b.0, b.1, b.2, b.3);
                self.draw_symbol(gl, BG_FILL_SYMBOL, &bg_transform, &back_color);
            }

            // Foreground rendering
            let mut transform = UnifiedTransform::new();
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
            
            transform.scale(cell_width / frame_width / ratio_x, cell_height / frame_height / ratio_y);

            let color = UnifiedColor::new(fg_color.0, fg_color.1, fg_color.2, fg_color.3);
            // fill instance buffer for opengl instance rendering
            // r.texsym is calculated by push_render_buffer using block layout formula
            self.draw_symbol(gl, r.texsym, &transform, &color);
            
            // Draw UNDERLINED effect: a line at the bottom of the cell
            // UNDERLINED 效果：在单元格底部绘制线条
            // Uses BG_FILL_SYMBOL (solid block in PETSCII) scaled to line thickness
            if modifier & MOD_UNDERLINED != 0 {
                let mut line_transform = UnifiedTransform::new();
                // Position at bottom of cell (90% down)
                let line_y = r.y + r.cy - r.h as f32 + cell_height * 0.9;
                line_transform.translate(r.x + r.cx - r.w as f32, line_y);
                if r.angle != 0.0 {
                    line_transform.rotate(r.angle);
                }
                line_transform.translate(-r.cx + r.w as f32, -r.cy + r.h as f32);

                // Scale to full width, thin height (8% of cell height)
                let line_frame = &self.symbols[BG_FILL_SYMBOL];
                let line_scale_x = cell_width / (line_frame.width / ratio_x) / ratio_x;
                let line_scale_y = (cell_height * 0.08) / (line_frame.height / ratio_y) / ratio_y;
                line_transform.scale(line_scale_x, line_scale_y);

                let line_color = UnifiedColor::new(fg_color.0, fg_color.1, fg_color.2, fg_color.3);
                self.draw_symbol(gl, BG_FILL_SYMBOL, &line_transform, &line_color);
            }

            // Draw CROSSED_OUT effect: a line through the middle of the cell
            // CROSSED_OUT 效果：在单元格中间绘制删除线
            if modifier & MOD_CROSSED_OUT != 0 {
                let mut line_transform = UnifiedTransform::new();
                // Position at middle of cell (50% down, adjusted for line thickness)
                let line_y = r.y + r.cy - r.h as f32 + cell_height * 0.46;
                line_transform.translate(r.x + r.cx - r.w as f32, line_y);
                if r.angle != 0.0 {
                    line_transform.rotate(r.angle);
                }
                line_transform.translate(-r.cx + r.w as f32, -r.cy + r.h as f32);

                // Scale to full width, thin height (8% of cell height)
                let line_frame = &self.symbols[BG_FILL_SYMBOL];
                let line_scale_x = cell_width / (line_frame.width / ratio_x) / ratio_x;
                let line_scale_y = (cell_height * 0.08) / (line_frame.height / ratio_y) / ratio_y;
                line_transform.scale(line_scale_x, line_scale_y);

                let line_color = UnifiedColor::new(fg_color.0, fg_color.1, fg_color.2, fg_color.3);
                self.draw_symbol(gl, BG_FILL_SYMBOL, &line_transform, &line_color);
            }
        }
        self.draw(gl);
    }

    // fn make_symbols_frame(&mut self, sheet: &mut GlTexture, x: f32, y: f32) -> GlCell {
    //     let width = *PIXEL_SYM_WIDTH.get().expect("lazylock init");
    //     let height = *PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    //     self.make_symbols_frame_custom(sheet, x, y, width, height, 0.0)
    // }

    /// Create a symbol frame with custom dimensions
    ///
    /// Used for TUI (16x32), Emoji (32x32), and Sprite (16x16) regions.
    /// tex_layer: 0 for Sprite/TUI/Emoji (Layer 0), 1 for CJK (Layer 1)
    fn make_symbols_frame_custom(&mut self, sheet: &mut GlTexture, x: f32, y: f32, width: f32, height: f32, tex_layer: f32) -> GlCell {
        let origin_x = 1.0;
        let origin_y = 1.0;
        let tex_width = sheet.width as f32;
        let tex_height = sheet.height as f32;

        let uv_left = x / tex_width;
        let uv_top = y / tex_height;
        let uv_width = width / tex_width;
        let uv_height = height / tex_height;

        GlCell {
            texture: sheet.texture,
            width,
            height,
            origin_x,
            origin_y,
            uv_left,
            uv_top,
            uv_width,
            uv_height,
            tex_layer,
        }
    }
}
