//! # Graphics Rendering Core Module
//!
//! This module contains the core data structures, constants and functions for
//! RustPixel's graphics rendering system. After the WGPU refactoring, this module
//! plays a more important role by providing unified data structures across backends.
//!
//! ## 🏗️ Module Responsibilities
//!
//! ### Core Data Structures
//! - **UnifiedColor**: Cross-backend color representation supporting RGBA float format
//! - **UnifiedTransform**: Unified 2D transformation matrix for sprite and texture transforms
//! - **RenderCell**: GPU-ready rendering unit data
//!
//! ### Texture and Symbol Management
//! - **PIXEL_TEXTURE_FILE**: Symbol texture file path constant
//! - **PIXEL_SYM_WIDTH/HEIGHT**: Global configuration for symbol dimensions
//! - Texture coordinate calculation and symbol index conversion
//!
//! ### Rendering Pipeline Abstraction
//! - **draw_all_graph()**: Unified graphics rendering entry point
//! - Buffer to RenderCell conversion logic
//! - Sprite rendering and Logo animation support
//!
//! ## 🚀 Design Benefits
//!
//! ### Cross-Backend Compatibility
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    graph.rs (This Module)                   │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │           Unified Data Structures                      │ │
//! │  │  ┌─────────────┬─────────────┬───────────────────────┐ │ │
//! │  │  │UnifiedColor │UnifiedTrans-│      RenderCell       │ │ │
//! │  │  │(RGBA)       │form (2D)    │   (GPU-ready)         │ │ │
//! │  │  └─────────────┴─────────────┴───────────────────────┘ │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! │                           │                                 │
//! │                           ▼                                 │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │              Backend Adapters                          │ │
//! │  │  ┌────────┬─────────┬─────────┬─────────┬────────────┐ │ │
//! │  │  │  SDL   │  Winit  │  Winit  │   Web   │  Crossterm │ │ │
//! │  │  │  +GL   │   +GL   │  +WGPU  │  +WebGL │    (Text)  │ │ │
//! │  │  └────────┴─────────┴─────────┴─────────┴────────────┘ │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Zero-Cost Abstractions
//! - **Compile-time specialization**: Each backend can optimize to best performance
//! - **Direct memory mapping**: RenderCell directly corresponds to GPU buffer format
//! - **No virtual function overhead**: Performance improvements after removing trait objects
//!
//! ## 📊 Symbol Texture System
//!
//! RustPixel uses a unified symbol texture to render characters and graphic elements.
//!
//! ### Character Types and Heights
//! - **Sprite Characters**: 16x16 pixels - for pixel art and sprites
//! - **TUI Characters**: 16x32 pixels - for UI components and text (double height)
//! - **Emoji**: 32x32 pixels - for emoji rendering
//!
//! ### Window Decoration
//! RustPixel now uses **OS native window decoration** (title bar, borders) instead of
//! custom-drawn borders. This provides better desktop integration and reduces rendering overhead.
//!
//! ### TUI Height Mode
//! Applications can enable TUI height mode for UI-focused rendering:
//! ```rust
//! ctx.adapter.get_base().gr.set_use_tui_height(true);
//! ```
//! This affects:
//! - Window height calculation (uses 32px per row instead of 16px)
//! - Mouse coordinate conversion (accounts for double-height characters)

use crate::{
    render::{buffer::Buffer, cell::tui_symidx, sprite::Layer, style::Color, symbol_map::calc_linear_index, AdapterBase},
    util::{ARect, PointF32, PointI32, PointU16, Rand},
    LOGO_FRAME,
};
// use log::info;
use std::sync::OnceLock;

// ============================================================================
// RenderTexture API - Unified RT management for graphics mode
// ============================================================================

/// RT size strategy
#[cfg(graphics_mode)]
#[derive(Clone, Debug)]
pub enum RtSize {
    /// Follow window size (default)
    FollowWindow,
    /// Fixed size in pixels
    Fixed(u32, u32),
}

#[cfg(graphics_mode)]
impl Default for RtSize {
    fn default() -> Self {
        RtSize::FollowWindow
    }
}

/// RT configuration
#[cfg(graphics_mode)]
#[derive(Clone, Debug)]
pub struct RtConfig {
    /// Size strategy
    pub size: RtSize,
}

#[cfg(graphics_mode)]
impl Default for RtConfig {
    fn default() -> Self {
        Self {
            size: RtSize::FollowWindow,
        }
    }
}

/// Blend mode for RT composition
#[cfg(graphics_mode)]
#[derive(Clone, Copy, Debug, Default)]
pub enum BlendMode {
    /// Normal alpha blending (default)
    #[default]
    Normal,
    /// Additive blending
    Add,
    /// Multiply blending
    Multiply,
    /// Screen blending
    Screen,
}

/// RT 合成项，用于 present() 函数
///
/// # 渲染流程
/// ```text
/// ┌────────────────────────────────────────────────────────────────────┐
/// │  RtComposite 控制 RT 纹理如何显示到屏幕                             │
/// ├────────────────────────────────────────────────────────────────────┤
/// │                                                                    │
/// │  content_size (纹理采样)          viewport (屏幕显示)              │
/// │  ┌───────────────────┐            ┌───────────────────┐           │
/// │  │ 原始内容尺寸       │    ───►   │ 显示位置和大小     │           │
/// │  │ 决定从 RT 采样多少 │            │ 决定显示在屏幕哪里 │           │
/// │  │ (缩放时不变)       │            │ (缩放时会变化)     │           │
/// │  └───────────────────┘            └───────────────────┘           │
/// │                                                                    │
/// │  scale_uniform(0.5) 的效果:                                        │
/// │    content_size = (320, 200)  // 保持不变，采样完整内容             │
/// │    viewport = (160, 100)       // 缩小一半，显示更小                │
/// │                                                                    │
/// └────────────────────────────────────────────────────────────────────┘
/// ```
///
/// # 使用示例
/// ```ignore
/// // 全屏显示 RT2
/// RtComposite::fullscreen(2)
///
/// // 居中显示 RT3，带 50% 缩放
/// ctx.centered_rt(3, 40, 25).scale_uniform(0.5)
///
/// // 自定义位置和透明度
/// RtComposite::at_position(3, 100, 100, 320, 200).alpha(128)
/// ```
///
/// Note: Uses ARect instead of Rect because Rect has automatic clipping
/// when width*height > u16::MAX, which is inappropriate for viewport dimensions.
#[cfg(graphics_mode)]
#[derive(Clone, Debug)]
pub struct RtComposite {
    /// RT 纹理索引 (0-3)
    /// - RT0, RT1: 通常用于 transition 效果的源纹理
    /// - RT2: 主渲染内容 (Scene 渲染目标)
    /// - RT3: 叠加层/特效层
    pub rt: usize,

    /// 显示区域 (屏幕坐标)
    /// - None = 全屏显示
    /// - Some(rect) = 自定义位置和大小
    /// 调用 scale() 后此值会变化
    pub viewport: Option<ARect>,

    /// 原始内容尺寸 (像素)
    /// 用于纹理采样计算，决定从 RT 中采样多少区域
    /// 调用 scale() 后此值保持不变，这是实现真正缩放的关键
    pub content_size: Option<(u32, u32)>,

    /// 混合模式 (Normal, Add, Multiply, Screen)
    pub blend: BlendMode,

    /// 透明度 (0-255, 255=完全不透明)
    pub alpha: u8,

    /// 额外变换 (旋转等)
    /// 会与基础变换（viewport 位置/大小）组合
    pub transform: Option<UnifiedTransform>,
}

#[cfg(graphics_mode)]
impl RtComposite {
    /// Create a fullscreen composite with default settings
    pub fn fullscreen(rt: usize) -> Self {
        Self {
            rt,
            viewport: None,
            content_size: None,
            blend: BlendMode::Normal,
            alpha: 255,
            transform: None,
        }
    }

    /// Create a composite with custom viewport
    /// Uses ARect to avoid Rect's automatic clipping for large viewports
    pub fn with_viewport(rt: usize, viewport: ARect) -> Self {
        // Store original size for texture sampling
        let content_size = Some((viewport.w, viewport.h));
        Self {
            rt,
            viewport: Some(viewport),
            content_size,
            blend: BlendMode::Normal,
            alpha: 255,
            transform: None,
        }
    }

    /// Set blend mode
    pub fn blend(mut self, blend: BlendMode) -> Self {
        self.blend = blend;
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: u8) -> Self {
        self.alpha = alpha;
        self
    }

