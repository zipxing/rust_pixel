// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::adapter::{
    gl::{
        render_general2d::GlRenderGeneral2d, render_symbols::GlRenderSymbols,
        render_transition::GlRenderTransition, texture::GlRenderTexture,
        GlRender,
    },
    BlendMode, RenderCell, RtComposite,
};
use crate::render::graph::{UnifiedColor, UnifiedTransform};
use glow::HasContext;
use log::info;

// open gl shader wrapper...
pub struct GlPixel {
    r_sym: GlRenderSymbols,
    r_g2d: GlRenderGeneral2d,
    r_trans: GlRenderTransition,

    render_textures: Vec<GlRenderTexture>,

    pub canvas_width: u32,
    pub canvas_height: u32,

    clear_color: UnifiedColor,
}

impl GlPixel {
    pub fn new(
        gl: &glow::Context,
        ver: &str,
        canvas_width: i32,
        canvas_height: i32,
        texw: i32,
        texh: i32,
        texdata: &[u8],
    ) -> Self {
        // gl render symbols for draw main buffer
        let mut r_sym = GlRenderSymbols::new(canvas_width as u32, canvas_height as u32);
        r_sym.init(gl, ver);
        r_sym.load_texture(gl, texw, texh, texdata);

        // gl render general2d for draw render texture
        let mut r_g2d = GlRenderGeneral2d::new(canvas_width as u32, canvas_height as u32);
        r_g2d.init(gl, ver);

        // gl render transition for transition effect
        let mut r_trans = GlRenderTransition::new(canvas_width as u32, canvas_height as u32);
        r_trans.init(gl, ver);

        unsafe {
            gl.enable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
        }

        // create 4 render texture for gl transition...
        let mut render_textures = vec![];
        let rt_hidden = [true, true, false, true];
        for i in 0..4 {
            let w = canvas_width as u32;
            let h = canvas_height as u32;
            let rt = GlRenderTexture::new(gl, w, h, rt_hidden[i]).unwrap();
            info!("rt...{:?}", rt.texture);
            render_textures.push(rt);
        }

        Self {
            canvas_width: canvas_width as u32,
            canvas_height: canvas_height as u32,
            r_sym,
            r_g2d,
            r_trans,
            render_textures,
            clear_color: UnifiedColor::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    // bind none for render to screen...
    pub fn bind_screen(&mut self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);
        }
    }

    // idx 0 - 3 : render to GlRenderTexture 0 - 3
    pub fn bind_target(&mut self, gl: &glow::Context, render_texture_idx: usize) {
        unsafe {
            let tex = &self.render_textures[render_texture_idx];
            gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                Some(tex.framebuffer),
            );
            // info!("bind_target...{} {} {}", render_texture_idx, tex.width, tex.height);
            gl.viewport(0, 0, tex.width as i32, tex.height as i32);
        }
    }

    pub fn set_clear_color(&mut self, color: UnifiedColor) {
        self.clear_color = color;
    }

    pub fn clear(&mut self, gl: &glow::Context) {
        unsafe {
            gl.clear_color(
                self.clear_color.r,
                self.clear_color.g,
                self.clear_color.b,
                self.clear_color.a,
            );
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn get_render_texture_hidden(&mut self, rtidx: usize) -> bool {
        self.render_textures[rtidx].is_hidden
    }

    pub fn set_render_texture_hidden(&mut self, rtidx: usize, h: bool) {
        self.render_textures[rtidx].is_hidden = h;
    }

    pub fn render_texture_to_screen_impl(
        &mut self,
        gl: &glow::Context,
        rtidx: usize,
        area: [f32; 4],
        transform: &UnifiedTransform,
        color: &UnifiedColor,
    ) {
        self.r_g2d
            .set_texture(gl, self.render_textures[rtidx].texture)
            .set_area(&area)
            .set_transform(transform)
            .set_color(color);
        self.r_g2d.prepare_draw(gl);
        self.r_g2d.draw(gl);
    }

    pub fn render_rbuf(
        &mut self,
        gl: &glow::Context,
        rbuf: &[RenderCell],
        ratio_x: f32,
        ratio_y: f32,
    ) {
        self.r_sym.render_rbuf(gl, rbuf, ratio_x, ratio_y);
    }

    pub fn render_trans_frame(
        &mut self,
        gl: &glow::Context,
        src_idx1: usize,
        src_idx2: usize,
        sidx: usize,
        progress: f32,
    ) {
        self.r_trans.set_texture(
            self.canvas_width,
            self.canvas_height,
            self.render_textures[src_idx1].texture,
            self.render_textures[src_idx2].texture,
        );
        self.r_trans.draw_trans(gl, sidx, progress);
    }

    /// Copy one render texture to another using framebuffer blit
    ///
    /// This is much more efficient than rendering through a shader for static copies.
    /// Uses OpenGL's blit operation to directly copy framebuffer contents.
    pub fn copy_rt(&mut self, gl: &glow::Context, src_index: usize, dst_index: usize) {
        unsafe {
            let src_tex = &self.render_textures[src_index];
            let dst_tex = &self.render_textures[dst_index];

            // Bind source framebuffer to READ_FRAMEBUFFER
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(src_tex.framebuffer));

            // Bind destination framebuffer to DRAW_FRAMEBUFFER
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(dst_tex.framebuffer));

            // Blit (copy) from source to destination
            gl.blit_framebuffer(
                0, 0, src_tex.width as i32, src_tex.height as i32,  // src rect
                0, 0, dst_tex.width as i32, dst_tex.height as i32,  // dst rect
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );

            // Restore normal framebuffer binding
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            // Make destination texture visible, keep source as-is
            self.render_textures[dst_index].is_hidden = false;
        }
    }

    pub fn get_canvas_size(&self) -> (u32, u32) {
        (self.canvas_width, self.canvas_height)
    }
}