    /// Offset viewport position by (dx, dy) pixels
    ///
    /// Adjusts the viewport position relative to its current location.
    /// Useful for fine-tuning after using `centered_*` methods.
    ///
    /// # Example
    /// ```ignore
    /// let rt3 = ctx.centered_rt(3, 40, 25).offset(-10, 0);  // shift left 10px
    /// ```
    pub fn offset(mut self, dx: i32, dy: i32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            vp.x += dx;
            vp.y += dy;
        }
        self
    }

    /// Set viewport x position directly
    pub fn x(mut self, x: i32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            vp.x = x;
        }
        self
    }

    /// Set viewport y position directly
    pub fn y(mut self, y: i32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            vp.y = y;
        }
        self
    }

    /// Scale rendering by factors (viewport-based scaling)
    ///
    /// This method scales the viewport size while maintaining its center position.
    /// The texture will be scaled to fill the new viewport size (true scaling, not clipping).
    ///
    /// # Parameters
    /// - `scale_x`: Horizontal scale factor (e.g., 0.5 = half size, 2.0 = double size)
    /// - `scale_y`: Vertical scale factor
    ///
    /// # Example
    /// ```ignore
    /// let rt3 = ctx.centered_rt(3, 40, 25).scale(2.0, 2.0);  // 2x larger, stays centered
    /// ```
    pub fn scale(mut self, scale_x: f32, scale_y: f32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            let old_w = vp.w;
            let old_h = vp.h;

            vp.w = (vp.w as f32 * scale_x) as u32;
            vp.h = (vp.h as f32 * scale_y) as u32;

            // Keep center position unchanged
            vp.x += (old_w as i32 - vp.w as i32) / 2;
            vp.y += (old_h as i32 - vp.h as i32) / 2;
        }

        self
    }

    /// Scale uniformly (same scale for both axes)
    ///
    /// # Parameters
    /// - `scale`: Scale factor applied to both width and height
    ///
    /// # Example
    /// ```ignore
    /// let rt3 = ctx.centered_rt(3, 40, 25).scale_uniform(1.5);  // 150% size
    /// ```
    pub fn scale_uniform(self, scale: f32) -> Self {
        self.scale(scale, scale)
    }

    /// Rotate the rendering (angle in degrees)
    ///
    /// # Parameters
    /// - `degrees`: Rotation angle in degrees (positive = counter-clockwise)
    ///
    /// # Example
    /// ```ignore
    /// let rt3 = ctx.centered_rt(3, 40, 25).rotate(45.0);  // 45° rotation
    /// ```
    pub fn rotate(mut self, degrees: f32) -> Self {
        let mut transform = self.transform.unwrap_or_else(UnifiedTransform::new);
        transform.rotate(degrees.to_radians());
        self.transform = Some(transform);
        self
    }

    /// Translate the rendering (offset position)
    ///
    /// Note: This is different from offset() which moves the viewport.
    /// translate() applies GPU-side transformation.
    ///
    /// # Parameters
    /// - `dx`: Horizontal offset in pixels
    /// - `dy`: Vertical offset in pixels
    pub fn translate(mut self, dx: f32, dy: f32) -> Self {
        let mut transform = self.transform.unwrap_or_else(UnifiedTransform::new);
        transform.translate(dx, dy);
        self.transform = Some(transform);
        self
    }

    /// Set viewport width directly
    ///
    /// Note: This changes width but keeps the x position, so the viewport
    /// will expand/shrink from the left edge. Use with `offset()` or after
    /// `centered_rt()` if you need to maintain centering.
    pub fn width(mut self, w: u32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            vp.w = w;
        }
        self
    }

    /// Set viewport height directly
    ///
    /// Note: This changes height but keeps the y position, so the viewport
    /// will expand/shrink from the top edge. Use with `offset()` or after
    /// `centered_rt()` if you need to maintain centering.
    pub fn height(mut self, h: u32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            vp.h = h;
        }
        self
    }

    /// Set viewport size directly
    ///
    /// Note: This changes size but keeps position unchanged. Use with
    /// `offset()` or after `centered_rt()` if you need to maintain centering.
    pub fn size(mut self, w: u32, h: u32) -> Self {
        if let Some(ref mut vp) = self.viewport {
            vp.w = w;
            vp.h = h;
        }
        self
    }

    // ========================================================================
    // Viewport Helper Methods
    // ========================================================================
    //
    // Viewport positioning involves converting between coordinate systems:
    //
    // ┌─────────────────────────────────────────────────────────────────────┐
    // │  Cell Coordinates (40x25)                                           │
    // │       │                                                             │
    // │       │ cells_to_pixel_size()                                       │
    // │       ▼                                                             │
    // │  Pixel Dimensions (320x200 @ ratio 1.0)                             │
    // │       │                                                             │
    // │       │ centered() / at_position()                                  │
    // │       ▼                                                             │
    // │  Canvas Position (centered on screen)                               │
    // └─────────────────────────────────────────────────────────────────────┘

    /// Convert cell dimensions to pixel dimensions
    ///
    /// Transforms logical cell sizes to pixel sizes, accounting for
    /// symbol size and DPI scaling ratio.
    ///
    /// # Parameters
    /// - `cell_w`, `cell_h`: Size in cells (e.g., 40x25)
    /// - `sym_w`, `sym_h`: Symbol pixel size (from PIXEL_SYM_WIDTH/HEIGHT)
    /// - `rx`, `ry`: DPI scaling ratio (from adapter.get_base().gr.ratio_x/y)
    ///
    /// # Returns
    /// (pixel_width, pixel_height) as u32 tuple
    ///
    /// # Example
    /// ```ignore
    /// let sym_w = *PIXEL_SYM_WIDTH.get().unwrap();
    /// let sym_h = *PIXEL_SYM_HEIGHT.get().unwrap();
    /// let rx = ctx.adapter.get_base().gr.ratio_x;
    /// let ry = ctx.adapter.get_base().gr.ratio_y;
    /// let (pw, ph) = RtComposite::cells_to_pixel_size(40, 25, sym_w, sym_h, rx, ry);
    /// ```
    pub fn cells_to_pixel_size(
        cell_w: u16,
        cell_h: u16,
        sym_w: f32,
        sym_h: f32,
        rx: f32,
        ry: f32,
    ) -> (u32, u32) {
        let pw = (cell_w as f32 * sym_w / rx) as u32;
        let ph = (cell_h as f32 * sym_h / ry) as u32;
        (pw, ph)
    }

    /// Create a centered viewport composite
    ///
    /// Calculates the position to center a viewport of given size
    /// within the canvas.
    ///
    /// # Parameters
    /// - `rt`: Render texture index (0-3)
    /// - `vp_w`, `vp_h`: Viewport size in pixels
    /// - `canvas_w`, `canvas_h`: Canvas size in pixels
    ///
    /// # Example
    /// ```ignore
    /// let canvas_w = ctx.adapter.get_base().gr.pixel_w as u32;
    /// let canvas_h = ctx.adapter.get_base().gr.pixel_h as u32;
    /// ctx.adapter.present(&[
    ///     RtComposite::fullscreen(2),
    ///     RtComposite::centered(3, 320, 200, canvas_w, canvas_h),
    /// ]);
    /// ```
    pub fn centered(rt: usize, vp_w: u32, vp_h: u32, canvas_w: u32, canvas_h: u32) -> Self {
        let x = ((canvas_w.saturating_sub(vp_w)) / 2) as i32;
        let y = ((canvas_h.saturating_sub(vp_h)) / 2) as i32;
        Self {
            rt,
            viewport: Some(ARect { x, y, w: vp_w, h: vp_h }),
            content_size: Some((vp_w, vp_h)),
            blend: BlendMode::Normal,
            alpha: 255,
            transform: None,
        }
    }

    /// Create a viewport at a specific position
    ///
    /// Places the viewport at the given canvas coordinates.
    ///
    /// # Parameters
    /// - `rt`: Render texture index (0-3)
    /// - `x`, `y`: Position in canvas pixels
    /// - `w`, `h`: Viewport size in pixels
    pub fn at_position(rt: usize, x: i32, y: i32, w: u32, h: u32) -> Self {
        Self {
            rt,
            viewport: Some(ARect { x, y, w, h }),
            content_size: Some((w, h)),
            blend: BlendMode::Normal,
            alpha: 255,
            transform: None,
        }
    }

    /// Create a centered viewport from cell dimensions (all-in-one helper)
    ///
    /// This is the highest-level API that handles all coordinate conversions
    /// automatically. Just provide the cell dimensions and render context info.
    ///
    /// # Parameters
    /// - `rt`: Render texture index (0-3)
    /// - `cell_w`, `cell_h`: Size in cells (e.g., 40x25)
    /// - `sym_w`, `sym_h`: Symbol pixel size (from PIXEL_SYM_WIDTH/HEIGHT)
    /// - `rx`, `ry`: DPI scaling ratio
    /// - `canvas_w`, `canvas_h`: Canvas size in pixels
    ///
    /// # Example
    /// ```ignore
    /// // In render draw():
    /// let sym_w = *PIXEL_SYM_WIDTH.get().unwrap();
    /// let sym_h = *PIXEL_SYM_HEIGHT.get().unwrap();
    /// let rx = ctx.adapter.get_base().gr.ratio_x;
    /// let ry = ctx.adapter.get_base().gr.ratio_y;
    /// let canvas_w = ctx.adapter.get_base().gr.pixel_w as u32;
    /// let canvas_h = ctx.adapter.get_base().gr.pixel_h as u32;
    ///
    /// ctx.adapter.present(&[
    ///     RtComposite::fullscreen(2),
    ///     RtComposite::centered_cells(3, 40, 25, sym_w, sym_h, rx, ry, canvas_w, canvas_h),
    /// ]);
    /// ```
    pub fn centered_cells(
        rt: usize,
        cell_w: u16,
        cell_h: u16,
        sym_w: f32,
        sym_h: f32,
        rx: f32,
        ry: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) -> Self {
        let (vp_w, vp_h) = Self::cells_to_pixel_size(cell_w, cell_h, sym_w, sym_h, rx, ry);
        Self::centered(rt, vp_w, vp_h, canvas_w, canvas_h)
    }
}

/// Symbol texture file path
///
/// The symbol texture is 4096×4096 pixels, organized as a 256×256 grid
/// (16 pixels per cell). Contains four regions:
///
/// Layout (4096×4096, 10 Sprite Rows):
/// ```text
/// ┌────────────────────────────────────────────────────┐
/// │ Sprite Region (rows 0-159): 40,960 sprites 16×16  │ 2560px
/// ├────────────────────────────────────────────────────┤
/// │ TUI Region (rows 160-191, cols 0-159): 2,560 TUI  │
/// │ Emoji Region (rows 160-191, cols 160-255): 768    │ 512px
/// ├────────────────────────────────────────────────────┤
/// │ CJK Region (rows 192-255): 4,096 chars 32×32      │ 1024px
/// └────────────────────────────────────────────────────┘
/// ```
///
/// Block assignments:
/// - Sprite: Block 0-159 (16×16 chars/block, 16×16px each)
/// - TUI: Block 160-169 (16×16 chars/block, 16×32px each)
/// - Emoji: Block 170-175 (8×16 chars/block, 32×32px each)
/// - CJK: Grid-based (128×32 chars, 32×32px each)
pub const PIXEL_TEXTURE_FILE: &str = "assets/pix/symbols.png";

/// Symbol width (in pixels) resolved from the symbol atlas (16 pixels)
///
/// Initialized exactly once during adapter initialization. Accessing this
/// before initialization will panic with "lazylock init".
///
/// Note: Both Sprite and TUI layers use the same width (16 pixels).
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();

/// Symbol height (in pixels) resolved from the symbol atlas (16 pixels for Sprite)
/// Note:
/// - Sprite layer: uses this value directly (16 pixels)
/// - TUI layer: uses double this value (32 pixels = PIXEL_SYM_HEIGHT * 2)
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();

/// X-axis DPI scaling ratio for coordinate conversion
///
/// Used to convert cell coordinates to pixel coordinates in graphics mode.
/// Typically set during adapter initialization.
/// Default value is 1.0 if not explicitly set.
pub static PIXEL_RATIO_X: OnceLock<f32> = OnceLock::new();

/// Y-axis DPI scaling ratio for coordinate conversion
///
/// Used to convert cell coordinates to pixel coordinates in graphics mode.
/// Typically set during adapter initialization.
/// Default value is 1.0 if not explicitly set.
pub static PIXEL_RATIO_Y: OnceLock<f32> = OnceLock::new();

/// Get X-axis ratio with default value of 1.0 if not set
pub fn get_ratio_x() -> f32 {
    *PIXEL_RATIO_X.get().unwrap_or(&1.0)
}

/// Get Y-axis ratio with default value of 1.0 if not set
pub fn get_ratio_y() -> f32 {
    *PIXEL_RATIO_Y.get().unwrap_or(&1.0)
}

/// Calculate the width of a single symbol (in pixels) based on the full texture width
///
/// # Parameters
/// - `width`: Total texture width
///
/// # Returns
/// Width of a single symbol
///
/// Calculates the width of a single 16x16 sprite cell based on texture dimensions.
/// The texture is organized as a 256x256 grid (256 columns × 256 rows).
/// For a 4096x4096 texture: 4096 / 256 = 16 pixels per symbol.
pub fn init_sym_width(width: u32) -> f32 {
    const TEXTURE_GRID_SIZE: f32 = 256.0;
    width as f32 / TEXTURE_GRID_SIZE
}

/// Calculate the height of a single symbol (in pixels) based on the full texture height
///
/// # Parameters
/// - `height`: Total texture height
///
/// # Returns
/// Height of a single symbol
///
/// Calculates the height of a single 16x16 sprite cell based on texture dimensions.
/// The texture is organized as a 256x256 grid (256 columns × 256 rows).
/// For a 4096x4096 texture: 4096 / 256 = 16 pixels per symbol.
pub fn init_sym_height(height: u32) -> f32 {
    const TEXTURE_GRID_SIZE: f32 = 256.0;
    height as f32 / TEXTURE_GRID_SIZE
}

/// Logo display width (in characters)
pub const PIXEL_LOGO_WIDTH: usize = 27;

/// Logo display height (in characters)
///
/// The logo is displayed during startup to show the project identity.
/// Uses RGB format storage with 3 bytes per pixel.
pub const PIXEL_LOGO_HEIGHT: usize = 12;

/// RustPixel Logo data
///
/// Predefined logo image data in RGB format, 3 bytes per pixel.
/// Displayed during game startup stage to provide brand identification.
///
/// Data format: [R, G, B, R, G, B, ...]
/// Dimensions: 27 × 12 pixels
pub const PIXEL_LOGO: [u8; PIXEL_LOGO_WIDTH * PIXEL_LOGO_HEIGHT * 3] = [
    32, 15, 1, 32, 202, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 239, 1, 32, 15, 1, 100, 239, 1, 32,
    239, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 165, 1, 32,
    165, 0, 32, 87, 1, 32, 15, 1, 18, 202, 1, 21, 202, 1, 19, 202, 1, 20, 202, 1, 32, 15, 1, 47,
    239, 1, 47, 239, 1, 116, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 87, 1, 32, 165, 0, 32, 165, 1, 32, 240, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 100, 239, 1, 81, 49, 1, 47, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15,
    1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15, 0, 32, 87, 1, 32, 15, 0, 32, 165, 0,
    47, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 238, 1, 47,
    238, 1, 47, 238, 1, 47, 239, 1, 100, 239, 1, 46, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1,
    104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1, 84,
    239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160,
    49, 1, 81, 49, 1, 32, 15, 1, 160, 86, 1, 32, 15, 1, 160, 49, 1, 47, 236, 1, 47, 236, 1, 46,
    234, 1, 160, 49, 1, 47, 239, 1, 81, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 47,
    239, 1, 160, 49, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 87, 1, 160, 45,
    1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 160, 45, 1, 32, 15, 1, 160, 45, 1, 32, 235, 1, 116, 235, 1,
    160, 45, 1, 47, 236, 1, 160, 45, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 46, 234, 1, 32, 15, 1,
    46, 234, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 1,
    32, 15, 0, 32, 197, 1, 160, 147, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 160, 147, 1, 32, 15,
    1, 160, 147, 1, 32, 235, 1, 116, 235, 1, 46, 235, 1, 81, 147, 1, 47, 239, 1, 47, 239, 1, 100,
    239, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 47, 239, 1, 32, 15, 1, 160, 147, 1,
    32, 239, 1, 84, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 32, 239, 1, 160, 147, 1, 47,
    239, 1, 104, 239, 1, 104, 240, 1, 160, 147, 1, 32, 15, 1, 160, 147, 1, 32, 15, 1, 116, 235, 1,
    160, 147, 1, 47, 239, 1, 160, 147, 1, 47, 239, 1, 47, 239, 1, 160, 147, 1, 104, 238, 1, 104,
    238, 1, 104, 238, 1, 104, 238, 1, 47, 242, 1, 160, 147, 1, 47, 239, 1, 104, 239, 1, 104, 239,
    1, 104, 239, 1, 47, 239, 1, 84, 239, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 81,
    214, 1, 47, 239, 1, 81, 214, 1, 47, 239, 1, 160, 214, 1, 47, 239, 1, 32, 0, 1, 46, 235, 1, 160,
    214, 1, 47, 236, 1, 81, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 47, 242, 1,
    81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 47, 239, 1, 32, 165, 1, 160, 214,
    1, 103, 239, 1, 32, 242, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 0, 1,
    32, 0, 1, 32, 87, 1, 32, 87, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 165, 0, 32,
    165, 0, 160, 214, 1, 103, 239, 1, 32, 242, 1, 32, 97, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97,
    0, 32, 165, 0, 32, 15, 1, 90, 214, 1, 47, 239, 1, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32,
    0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32,
    15, 1, 32, 15, 1, 32, 15, 1,
];