/// OpenGL Pixel Renderer with owned context
///
/// This structure owns both the OpenGL context and the GlPixel renderer,
/// providing a unified interface that matches the WGPU model where
/// all rendering resources are owned by the renderer.
pub struct GlPixelRenderer {
    /// OpenGL context handle
    pub gl: glow::Context,
    
    /// OpenGL pixel renderer instance  
    pub gl_pixel: GlPixel,
}

impl GlPixelRenderer {
    /// Create new OpenGL pixel renderer with owned context
    pub fn new(
        gl: glow::Context,
        ver: &str,
        canvas_width: i32,
        canvas_height: i32,
        texw: i32,
        texh: i32,
        texdata: &[u8],
    ) -> Self {
        let gl_pixel = GlPixel::new(
            &gl,
            ver,
            canvas_width,
            canvas_height,
            texw,
            texh,
            texdata,
        );
        
        Self { gl, gl_pixel }
    }
    
    /// Get reference to OpenGL context
    pub fn get_gl(&self) -> &glow::Context {
        &self.gl
    }
    
    /// Get mutable reference to OpenGL context  
    pub fn get_gl_mut(&mut self) -> &mut glow::Context {
        &mut self.gl
    }
    
    /// Get reference to GlPixel
    pub fn get_gl_pixel(&self) -> &GlPixel {
        &self.gl_pixel
    }
    
    /// Get mutable reference to GlPixel
    pub fn get_gl_pixel_mut(&mut self) -> &mut GlPixel {
        &mut self.gl_pixel
    }
}





// Separate impl block for GlPixelRenderer convenience methods
impl GlPixelRenderer {
    /// Render textures to screen without external context (convenience method)
    ///
    /// This method is specific to GlPixelRenderer and doesn't require external context
    /// since it owns the OpenGL context.
    pub fn render_textures_to_screen_self_contained(
        &mut self,
        ratio_x: f32,
        ratio_y: f32,
    ) -> Result<(), String> {
        // Bind to screen framebuffer
        self.gl_pixel.bind_screen(&self.gl);
        
        // Layer 1: Draw render_texture 2 (main game content)
        if !self.gl_pixel.get_render_texture_hidden(2) {
            self.gl_pixel.render_texture_to_screen_impl(
                &self.gl,
                2,
                [0.0, 0.0, 1.0, 1.0], // Full-screen quad
                &crate::render::graph::UnifiedTransform::new(),
                &crate::render::graph::UnifiedColor::white(),
            );
        }

        // Layer 2: Draw render_texture 3 (transition effects and overlays)
        if !self.gl_pixel.get_render_texture_hidden(3) {
            // Calculate proper scaling for high-DPI displays
            let (canvas_width, canvas_height) = self.gl_pixel.get_canvas_size();
            let pcw = canvas_width as f32;
            let pch = canvas_height as f32;

            // Calculate scaled dimensions for transition layer
            let pw = pcw / ratio_x;
            let ph = pch / ratio_y;

            // Create unified transform with proper scaling
            let mut unified_transform = crate::render::graph::UnifiedTransform::new();
            unified_transform.scale(pw / pcw, ph / pch);

            // OpenGL Y-axis: bottom-left origin
            let viewport = [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch];

            self.gl_pixel.render_texture_to_screen_impl(
                &self.gl,
                3,
                viewport,
                &unified_transform,
                &crate::render::graph::UnifiedColor::white(),
            );
        }

        Ok(())
    }
    