/// 🎨 Unified Color Representation
///
/// This struct provides cross-backend color abstraction, one of the core data structures
/// after the WGPU refactoring. Supports color representation and conversion for all
/// graphics backends (OpenGL, WGPU, WebGL).
///
/// ## 🔄 Cross-Backend Compatibility
///
/// ```text
/// UnifiedColor (RGBA f32)
///      │
///      ├─→ OpenGL: glColor4f(r, g, b, a)
///      ├─→ WGPU: wgpu::Color { r, g, b, a }
///      ├─→ WebGL: gl.uniform4f(location, r, g, b, a)
///      └─→ Crossterm: Color::Rgb { r: u8, g: u8, b: u8 }
/// ```
///
/// ## 🚀 Performance Features
/// - **Compile-time optimization**: Zero-cost abstraction, fully inlinable by compiler
/// - **Cache-friendly**: Compact memory layout (16 bytes)
/// - **SIMD compatible**: 4 f32 aligned, suitable for vectorization
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnifiedColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl UnifiedColor {
    /// Create a new color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create white color
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    /// Create black color
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Convert to array format
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// 🔄 Unified 2D Transformation Matrix
///
/// This struct provides cross-backend 2D transformation abstraction, supporting
/// translation, scaling, rotation and other operations. After the WGPU refactoring,
/// it became the unified transformation representation for all graphics backends.
///
/// ## 📐 Matrix Layout
///
/// ```text
/// │m00  m01  m20│   │sx   0   tx│   Translation: (tx, ty)
/// │m10  m11  m21│ = │0   sy   ty│   Scale:       (sx, sy)  
/// │ 0    0    1 │   │0    0    1│   Rotation:    cos/sin in m00,m01,m10,m11
/// ```
///
/// ## 🔄 Backend Conversion
///
/// ```text
/// UnifiedTransform (2D Matrix)
///      │
///      ├─→ OpenGL: glUniformMatrix3fv(uniform, matrix)
///      ├─→ WGPU: bytemuck::cast_slice(&transform.to_array())
///      ├─→ WebGL: gl.uniformMatrix3fv(location, false, matrix)
///      └─→ Sprites: Apply to position/scale directly
/// ```
///
/// ## ⚡ Use Cases
/// - **Sprite transformation**: Position, scaling, rotation animations
/// - **UI layout**: Relative positioning of panels and controls
/// - **Effect rendering**: Particle systems and transition effects
/// - **Camera**: View transformation and projection matrices
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnifiedTransform {
    pub m00: f32,
    pub m01: f32,
    pub m10: f32,
    pub m11: f32,
    pub m20: f32,
    pub m21: f32,
}

impl UnifiedTransform {
    /// Create identity transform
    pub fn new() -> Self {
        Self {
            m00: 1.0,
            m01: 0.0,
            m10: 0.0,
            m11: 1.0,
            m20: 0.0,
            m21: 0.0,
        }
    }

    /// Create transform with specific values  
    /// Parameters are in same order as field definition: m00, m01, m10, m11, m20, m21
    pub fn new_with_values(m00: f32, m01: f32, m10: f32, m11: f32, m20: f32, m21: f32) -> Self {
        Self {
            m00,
            m01,
            m10,
            m11,
            m20,
            m21,
        }
    }

    /// Apply scaling transformation
    pub fn scale(&mut self, x: f32, y: f32) {
        // Correct scaling (matches WGPU behavior)
        self.m00 *= x;
        self.m10 *= y;
        self.m01 *= x;
        self.m11 *= y;
    }

    /// Apply translation transformation
    pub fn translate(&mut self, x: f32, y: f32) {
        // Correct matrix multiplication for translation (matches WGPU behavior)
        self.m20 += self.m00 * x + self.m10 * y;
        self.m21 += self.m01 * x + self.m11 * y;
    }

    /// Apply rotation (angle in radians)
    pub fn rotate(&mut self, angle: f32) {
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let m00 = self.m00;
        let m01 = self.m01;
        let m10 = self.m10;
        let m11 = self.m11;

        // Match WGPU's working rotation calculation:
        self.m00 = m00 * cos_a - m10 * sin_a;
        self.m10 = m00 * sin_a + m10 * cos_a;
        self.m01 = m01 * cos_a - m11 * sin_a;
        self.m11 = m01 * sin_a + m11 * cos_a;
    }

    /// Apply horizontal skew/shear transformation (for ITALIC effect)
    /// 
    /// This transforms coordinates as: x' = x + y * shear_x
    /// A positive shear_x value slants the top of the character to the right.
    /// Typical italic angle is about 12-15 degrees, which corresponds to
    /// a shear factor of approximately 0.2-0.27 (tan(12°) ≈ 0.21)
    /// 
    /// # Parameters
    /// - `shear_x`: The horizontal shear factor (tan of the slant angle)
    pub fn skew_x(&mut self, shear_x: f32) {
        // Skew matrix: [1, shear_x; 0, 1]
        // Multiplying current matrix M by skew matrix S:
        // [m00, m10]   [1, shear_x]   [m00, m00*shear_x + m10]
        // [m01, m11] * [0, 1      ] = [m01, m01*shear_x + m11]
        self.m10 += self.m00 * shear_x;
        self.m11 += self.m01 * shear_x;
    }

    /// Reset to identity matrix
    pub fn identity(&mut self) {
        self.m00 = 1.0;
        self.m01 = 0.0;
        self.m10 = 0.0;
        self.m11 = 1.0;
        self.m20 = 0.0;
        self.m21 = 0.0;
    }

    /// Set from another transform
    pub fn set(&mut self, other: &UnifiedTransform) {
        *self = *other;
    }

    /// Create a copy of this transform
    pub fn copy(&self) -> Self {
        *self
    }

    /// Compose (multiply) this transform with another
    ///
    /// Returns a new transform that is the result of applying `self` first, then `other`.
    /// This is matrix multiplication: result = other * self
    ///
    /// # Parameters
    /// - `other`: The transform to apply after this one
    ///
    /// # Returns
    /// A new composed transform
    pub fn compose(&self, other: &UnifiedTransform) -> Self {
        // Matrix multiplication of 2D affine transforms
        // [a c tx]   [a' c' tx']   [a*a'+c*b'  a*c'+c*d'  a*tx'+c*ty'+tx]
        // [b d ty] * [b' d' ty'] = [b*a'+d*b'  b*c'+d*d'  b*tx'+d*ty'+ty]
        // [0 0  1]   [0  0   1]    [0          0          1              ]

        Self {
            m00: self.m00 * other.m00 + self.m10 * other.m01,
            m01: self.m01 * other.m00 + self.m11 * other.m01,
            m10: self.m00 * other.m10 + self.m10 * other.m11,
            m11: self.m01 * other.m10 + self.m11 * other.m11,
            m20: self.m00 * other.m20 + self.m10 * other.m21 + self.m20,
            m21: self.m01 * other.m20 + self.m11 * other.m21 + self.m21,
        }
    }

    /// Multiply with another transform
    pub fn multiply(&mut self, other: &UnifiedTransform) {
        let new_m00 = self.m00 * other.m00 + self.m01 * other.m10;
        let new_m01 = self.m00 * other.m01 + self.m01 * other.m11;
        let new_m10 = self.m10 * other.m00 + self.m11 * other.m10;
        let new_m11 = self.m10 * other.m01 + self.m11 * other.m11;
        let new_m20 = self.m20 * other.m00 + self.m21 * other.m10 + other.m20;
        let new_m21 = self.m20 * other.m01 + self.m21 * other.m11 + other.m21;

        self.m00 = new_m00;
        self.m01 = new_m01;
        self.m10 = new_m10;
        self.m11 = new_m11;
        self.m20 = new_m20;
        self.m21 = new_m21;
    }

    /// Convert to 4x4 matrix for GPU uniforms (column-major order)
    pub fn to_matrix4(&self) -> [[f32; 4]; 4] {
        [
            [self.m00, self.m01, 0.0, 0.0],
            [self.m10, self.m11, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [self.m20, self.m21, 0.0, 1.0],
        ]
    }
}

impl Default for UnifiedTransform {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU rendering unit structure
///
/// RenderCell serves as the intermediate data format between game buffers and
/// the GPU rendering pipeline. This design provides the following advantages:
///
/// ## Design Benefits
/// - **GPU optimization**: Data pre-formatted for efficient GPU upload
/// - **Batch processing**: Multiple units can be rendered in single draw calls
/// - **Flexible rendering**: Supports rotation, scaling and complex effects
/// - **Memory efficient**: Compact representation for large scenes
///
/// ## Rendering Pipeline Integration
/// ```text
/// ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
/// │   Buffer    │───►│ RenderCell  │───►│ OpenGL/GPU  │
/// │(Characters) │    │   Array     │    │  Rendering  │
/// └─────────────┘    └─────────────┘    └─────────────┘
/// ```
///
/// Each RenderCell contains all information needed to render a character or sprite,
/// including color, position, rotation and texture coordinates.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderCell {
    /// Foreground color RGBA components (0.0-1.0 range)
    ///
    /// Used for character/symbol rendering. Alpha component controls
    /// transparency and blending operations.
    pub fcolor: (f32, f32, f32, f32),

    /// Optional background color RGBA components
    ///
    /// When present, renders a colored background behind the symbol.
    /// If None, background is transparent.
    pub bcolor: Option<(f32, f32, f32, f32)>,

    /// Packed texture and symbol index value
    ///
    /// - High bits: Texture index (which texture to use)
    /// - Low bits: Symbol index (which character/symbol in texture)
    pub texsym: usize,

    /// Screen-space X position (in pixels)
    ///
    /// Note: This value is derived from high-level destination rectangle `s`
    /// produced by helper functions (e.g., `render_helper*`). It may be offset
    /// relative to the original logical top-left to cooperate with the backend
    /// transform chain, which applies additional translation and scaling.
    pub x: f32,

    /// Screen-space Y position (in pixels)
    ///
    /// See notes in `x` for how this value cooperates with the backend transform
    /// chain for final positioning.
    pub y: f32,

    /// Destination width (in pixels)
    ///
    /// This is the final display width for the symbol instance after ratio-based
    /// adjustments performed in helper functions.
    pub w: u32,

    /// Destination height (in pixels)
    ///
    /// This is the final display height for the symbol instance after ratio-based
    /// adjustments performed in helper functions.
    pub h: u32,

    /// Rotation angle (radians)
    ///
    /// Used for sprite rotation effects. 0.0 means no rotation.
    pub angle: f32,

    /// Rotation center X coordinate
    ///
    /// Defines the pivot point for rotation.
    pub cx: f32,

    /// Rotation center Y coordinate
    ///
    /// Defines the pivot point for rotation.
    pub cy: f32,

    /// Style modifier flags for text rendering effects
    /// 样式修饰符标志，用于文本渲染效果
    ///
    /// Bit flags matching Modifier enum:
    /// - 0x0001: BOLD (increase color intensity)
    /// - 0x0002: DIM (reduce alpha)
    /// - 0x0004: ITALIC (shader slant effect)
    /// - 0x0008: UNDERLINED (shader bottom line)
    /// - 0x0010: SLOW_BLINK (ignored in graphics mode)
    /// - 0x0020: RAPID_BLINK (ignored in graphics mode)
    /// - 0x0040: REVERSED (swap fg/bg colors)
    /// - 0x0080: HIDDEN (set alpha to 0)
    /// - 0x0100: CROSSED_OUT (shader middle line)
    pub modifier: u16,
}

pub struct Graph {
    /// Physical window width in pixels
    pub pixel_w: u32,

    /// Physical window height in pixels
    pub pixel_h: u32,

    /// Horizontal scaling ratio for different DPI displays
    ///
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_x: f32,

    /// Vertical scaling ratio for different DPI displays
    ///
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_y: f32,

    /// Whether to use TUI character height (32px) instead of Sprite height (16px)
    ///
    /// - true: Use TUI characters for UI components (double height)
    /// - false: Use Sprite characters for pixel art (single height)
    pub use_tui_height: bool,

    /// Render flag controlling immediate vs buffered rendering
    ///
    /// - true: Direct rendering to screen (normal mode)
    /// - false: Buffered rendering for external access (used for FFI/WASM)
    pub rflag: bool,

    /// Render buffer storing RenderCell array for buffered mode
    ///
    /// When rflag is false, rendered data is stored rbuf instead of
    /// being directly drawn to screen. Used for external access to
    /// rendering data (e.g., Python FFI, WASM exports).
    pub rbuf: Vec<RenderCell>,
}

impl Graph {
    /// Create new graphics rendering context
    ///
    /// Initializes all graphics mode related data structures and rendering state.
    /// Render flag defaults to true (direct rendering to screen).
    /// TUI height mode defaults to false (using Sprite character height).
    pub fn new() -> Self {
        Self {
            pixel_w: 0,
            pixel_h: 0,
            ratio_x: 1.0,
            ratio_y: 1.0,
            use_tui_height: false,
            rflag: true,
            rbuf: Vec::new(),
        }
    }

    /// Set X-axis scaling ratio
    ///
    /// Used for handling scaling adaptation for different DPI displays.
    /// This value affects pixel width calculation and rendering coordinate conversion.
    /// Also sets the global PIXEL_RATIO_X for use in Sprite coordinate conversion.
    ///
    /// # Parameters
    /// - `rx`: X-axis scaling ratio (1.0 for standard ratio)
    pub fn set_ratiox(&mut self, rx: f32) {
        self.ratio_x = rx;
        let _ = PIXEL_RATIO_X.set(rx);
    }

    /// Set Y-axis scaling ratio
    ///
    /// Used for handling scaling adaptation for different DPI displays.
    /// This value affects pixel height calculation and rendering coordinate conversion.
    /// Also sets the global PIXEL_RATIO_Y for use in Sprite coordinate conversion.
    ///
    /// # Parameters
    /// - `ry`: Y-axis scaling ratio (1.0 for standard ratio)
    pub fn set_ratioy(&mut self, ry: f32) {
        self.ratio_y = ry;
        let _ = PIXEL_RATIO_Y.set(ry);
    }

    /// Set whether to use TUI character height mode
    ///
    /// TUI mode uses double height characters (32px) suitable for UI components.
    /// Non-TUI mode uses standard sprite height (16px) suitable for pixel art.
    ///
    /// # Parameters
    /// - `use_tui`: true for TUI mode (double height), false for Sprite mode (standard height)
    pub fn set_use_tui_height(&mut self, use_tui: bool) {
        self.use_tui_height = use_tui;
    }

    /// Calculate and set pixel dimensions based on current settings
    ///
    /// Calculates actual pixel width and height based on cell count, symbol dimensions
    /// and scaling ratios. This is the core method for graphics mode window size calculation.
    ///
    /// # Parameters
    /// - `cell_w`: Game area width (character cell count)
    /// - `cell_h`: Game area height (character cell count)
    ///
    /// # Calculation Formula
    /// ```text
    /// pixel_w = cell_w * symbol_width / ratio_x
    /// pixel_h = cell_h * symbol_height * height_multiplier / ratio_y
    /// ```
    /// Where:
    /// - No border space is added (uses OS window decoration instead)
    /// - height_multiplier = 2.0 for TUI mode (32px), 1.0 for Sprite mode (16px)
    pub fn set_pixel_size(&mut self, cell_w: u16, cell_h: u16) {
        let sym_width = PIXEL_SYM_WIDTH.get().expect("lazylock init");
        let sym_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init");
        
        // Calculate window size without border space
        self.pixel_w = (cell_w as f32 * sym_width / self.ratio_x) as u32;
        
        // Use TUI character height (double sprite height) if use_tui_height is true
        // Otherwise use standard Sprite character height
        let height_multiplier = if self.use_tui_height { 2.0 } else { 1.0 };
        self.pixel_h = (cell_h as f32 * sym_height * height_multiplier / self.ratio_y) as u32;
    }

    /// Get single character cell width (pixels)
    ///
    /// Calculates actual pixel width of a single character cell based on symbol
    /// texture dimensions and current X-axis scaling ratio. This value is used
    /// for precise position calculation and rendering layout.
    ///
    /// # Returns
    /// Pixel width of a single character cell
    pub fn cell_width(&self) -> f32 {
        PIXEL_SYM_WIDTH.get().expect("lazylock init") / self.ratio_x
    }

    /// Get single character cell height (pixels)
    ///
    /// Calculates actual pixel height of a single character cell based on symbol
    /// texture dimensions and current Y-axis scaling ratio. This value is used
    /// for precise position calculation and rendering layout.
    ///
    /// # Returns
    /// Pixel height of a single character cell (base Sprite height: 16px / ratio_y)
    ///
    /// # Note
    /// This returns the base Sprite character height. For TUI mode window height calculation,
    /// see `set_pixel_size` which applies the `use_tui_height` flag to double the height.
    pub fn cell_height(&self) -> f32 {
        PIXEL_SYM_HEIGHT.get().expect("lazylock init") / self.ratio_y
    }
}

/// Convert high-level element data to a GPU-ready RenderCell
///
/// This function converts individual game elements (characters, sprites, etc.) into
/// a GPU-ready RenderCell. It handles:
/// - Texture/symbol indexing and packing (texsym)
/// - Color normalization (u8 → f32)
/// - Destination rectangle mapping (position and size)
/// - Rotation and rotation center
///
/// ## Conversion Process
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Data Transformation                        │
/// │                                                             │
/// │  Game Data Input:                                           │
/// │  ├── Colors (u8 RGBA) ────────► Normalized (f32 RGBA)       │
/// │  ├── Texture & Symbol Index ──► Packed texsym value         │
/// │  ├── Screen Rectangle ─────────► Position & dimensions      │
/// │  ├── Rotation angle ───────────► Angle + center point       │
/// │  └── Background color ─────────► Optional background        │
/// │                                                             │
/// │                       ▼                                     │
/// │               ┌─────────────────────┐                       │
/// │               │    RenderCell       │                       │
/// │               │   (GPU-ready)       │                       │
/// │               └─────────────────────┘                       │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// # Parameters
/// - `rbuf`: Target RenderCell vector to append to
/// - `fc`: Foreground color as (R,G,B,A) in 0-255 range
/// - `bgc`: Optional background color
/// - `texidx`: Texture region identifier (0=TUI, 255=Emoji, 1-254=Sprite)
/// - `symidx`: Symbol index within the region (0-255 for most, 0-1023 for TUI)
/// - `s`: Destination rectangle in screen space (pixels). The helper functions
///        already apply ratio-based sizing and spacing; this function may derive
///        an offset from it to cooperate with backend transform chain.
/// - `angle`: Rotation angle in degrees (will be converted to radians internally)
/// - `ccp`: Center point for rotation
/// - `modifier`: Style modifier flags (see Modifier enum in style.rs)
///
/// # Linear Symbol Array Layout (4096x4096 texture)
///
/// The symbols array uses a simple linear indexing scheme:
/// - **Sprite**: [0, 40959] = 160 blocks × 256 symbols (16×16px each)
///   - Formula: texidx * 256 + symidx
/// - **TUI**: [40960, 43519] = 10 blocks × 256 symbols (16×32px each)
///   - Formula: 40960 + (texidx - 160) * 256 + symidx
/// - **Emoji**: [43520, 44287] = 6 blocks × 128 symbols (32×32px each)
///   - Formula: 43520 + (texidx - 170) * 128 + symidx
/// - **CJK**: [44288, 48383] = 128 cols × 32 rows = 4096 symbols (32×32px each)
///   - Formula: 44288 + linear_offset
pub fn push_render_buffer(
    rbuf: &mut Vec<RenderCell>,
    fc: &(u8, u8, u8, u8),
    bgc: &Option<(u8, u8, u8, u8)>,
    texidx: usize,
    symidx: usize,
    s: ARect,
    angle: f64,
    ccp: &PointI32,
    modifier: u16,
) {
    let mut wc = RenderCell {
        fcolor: (
            fc.0 as f32 / 255.0,
            fc.1 as f32 / 255.0,
            fc.2 as f32 / 255.0,
            fc.3 as f32 / 255.0,
        ),
        modifier,
        ..Default::default()
    };
    if let Some(bc) = bgc {
        wc.bcolor = Some((
            bc.0 as f32 / 255.0,
            bc.1 as f32 / 255.0,
            bc.2 as f32 / 255.0,
            bc.3 as f32 / 255.0,
        ));
    } else {
        wc.bcolor = None;
    }

    // Calculate linear symbol index using centralized function from symbol_map
    wc.texsym = calc_linear_index(texidx, symidx);
    // Derive the instance anchor from the destination rectangle produced by helper functions.
    //
    // The backend transform chain applies additional translation and ratio-compensation.
    // Here we set the instance anchor relative to the destination rectangle to match that chain.
    wc.x = s.x as f32 + s.w as f32;
    wc.y = s.y as f32 + s.h as f32;
    wc.w = s.w;
    wc.h = s.h;
    if angle == 0.0 {
        wc.angle = angle as f32;
    } else {
        let mut aa = (1.0 - angle / 180.0) * std::f64::consts::PI;
        let pi2 = std::f64::consts::PI * 2.0;
        while aa < 0.0 {
            aa += pi2;
        }
        while aa > pi2 {
            aa -= pi2;
        }
        wc.angle = aa as f32;
    }
    wc.cx = ccp.x as f32;
    wc.cy = ccp.y as f32;
    rbuf.push(wc);
}

/// Position calculation helper for rendering (no scaling)
/// sh: (sym_index, tex_index, fg, bg) - first 4 elements from CellInfo
pub fn render_helper(
    cell_w: u16,
    r: PointF32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    use_tui: bool,
) -> (ARect, usize, usize) {
    render_helper_with_scale(cell_w, r, i, sh, p, use_tui, 1.0, 1.0, 1.0, None)
}

/// Enhanced helper that returns destination rectangle and symbol indices with per-sprite scaling.
/// sh: (sym_index, tex_index, fg, bg) - first 4 elements from CellInfo
///
/// # Parameters
/// - `scale_x`, `scale_y`: Combined scale (sprite_scale * cell_scale)
/// - `sprite_scale_y`: Sprite-level Y scale, used for row height and vertical centering
/// - `cumulative_x`: Pre-computed cumulative X position in pixel space (before adding p.x).
///   When Some, uses cumulative layout; when None, uses grid-based layout.
///
/// Returns: (dest_rect, texture_id, symbol_id)
pub fn render_helper_with_scale(
    cell_w: u16,
    r: PointF32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    use_tui: bool,        // Global TUI mode flag; TUI height also auto-detected from block index
    scale_x: f32,         // Combined (sprite * cell) scaling along X
    scale_y: f32,         // Combined (sprite * cell) scaling along Y
    sprite_scale_y: f32,  // Sprite-level Y scale for row height calculation
    cumulative_x: Option<f32>, // Pre-computed cumulative X pixel position
) -> (ARect, usize, usize) {
    let w = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as i32; // 16 pixels
    // Height depends on character type:
    // - Sprite (block 0-159): 16 pixels
    // - TUI (block 160-169): 32 pixels (double height)
    // Auto-detect from block index so sprites with TUI-region cells
    // get correct height without needing manual scale_y=2.0 workaround.
    let cell_is_tui = use_tui || (sh.1 >= 160 && sh.1 < 170);
    let h = if cell_is_tui {
        (*PIXEL_SYM_HEIGHT.get().expect("lazylock init") * 2.0) as i32 // TUI: 32 pixels
    } else {
        *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as i32 // Sprite: 16 pixels
    };

    let dsty = i as u16 / cell_w;

    let tx = sh.1 as usize;

    // Compute tiling-corrected sizes and positions.
    // When cell size is non-integer (e.g., 16/1.2 = 13.333), independently rounding
    // each cell's position and width creates 1-pixel gaps between adjacent cells.
    // Fix: compute width/height as round(next_pos) - round(this_pos), ensuring
    // adjacent cells tile perfectly (sizes alternate e.g. 13,14,13,13,14,...).
    let (scaled_x, scaled_y, scaled_w, scaled_h) = if let Some(cum_x) = cumulative_x {
        // Cumulative layout: X from accumulated widths, Y centered within row
        let row_h = h as f32 / r.y * sprite_scale_y;
        let cell_h = h as f32 / r.y * scale_y;
        let y_offset = (row_h - cell_h) / 2.0;
        let base_y = dsty as f32 * row_h;
        let this_y_f = base_y + y_offset;
        let next_y_f = (dsty as f32 + 1.0) * row_h + y_offset;
        // Width: use rounded value here; caller (render_buffer_to_cells) will
        // correct it with grid_advance info for perfect X-direction tiling.
        let w_val = (w as f32 / r.x * scale_x).round() as u32;
        // Height: tiling-corrected only when cell fills its row (no per-cell Y scaling).
        // When y_offset != 0, cell is intentionally smaller and centered in row.
        let h_val = if y_offset.abs() < 0.01 {
            (next_y_f.round() - this_y_f.round()) as u32
        } else {
            (h as f32 / r.y * scale_y).round() as u32
        };
        (cum_x.round(), this_y_f.round(), w_val, h_val)
    } else {
        // Grid-based layout with tiling fix
        let dstx = i as u16 % cell_w;
        let cell_f_w = w as f32 / r.x * scale_x;
        let cell_f_h = h as f32 / r.y * scale_y;
        let this_x = (dstx as f32 * cell_f_w).round();
        let this_y = (dsty as f32 * cell_f_h).round();
        let next_x = ((dstx as f32 + 1.0) * cell_f_w).round();
        let next_y = ((dsty as f32 + 1.0) * cell_f_h).round();
        (this_x, this_y, (next_x - this_x) as u32, (next_y - this_y) as u32)
    };

    (
        ARect {
            x: scaled_x as i32 + p.x as i32,
            y: scaled_y as i32 + p.y as i32,
            w: scaled_w,
            h: scaled_h,
        },
        tx,
        sh.0 as usize,
    )
}

/// Unified buffer to RenderCell conversion with full transformation support
///
/// This is the core rendering primitive that converts a Buffer's content to RenderCell
/// format. It supports both TUI mode (16×32 characters) and Sprite mode (8×8 characters),
/// with full transformation parameters (alpha, scale, rotation).
///
/// # Parameters
/// - `buf`: Source buffer containing character data
/// - `rx`, `ry`: Display ratio for scaling compensation
/// - `use_tui`: Use TUI characters (16×32) if true, Sprite characters (8×8) if false
/// - `alpha`: Overall transparency (0=transparent, 255=opaque)
/// - `scale_x`, `scale_y`: Overall scale factors (1.0 = no scaling)
/// - `angle`: Overall rotation angle in degrees (0.0 = no rotation)
/// - `f`: Callback function to process each RenderCell
///
/// # TUI Mode Special Handling
/// When `use_tui=true`:
/// - Symbols in Sprite region (< 160) are remapped to TUI region (160+)
/// - Emoji (block >= 170) are rendered with double width
/// - Emoji preserve original colors (no color modulation)
pub fn render_buffer_to_cells<F>(
    buf: &Buffer,
    rx: f32,
    ry: f32,
    use_tui: bool,
    alpha: u8,
    scale_x: f32,
    scale_y: f32,
    angle: f64,
    mut f: F,
) where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize, f64, PointI32, u16),
{
    let px = buf.area.x;
    let py = buf.area.y;
    let pw = buf.area.width;
    let ph = buf.area.height;

    let w = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as f32;
    let h = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as f32;

    // Base cell width in pixel space (before any scaling)
    let base_cell_w = w / rx;

    // Track cumulative X position per row for per-cell scale support
    let mut cumulative_x: f32 = 0.0;
    let mut last_row: u16 = 0;

    let mut skip_next = false;
    for (i, cell) in buf.content.iter().enumerate() {
        let row = i as u16 / pw;

        // Reset cumulative_x at row boundary
        if row != last_row {
            cumulative_x = 0.0;
            last_row = row;
        }

        if skip_next {
            skip_next = false;
            // Don't accumulate width - already counted by the emoji's double width
            continue;
        }

        // Combined scale: sprite-level * per-cell
        let cell_sx = scale_x * cell.scale_x;
        let cell_sy = scale_y * cell.scale_y;

        // Extract CellInfo: symidx, texidx, fg, bg, modifier
        let cell_info = cell.get_cell_info();
        let mut sh = (cell_info.0, cell_info.1, cell_info.2, cell_info.3);
        let modifier = cell_info.4.bits();

        // TUI mode: remap Sprite region symbols to TUI region
        if use_tui && sh.1 < 160 {
            if let Some((block, idx)) = tui_symidx(&cell.symbol) {
                sh.1 = block;
                sh.0 = idx;
            } else {
                sh.1 = 160;
                sh.0 = 0;
            }
        }

        // Fixed grid slot width (sprite-level scale only; per-cell scale doesn't affect spacing)
        let slot_w = base_cell_w * scale_x;
        // Center the scaled cell within its fixed grid slot
        let rendered_w = base_cell_w * cell_sx;
        let x_center_offset = (slot_w - rendered_w) / 2.0;

        // Calculate destination rectangle with combined scaling and centered X
        let (mut s2, texidx, symidx) = render_helper_with_scale(
            pw,
            PointF32 { x: rx, y: ry },
            i,
            &sh,
            PointU16 { x: px, y: py },
            use_tui,
            cell_sx,
            cell_sy,
            scale_y,
            Some(cumulative_x + x_center_offset),
        );

        // Grid advance: fixed slot width (per-cell scale doesn't shift neighbors)
        let mut grid_advance = slot_w;

        // TUI mode: handle Emoji double-width
        if use_tui && texidx >= 170 {
            s2.w *= 2;
            grid_advance = slot_w * 2.0;
            if (i + 1) % pw as usize != 0 {
                skip_next = true;
            }
        }

        // Fix width to ensure perfect tiling (prevent sub-pixel gaps from integer rounding).
        // When cell size is non-integer (e.g., 16/1.2 = 13.333), independently rounding
        // each cell's position and using a fixed rounded width creates 1-pixel gaps every
        // few cells. Fix by computing width as: round(next_position) - round(this_position),
        // so adjacent cells tile without gaps (widths alternate e.g. 13,14,13,13,14,...).
        // Only apply when cell fills its slot (no per-cell scaling); cells with per-cell
        // scaling (e.g., 0.5x emoji bullets) are intentionally smaller and don't need tiling.
        if x_center_offset.abs() < 0.01 {
            let this_x_f = cumulative_x + x_center_offset;
            let next_x_f = this_x_f + grid_advance;
            let corrected_w = next_x_f.round() as i32 - this_x_f.round() as i32;
            if corrected_w > 0 {
                s2.w = corrected_w as u32;
            }
        }

        // Accumulate by fixed grid slot width
        cumulative_x += grid_advance;

        // Calculate rotation center point (uses sprite-level scale, not per-cell)
        let x = i % pw as usize;
        let y = i / pw as usize;
        let original_offset_x = (pw as f32 / 2.0 - x as f32) * w / rx;
        let original_offset_y = (ph as f32 / 2.0 - y as f32) * h / ry;
        let ccp = PointI32 {
            x: (original_offset_x * scale_x) as i32,
            y: (original_offset_y * scale_y) as i32,
        };

        // Apply alpha to colors
        // For Emoji in TUI mode, use white (no color modulation)
        let fc = if use_tui && texidx >= 170 && texidx <= 175 {
            (255, 255, 255, alpha)
        } else {
            let mut rgba = sh.2.get_rgba();
            rgba.3 = alpha;
            rgba
        };

        let bc = if sh.3 != Color::Reset {
            let mut brgba = sh.3.get_rgba();
            brgba.3 = alpha;
            Some(brgba)
        } else {
            None
        };

        f(&fc, &bc, s2, texidx, symidx, angle, ccp, modifier);
    }
}

/// Render pixel sprites with rotation and transformation support
///
/// **DEPRECATED**: Use `render_buffer_to_cells` instead. This function is kept
/// for backward compatibility but internally uses the unified function.
///
/// This function processes individual sprite objects and converts them to renderable
/// format. It supports advanced features like rotation, scaling, and complex
/// transformations while maintaining efficient rendering performance.
///
/// ## Sprite Rendering Pipeline
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │                   Sprite Processing                        │
/// │                                                            │
/// │  ┌─────────────┐                                           │
/// │  │   Sprite    │                                           │
/// │  │   Object    │                                           │
/// │  │  ┌───────┐  │  ┌─────────────────────────────────────┐  │
/// │  │  │Pixels │  │  │        Transformation               │  │
/// │  │  │Array  │  │  │  ┌────────────────────────────────┐ │  │
/// │  │  └───────┘  │  │  │  1. Position calculation       │ │  │
/// │  │     │       │  │  │  2. Rotation matrix applied    │ │  │
/// │  │     ▼       │  │  │  3. Scaling based on rx/ry     │ │  │
/// │  │  ┌───────┐  │  │  │  4. Color & texture mapping    │ │  │
/// │  │  │Colors │  │  │  └────────────────────────────────┘ │  │
/// │  │  │&Flags │  │  └─────────────────────────────────────┘  │
/// │  │  └───────┘  │                     │                     │
/// │  └─────────────┘                     ▼                     │
/// │                        ┌─────────────────────┐             │
/// │                        │  Callback Function  │             │
/// │                        │ (push_render_buffer)│             │
/// │                        └─────────────────────┘             │
/// │                                 │                          │
/// │                                 ▼                          │
/// │                        ┌─────────────────────┐             │
/// │                        │    RenderCell       │             │
/// │                        │      Array          │             │
/// │                        └─────────────────────┘             │
/// └────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Features Supported
/// - **Rotation**: Full 360-degree rotation around sprite center
/// - **Scaling**: Display ratio compensation for different screen densities
/// - **Transparency**: Alpha blending and background color support
/// - **Instanced Rendering**: Efficient batch processing for multiple sprites
///
/// # Parameters
/// - `pixel_spt`: Sprite object containing pixel data and transformation info
/// - `rx`: Horizontal scaling ratio for display compensation
/// - `ry`: Vertical scaling ratio for display compensation
/// - `f`: Callback function to process each sprite pixel
pub fn render_layers<F>(layers: &mut Layer, rx: f32, ry: f32, mut f: F)
where
    // Callback signature: (fg_color, bg_color, dst_rect, tex_idx, sym_idx, angle, center_point)
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize, f64, PointI32),
{
    // Sort by render_weight
    layers.update_render_index();

    for si in &layers.render_index.clone() {
        let s = &layers.sprites[si.0];
        if s.is_hidden() {
            continue;
        }

        // Use unified function with sprite's transformation parameters
        render_buffer_to_cells(
            &s.content,
            rx,
            ry,
            false,      // No TUI remapping; TUI height auto-detected from block index
            s.alpha,
            s.scale_x,
            s.scale_y,
            s.angle,
            |fc, bc, s2, texidx, symidx, angle, ccp, _modifier| {
                // Forward to original callback (ignore modifier for backward compatibility)
                f(fc, bc, s2, texidx, symidx, angle, ccp);
            },
        );
    }
}

/// Main buffer rendering with character-to-pixel conversion
///
/// This function processes the main game buffer containing character data and
/// converts it to renderable format. It follows the principle.md design where
/// characters are the fundamental rendering unit, with each character mapped
/// to symbols in the texture atlas.
///
/// ## Buffer Rendering Process
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │                   Main Buffer Processing                   │
/// │                                                            │
/// │  ┌─────────────────────┐                                   │
/// │  │      Buffer         │                                   │
/// │  │   ┌─────────────┐   │                                   │
/// │  │   │ Character   │   │    ┌─────────────────────────────┐│
/// │  │   │   Grid      │   │    │   Per-Character Process     ││
/// │  │   │             │   │    │                             ││
/// │  │   │ ┌─┬─┬─┬─┐   │   │    │ 1. Read character data      ││
/// │  │   │ │A│B│C│D│   │   │    │ 2. Extract colors & symbol  ││
/// │  │   │ ├─┼─┼─┼─┤   │──────► │ 3. Calculate screen pos     ││
/// │  │   │ │E│F│G│H│   │   │    │ 4. Map to texture coords    ││
/// │  │   │ ├─┼─┼─┼─┤   │   │    │ 5. Call render callback     ││
/// │  │   │ │I│J│K│L│   │   │    │                             ││
/// │  │   │ └─┴─┴─┴─┘   │   │    └─────────────────────────────┘│
/// │  │   └─────────────┘   │                     │             │
/// │  └─────────────────────┘                     ▼             │
/// │                                ┌─────────────────────┐     │
/// │                                │   RenderCell Array  │     │
/// │                                │   (GPU-ready data)  │     │
/// │                                └─────────────────────┘     │
/// └────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Character Data Structure
/// Each character in the buffer contains:
/// - **Symbol Index**: Which character/symbol to display
/// - **Texture Index**: Which texture sheet to use
/// - **Foreground Color**: Primary character color
/// - **Background Color**: Optional background fill color
/// - **Style Flags**: Bold, italic, underline, etc.
///
/// # Parameters
/// - `buf`: Game buffer containing character grid data
/// - `width`: Buffer width in characters
/// - `rx`: Horizontal scaling ratio for display adaptation
/// - `ry`: Vertical scaling ratio for display adaptation
/// - `use_tui`: Use TUI characters (16×32) instead of Sprite characters (16×16)
/// - `f`: Callback function to process each character (RenderCell)
///   Signature: (fg_color, bg_color, dest_rect, tex_idx, sym_idx, modifier)
pub fn render_main_buffer<F>(
    buf: &Buffer,
    _width: u16,  // Deprecated: width is now taken from buf.area.width
    rx: f32,
    ry: f32,
    use_tui: bool,
    mut f: F,
) where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize, u16),
{
    // Use unified function with default transformation (no scale, no rotation, full opacity)
    render_buffer_to_cells(
        buf,
        rx,
        ry,
        use_tui,
        255,   // alpha: fully opaque
        1.0,   // scale_x: no scaling
        1.0,   // scale_y: no scaling
        0.0,   // angle: no rotation
        |fc, bc, s2, texidx, symidx, _angle, _ccp, modifier| {
            // Forward to original callback (ignore angle and ccp for backward compatibility)
            f(fc, bc, s2, texidx, symidx, modifier);
        },
    );
}