    /// Render buffer to texture without external context (convenience method)
    ///
    /// This method is specific to GlPixelRenderer and doesn't require external context
    /// since it owns the OpenGL context.
    pub fn render_buffer_to_texture_self_contained(
        &mut self,
        rbuf: &[crate::render::graph::RenderCell],
        rtidx: usize,
        debug: bool,
        ratio_x: f32,
        ratio_y: f32,
    ) -> Result<(), String> {
        // Set clear color
        let clear_color = if debug {
            crate::render::graph::UnifiedColor::new(1.0, 0.0, 0.0, 1.0) // Red for debug
        } else {
            crate::render::graph::UnifiedColor::black() // Black for normal
        };
        self.gl_pixel.set_clear_color(clear_color);

        // Bind the target render texture
        self.gl_pixel.bind_target(&self.gl, rtidx);
        
        // Clear the target
        self.gl_pixel.clear(&self.gl);
        
        // Render symbols to the bound target
        self.gl_pixel.render_rbuf(&self.gl, rbuf, ratio_x, ratio_y);

        Ok(())
    }

    /// Render GL transition frame with effect and progress (convenience method for petview)
    pub fn render_gl_transition(&mut self, src_tex1: usize, src_tex2: usize, dst_tex: usize, effect: usize, progress: f32) {
        self.gl_pixel.bind_target(&self.gl, dst_tex);
        self.gl_pixel.set_render_texture_hidden(dst_tex, false);
        self.gl_pixel.render_trans_frame(&self.gl, src_tex1, src_tex2, effect, progress);
    }
    
    /// Setup transition buffer rendering (convenience method for petview)
    pub fn setup_transbuf_rendering(&mut self, rtidx: usize) {
        self.gl_pixel.bind_target(&self.gl, rtidx);
        self.gl_pixel.set_render_texture_hidden(rtidx, true);
    }

    /// Copy one render texture to another
    ///
    /// Uses efficient framebuffer blit operation to copy texture contents.
    /// Much faster than rendering with a shader for static display purposes.
    pub fn copy_rt(&mut self, src_index: usize, dst_index: usize) {
        self.gl_pixel.copy_rt(&self.gl, src_index, dst_index);
    }
    
    /// Bind screen and set viewport for Retina displays (convenience method for Winit)
    pub fn bind_screen_with_viewport(&mut self, physical_width: i32, physical_height: i32) {
        self.gl_pixel.bind_screen(&self.gl);
        
        // Set correct viewport for Retina displays
        unsafe {
            use glow::HasContext;
            self.gl.viewport(0, 0, physical_width, physical_height);
        }
    }
    
    // ========================================================================
    // RT API Methods
    // ========================================================================