/// Render window borders (DEPRECATED - Not used in current implementation)
///
/// **Note**: This function is kept for backward compatibility but is no longer used.
/// RustPixel now uses OS native window decoration (title bar, border) instead of
/// custom-drawn borders for better integration with the desktop environment.
///
/// ## Previous Border Layout (Historical Reference)
/// ```text
/// ┌───────────────────────────────────────────────────────┐
/// │                      Window Border                    │
/// │  ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  ├─┤                 Game Content Area           ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                     80 x 40                 ├─┤  │
/// │  ├─┤                  Character Grid             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘  │
/// └───────────────────────────────────────────────────────┘
/// ```
///
/// The border consists of:
/// - **Top/Bottom Edges**: Horizontal line characters
/// - **Left/Right Edges**: Vertical line characters
/// - **Corners**: Corner junction characters
/// - **Consistent Styling**: Matches the game's visual theme
///
/// # Parameters
/// - `cell_w`: Game area width in characters
/// - `cell_h`: Game area height in characters
/// - `rx`: Horizontal scaling ratio
/// - `ry`: Vertical scaling ratio
/// - `f`: Callback function to render each border character
///
/// # Deprecation Notice
/// This function is no longer called in the rendering pipeline. Applications now
/// use OS window decoration for better native integration.
#[allow(dead_code)]
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, usize, usize),
{
    let sh_top = (102u8, 1u8, Color::Indexed(7), Color::Reset);
    let sh_other = (24u8, 2u8, Color::Indexed(7), Color::Reset);
    let sh_close = (214u8, 1u8, Color::Indexed(7), Color::Reset);

    for n in 0..cell_h as usize + 2 {
        for m in 0..cell_w as usize + 2 {
            if n != 0 && n != cell_h as usize + 1 && m != 0 && m != cell_w as usize + 1 {
                continue;
            }
            let rsh;
            if n == 0 {
                if m as u16 <= cell_w {
                    rsh = &sh_top;
                } else {
                    rsh = &sh_close;
                }
            } else {
                rsh = &sh_other;
            }
            let (s2, texidx, symidx) = render_helper(
                cell_w + 2,
                PointF32 { x: rx, y: ry },
                n * (cell_w as usize + 2) + m,
                rsh,
                PointU16 { x: 0, y: 0 },
                false, // Border uses Sprite characters (8×8)
            );
            let fc = rsh.2.get_rgba();
            let bc = None;
            f(&fc, &bc, s2, texidx, symidx);
        }
    }
}

/// Render the RustPixel logo animation with dynamic effects
///
/// This function renders the animated RustPixel logo during the startup sequence.
/// It provides a visually appealing introduction to the framework with dynamic
/// effects and proper centering across different screen resolutions.
///
/// ## Logo Animation Sequence
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Logo Animation Timeline                  │
/// │                                                             │
/// │  Stage 0 ────────────────────────────────► LOGO_FRAME       │
/// │    │                                            │           │
/// │    ▼                                            ▼           │
/// │  ┌─────────────────┐                    ┌─────────────────┐ │
/// │  │  Logo Display   │                    │  Start Game     │ │
/// │  │                 │                    │   Rendering     │ │
/// │  │  ┌───────────┐  │                    │                 │ │
/// │  │  │ ██████    │  │   Dynamic Effects: │                 │ │
/// │  │  │ ██  ██    │  │   - Random colors  │                 │ │
/// │  │  │ ██████    │  │   - Centered pos   │                 │ │
/// │  │  │ ██  ██    │  │   - Smooth trans   │                 │ │
/// │  │  │ ██  ██    │  │   - Frame timing   │                 │ │
/// │  │  └───────────┘  │                    │                 │ │
/// │  └─────────────────┘                    └─────────────────┘ │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Rendering Features
/// - **Centered Positioning**: Automatically centers on any screen size
/// - **Dynamic Colors**: Randomly generated color effects for visual appeal
/// - **Smooth Animation**: Frame-based timing for consistent display
/// - **High-DPI Support**: Proper scaling for different display densities
/// - **Cross-platform**: Works consistently across SDL, Winit, and Web modes
///
/// ## Logo Data Processing
/// The function processes the PIXEL_LOGO constant array where each character
/// is represented by 3 bytes: [symbol_id, texture_id, flags]. The logo is
/// dynamically positioned and colored based on the current animation stage.
///
/// # Parameters
/// - `srx`: Screen horizontal scaling ratio
/// - `sry`: Screen vertical scaling ratio
/// - `spw`: Screen physical width in pixels
/// - `sph`: Screen physical height in pixels
/// - `rd`: Random number generator for color effects
/// - `stage`: Current animation stage (0 to LOGO_FRAME)
/// - `f`: Callback function to render each logo character
pub fn render_logo<F>(srx: f32, sry: f32, spw: u32, sph: u32, rd: &mut Rand, stage: u32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, usize, usize),
{
    let rx = srx * 1.0;
    let ry = sry * 1.0;
    for y in 0usize..PIXEL_LOGO_HEIGHT {
        for x in 0usize..PIXEL_LOGO_WIDTH {
            let sci = y * PIXEL_LOGO_WIDTH + x;
            let symw = PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
            let symh = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

            let (mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                PointF32 { x: rx, y: ry },
                sci,
                &(
                    PIXEL_LOGO[sci * 3],
                    PIXEL_LOGO[sci * 3 + 2],
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]),
                    Color::Reset,
                ),
                PointU16 {
                    x: spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                    y: sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                },
                false, // Logo uses Sprite characters (8×8)
            );
            let fc = Color::Indexed(PIXEL_LOGO[sci * 3 + 1]).get_rgba();

            let randadj = 12 - (rd.rand() % 24) as i32;
            let sg = LOGO_FRAME as u8 / 3;
            let r: u8;
            let g: u8;
            let b: u8;
            let a: u8;
            if stage <= sg as u32 {
                r = (stage as u8).saturating_mul(10);
                g = (stage as u8).saturating_mul(10);
                b = (stage as u8).saturating_mul(10);
                a = 255;
                s2.x += randadj;
            } else if stage <= sg as u32 * 2 {
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255;
            } else {
                let cc = (stage as u8 - sg * 2).saturating_mul(10);
                r = fc.0.saturating_sub(cc);
                g = fc.1.saturating_sub(cc);
                b = fc.2.saturating_sub(cc);
                a = 255;
            }
            f(&(r, g, b, a), s2, texidx, symidx);
        }
    }
}