    /// Set OpenGL blend mode based on BlendMode enum
    fn set_blend_mode(&self, blend: BlendMode) {
        unsafe {
            match blend {
                BlendMode::Normal => {
                    self.gl.blend_func_separate(
                        glow::SRC_ALPHA,
                        glow::ONE_MINUS_SRC_ALPHA,
                        glow::ONE,
                        glow::ONE_MINUS_SRC_ALPHA,
                    );
                }
                BlendMode::Add => {
                    self.gl.blend_func(glow::SRC_ALPHA, glow::ONE);
                }
                BlendMode::Multiply => {
                    self.gl.blend_func(glow::DST_COLOR, glow::ZERO);
                }
                BlendMode::Screen => {
                    self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_COLOR);
                }
            }
        }
    }

    /// 将 RenderTexture(s) 合成输出到屏幕
    ///
    /// # RT 渲染流程图
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────────────────┐
    /// │                        RT Present 渲染流程                               │
    /// ├─────────────────────────────────────────────────────────────────────────┤
    /// │                                                                         │
    /// │  1. RT Texture (canvas_w × canvas_h)     2. 纹理采样 (area)             │
    /// │  ┌────────────────────────────┐          ┌─────────────────┐            │
    /// │  │                            │          │   content_w     │            │
    /// │  │  ┌──────────────┐          │   ───►   │  ┌───────────┐  │            │
    /// │  │  │   Content    │ content_h│   area   │  │  Sampled  │  │ content_h  │
    /// │  │  │   (实际内容)  │          │   采样   │  │   Area    │  │            │
    /// │  │  └──────────────┘          │          │  └───────────┘  │            │
    /// │  │         Empty Space        │          └─────────────────┘            │
    /// │  └────────────────────────────┘                                         │
    /// │                                                                         │
    /// │  3. 变换矩阵 (transform)              4. 最终屏幕输出                    │
    /// │  ┌─────────────────┐                  ┌────────────────────────┐        │
    /// │  │ Scale:          │                  │        Screen          │        │
    /// │  │   vp_w/canvas_w │    ───►          │  ┌────────────────┐    │        │
    /// │  │   vp_h/canvas_h │  Transform       │  │    Viewport    │    │        │
    /// │  │                 │    变换          │  │   (vp_x,vp_y)  │    │        │
    /// │  │ Translate:      │                  │  │   vp_w × vp_h  │    │        │
    /// │  │   tx, ty (NDC)  │                  │  └────────────────┘    │        │
    /// │  └─────────────────┘                  └────────────────────────┘        │
    /// │                                                                         │
    /// └─────────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// # 关键概念
    /// - **area**: 控制从 RT 纹理中采样哪一部分 (UV 坐标, 0.0-1.0)
    /// - **transform**: 控制采样后的内容显示在屏幕的什么位置、多大尺寸
    /// - **content_size**: 原始内容尺寸，用于 area 计算（缩放时保持不变）
    /// - **viewport**: 显示区域尺寸，用于 transform 计算（缩放时会变化）
    ///
    /// # 缩放原理
    /// ```text
    /// scale_uniform(0.5) 的效果:
    ///   content_size = (320, 200)  // 不变，采样整个内容
    ///   viewport = (160, 100)       // 缩小，显示更小
    ///   结果: 采样完整内容，但显示为 50% 大小
    /// ```
    pub fn present(&mut self, composites: &[RtComposite]) {
        // ═══════════════════════════════════════════════════════════════════════
        // Note: Caller must bind screen and set viewport before calling this method.
        // Do NOT call bind_screen() here - it resets viewport to canvas_size,
        // which breaks Retina display support where physical_size != canvas_size.
        // ═══════════════════════════════════════════════════════════════════════

        // 获取画布尺寸 (RT 纹理的实际大小)
        let (canvas_width, canvas_height) = self.gl_pixel.get_canvas_size();
        let pcw = canvas_width as f32;  // pixel canvas width
        let pch = canvas_height as f32; // pixel canvas height

        // ═══════════════════════════════════════════════════════════════════════
        // Step 2: 遍历所有 RtComposite，依次渲染到屏幕
        // ═══════════════════════════════════════════════════════════════════════
        for composite in composites {
            let rtidx = composite.rt;

            // 跳过隐藏的 RT
            if self.gl_pixel.get_render_texture_hidden(rtidx) {
                continue;
            }

            // 设置混合模式 (Normal, Add, Multiply, Screen)
            self.set_blend_mode(composite.blend);

            // ═══════════════════════════════════════════════════════════════════
            // Step 3: 计算 area (纹理采样区域) 和 transform (屏幕变换矩阵)
            // ═══════════════════════════════════════════════════════════════════
            let (area, transform) = if let Some(ref vp) = composite.viewport {
                // ─────────────────────────────────────────────────────────────────
                // 自定义 viewport 模式
                // ─────────────────────────────────────────────────────────────────

                // viewport 参数 (显示位置和大小，可能被 scale() 修改过)
                let vp_x = vp.x as f32;  // 显示位置 X
                let vp_y = vp.y as f32;  // 显示位置 Y
                let vp_w = vp.w as f32;  // 显示宽度 (缩放后)
                let vp_h = vp.h as f32;  // 显示高度 (缩放后)

                // 获取 content_size (原始内容尺寸，用于纹理采样)
                // 这是缩放功能的关键：content_size 保持不变，viewport 变化
                let (content_w, content_h) = composite.content_size
                    .map(|(w, h)| (w as f32, h as f32))
                    .unwrap_or((vp_w, vp_h));

                // ─────────────────────────────────────────────────────────────────
                // 计算 area: 纹理采样区域 [x, y, width, height] (UV 坐标 0.0-1.0)
                // ─────────────────────────────────────────────────────────────────
                // OpenGL 纹理坐标系: (0,0) 在左下角, (1,1) 在右上角
                // 但内容渲染在 RT 的左上角，所以需要 Y 轴翻转
                //
                // area[0] = 0.0                       // 采样起始 X (左边缘)
                // area[1] = (pch - content_h) / pch   // 采样起始 Y (从上往下算)
                // area[2] = content_w / pcw           // 采样宽度
                // area[3] = content_h / pch           // 采样高度
                let area = [0.0, (pch - content_h) / pch, content_w / pcw, content_h / pch];

                // ─────────────────────────────────────────────────────────────────
                // 计算 transform: 屏幕变换矩阵 (决定显示位置和大小)
                // ─────────────────────────────────────────────────────────────────
                let mut base_transform = UnifiedTransform::new();

                // 缩放: 将 [-1,1] 的全屏四边形缩放到 viewport 大小
                // vp_w/pcw 和 vp_h/pch 决定显示区域相对于画布的比例
                base_transform.scale(vp_w / pcw, vp_h / pch);

                // 平移: 将缩放后的四边形移动到正确位置
                // NDC (Normalized Device Coordinates) 坐标系:
                //   中心 (0,0), 范围 [-1,1], Y 轴向上
                // 屏幕坐标系:
                //   左上角 (0,0), Y 轴向下
                // 转换公式:
                //   tx = (2 * vp_x + vp_w - pcw) / pcw  // X 方向偏移
                //   ty = (pch - 2 * vp_y - vp_h) / pch  // Y 方向偏移 (翻转)
                let tx = (2.0 * vp_x + vp_w - pcw) / pcw;
                let ty = (pch - 2.0 * vp_y - vp_h) / pch;
                base_transform.translate(tx, ty);

                // 如果用户指定了额外的变换（旋转等），与基础变换组合
                let final_transform = if let Some(ref user_transform) = composite.transform {
                    base_transform.compose(user_transform)
                } else {
                    base_transform
                };

                (area, final_transform)
            } else {
                // ─────────────────────────────────────────────────────────────────
                // 全屏模式: 采样整个纹理，无变换（或使用用户指定的变换）
                // ─────────────────────────────────────────────────────────────────
                let area = [0.0, 0.0, 1.0, 1.0];  // 采样整个纹理
                let transform = composite.transform.unwrap_or_else(UnifiedTransform::new);
                (area, transform)
            };

            // ═══════════════════════════════════════════════════════════════════
            // Step 4: 执行实际渲染
            // ═══════════════════════════════════════════════════════════════════
            // 设置颜色（包含 alpha 透明度）
            let alpha_f = composite.alpha as f32 / 255.0;
            let color = UnifiedColor::new(1.0, 1.0, 1.0, alpha_f);

            // 调用底层渲染函数，将 RT 纹理绘制到屏幕
            // 内部流程:
            //   1. 绑定 RT 纹理
            //   2. 设置 uniform: area, transform, color
            //   3. 绘制全屏四边形 (2个三角形, 4个顶点)
            //   4. Vertex Shader: 根据 area 计算纹理坐标, 根据 transform 计算顶点位置
            //   5. Fragment Shader: 采样纹理颜色，乘以 color (应用 alpha)
            self.gl_pixel.render_texture_to_screen_impl(
                &self.gl,
                rtidx,
                area,
                &transform,
                &color,
            );
        }

        // 恢复默认混合模式
        self.set_blend_mode(BlendMode::Normal);
    }

    /// Present with default settings and physical size for Retina displays
    pub fn present_default_with_physical_size(
        &mut self,
        ratio_x: f32,
        ratio_y: f32,
        physical_size: Option<(u32, u32)>,
    ) {
        // Bind screen framebuffer
        self.gl_pixel.bind_screen(&self.gl);

        // Set viewport to physical size if provided (for Retina displays)
        if let Some((pw, ph)) = physical_size {
            unsafe {
                self.gl.viewport(0, 0, pw as i32, ph as i32);
            }
        }

        // Note: Don't clear screen here - matches original behavior
        // Clearing would fill the entire viewport with black, which may
        // affect transparent areas on web canvas

        let unified_color = UnifiedColor::white();

        // Layer 1: Draw render_texture 2 (main game content) - fullscreen
        if !self.gl_pixel.get_render_texture_hidden(2) {
            self.gl_pixel.render_texture_to_screen_impl(
                &self.gl,
                2,
                [0.0, 0.0, 1.0, 1.0],
                &UnifiedTransform::new(),
                &unified_color,
            );
        }

        // Layer 2: Draw render_texture 3 (transition effects and overlays)
        if !self.gl_pixel.get_render_texture_hidden(3) {
            let (canvas_width, canvas_height) = self.gl_pixel.get_canvas_size();
            let pcw = canvas_width as f32;
            let pch = canvas_height as f32;

            let pw = pcw / ratio_x;
            let ph = pch / ratio_y;

            let mut unified_transform = UnifiedTransform::new();
            unified_transform.scale(pw / pcw, ph / pch);

            let viewport = [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch];

            self.gl_pixel.render_texture_to_screen_impl(
                &self.gl,
                3,
                viewport,
                &unified_transform,
                &unified_color,
            );
        }
    }
}