/// Generate unified render buffer from main buffer and sprite layers
///
/// This is the main rendering pipeline entry point that merges multiple rendering sources
/// into a unified GPU-ready buffer. It handles:
/// - Main character buffer rendering
/// - Sprite layer composition
/// - Logo animation during startup
///
/// # Parameters
/// - `cb`: Current main buffer containing character grid data
/// - `_pb`: Previous buffer (unused, kept for API compatibility)
/// - `ps`: Vector of sprite layers to render
/// - `stage`: Current animation stage (for logo animation, 0 = game content)
/// - `base`: Adapter base containing graphics context and scaling information
///
/// # Returns
/// Vector of RenderCell ready for GPU rendering
///
/// # Rendering Order
/// 1. If stage > 0: Render animated logo (startup sequence)
/// 2. If stage == 0: Render main buffer + sprite layers (normal game rendering)
pub fn generate_render_buffer(
    cb: &Buffer,
    _pb: &Buffer,
    ps: &mut Vec<Layer>,
    stage: u32,
    base: &mut AdapterBase,
) -> Vec<RenderCell> {
    let mut rbuf = vec![];
    let width = cb.area.width;
    let pz = PointI32 { x: 0, y: 0 };

    // render logo...
    if stage <= LOGO_FRAME {
        render_logo(
            base.gr.ratio_x,
            base.gr.ratio_y,
            base.gr.pixel_w,
            base.gr.pixel_h,
            &mut base.rd,
            stage,
            |fc, s2, texidx, symidx| {
                // Logo uses no modifier (0)
                push_render_buffer(&mut rbuf, fc, &None, texidx, symidx, s2, 0.0, &pz, 0);
            },
        );
        return rbuf;
    }

    let rx = base.gr.ratio_x;
    let ry = base.gr.ratio_y;

    // Custom border rendering - use OS window decoration instead
    // #[cfg(graphics_backend)]
    // render_border(base.cell_w, base.cell_h, rx, ry, &mut rfunc);

    // Rendering order (back to front):
    // 1. Pixel Sprites (game objects, backgrounds)
    // 2. Main Buffer (TUI layer - always on top)
    //
    // This ensures TUI layer is rendered last and appears on top of all sprites.

    if stage > LOGO_FRAME {
        // render pixel_sprites first (bottom layer)
        // Note: All layers are now uniform (is_pixel removed), render all non-hidden layers
        for item in ps {
            if !item.is_hidden {
                render_layers(item, rx, ry, |fc, bc, s2, texidx, symidx, angle, ccp| {
                    // Pixel sprites currently don't use modifier (0)
                    push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, angle, &ccp, 0);
                });
            }
        }

        // render main buffer last (TUI layer - top layer)
        // Use TUI characters (8×16) for UI components in graphics mode
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize,
                         modifier: u16| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz, modifier);
        };
        render_main_buffer(cb, width, rx, ry, true, &mut rfunc);
    }

    rbuf
}
