// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Sprite further encapsulates Buffer.
//! It is also the most common component in RustPixel.
//! It provides drawing methods such as set_border, draw_line, draw_circle.
//! Refer to util/shape.rs for an example of how to draw a line.

use crate::{
    asset::{AssetManager, AssetState, AssetType},
    render::buffer::Buffer,
    util::{PointF32, Rect},
};
#[cfg(graphics_mode)]
use crate::render::graph::{get_ratio_x, get_ratio_y, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use std::ops::{Deref, DerefMut};

mod layer;
pub use layer::Layer;

// Re-export from buffer for backward compatibility
pub use crate::render::buffer::{SYMBOL_LINE, Borders, BorderType};

/// Used to simplify the call to set_content_by_asset method
/// Returns true if asset is loaded and ready, false if still loading (web mode)
#[macro_export]
macro_rules! asset2sprite {
    ($spr:expr, $ctx:expr, $loc:expr $(, $arg:expr)* ) => {{
        let ll = $loc.to_lowercase();
        // determine asset type...
        let mut at = AssetType::ImgPix;
        if ll.ends_with(".txt") {
            at = AssetType::ImgEsc;
        }
        if ll.ends_with(".pix") {
            at = AssetType::ImgPix;
        }
        if ll.ends_with(".ssf") {
            at = AssetType::ImgSsf;
        }
        // collect other args...
        let mut va = Vec::new();
        $( va.push($arg); )*
        let mut frame_idx = 0;
        let mut x = 0;
        let mut y = 0;
        match va.len() {
            1 => {
                frame_idx = va[0];
            },
            3 => {
                frame_idx = va[0];
                x = va[1] as u16;
                y = va[2] as u16;
            },
            _ => {},
        }

        // Use global GAME_CONFIG for project path
        // 使用全局 GAME_CONFIG 获取项目路径
        let nl = if cfg!(not(target_arch = "wasm32")) {
            &format!("{}{}assets{}{}",
                rust_pixel::get_game_config().project_path,
                std::path::MAIN_SEPARATOR,
                std::path::MAIN_SEPARATOR,
                $loc)
        } else {
            &format!("assets{}{}", std::path::MAIN_SEPARATOR, $loc)
        };

        // call spr.set_content_by_asset...
        $spr.set_content_by_asset(
            &mut $ctx.asset_manager,
            at,
            nl,
            frame_idx,
            x,
            y,
        );

        // Return loading status
        $spr.check_asset_request(&mut $ctx.asset_manager)
    }};
}

/// New macro specifically for handling raw paths (no path processing)
#[macro_export]
macro_rules! asset2sprite_raw {
    ($spr:expr, $ctx:expr, $loc:expr $(, $arg:expr)* ) => {{
        let ll = $loc.to_lowercase();
        // determine asset type...
        let mut at = AssetType::ImgPix;
        if ll.ends_with(".txt") {
            at = AssetType::ImgEsc;
        }
        if ll.ends_with(".pix") {
            at = AssetType::ImgPix;
        }
        if ll.ends_with(".ssf") {
            at = AssetType::ImgSsf;
        }
        
        // collect other args...
        let mut va = Vec::new();
        $( va.push($arg); )*
        let mut frame_idx = 0;
        let mut x = 0;
        let mut y = 0;
        match va.len() {
            1 => {
                frame_idx = va[0];
            },
            3 => {
                frame_idx = va[0];
                x = va[1] as u16;
                y = va[2] as u16;
            },
            _ => {},
        }
        
        // Use raw path directly without any processing
        let nl = $loc;
        
        $spr.set_content_by_asset(
            &mut $ctx.asset_manager,
            at,
            nl,
            frame_idx,
            x,
            y,
        );
    }};
}

pub trait Widget {
    fn render(&mut self, am: &mut AssetManager, buf: &mut Buffer);
}

/// Sprite: A positioned drawable element with dual coordinate semantics.
///
/// # Coordinate System
///
/// Sprite uses **dual coordinate semantics** that automatically adapt to the rendering mode:
///
/// ## Text Mode (Terminal/Crossterm)
/// - Position coordinates (`content.area.x/y`) are in **Cell units** (character grid)
/// - Each unit represents one character cell in the terminal
/// - Position is automatically aligned to the character grid
/// - Graphics-specific features (rotation, transparency, scaling) are ignored
/// - Example: `set_pos(10, 5)` places sprite at column 10, row 5
///
/// ## Graphics Mode (SDL/Glow/WGPU/Web)
/// - Position coordinates are in **Pixel units** (sub-pixel precision)
/// - Values are cast from u16 to f32 for rendering calculations
/// - Supports pixel-perfect positioning, rotation, transparency, and scaling
/// - Example: `set_pos(100, 50)` places sprite at pixel position (100.0, 50.0)
///
/// ## Helper Methods
///
/// For code clarity, use these helper methods:
/// - `set_cell_pos(x, y)` - Explicitly set position in cell units (text mode intent)
/// - `set_pixel_pos(x, y)` - Explicitly set position in pixel units (graphics mode intent)
/// - `pixel_pos()` - Get current position as floating-point pixels
///
/// ## Field Details
///
/// - `content.area.x/y`: Position coordinates (dual semantics: cells in text, pixels in graphics)
/// - `angle`: Rotation angle in degrees (graphics mode only)
/// - `alpha`: Transparency level 0-255 (graphics mode only, text mode uses merge logic)
/// - `scale_x/scale_y`: Scaling factors (graphics mode only)
/// - `render_weight`: Controls rendering order (negative = hidden)
///
#[derive(Clone)]
pub struct Sprite {
    pub content: Buffer,
    pub angle: f64,
    pub alpha: u8,
    pub scale_x: f32,  // X-axis scaling factor: 1.0 = normal, 0.5 = half width
    pub scale_y: f32,  // Y-axis scaling factor: 1.0 = normal, 2.0 = double height
    pub use_tui: bool,  // Render text as TUI chars (16×32) instead of Sprite chars (16×16)
    asset_request: Option<(AssetType, String, usize, u16, u16)>,
    render_weight: i32,
}

impl Widget for Sprite {
    fn render(&mut self, am: &mut AssetManager, buf: &mut Buffer) {
        if !self.is_hidden() {
            self.check_asset_request(am);
            buf.merge(&self.content, self.alpha, true);
        }
    }
}

/// Deref to Buffer so all Buffer content-drawing methods
/// (set_color_str, draw_circle, draw_line, set_border, etc.)
/// are automatically available on Sprite without wrapping.
impl Deref for Sprite {
    type Target = Buffer;
    fn deref(&self) -> &Buffer {
        &self.content
    }
}

impl DerefMut for Sprite {
    fn deref_mut(&mut self) -> &mut Buffer {
        &mut self.content
    }
}

impl Sprite {
    /// Create a new Sprite with Sprite mode buffer (PUA encoding).
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        let area = Rect::new(x, y, width, height);
        let buffer = Buffer::empty_sprite(area);  // Sprite mode by default
        Self {
            content: buffer,
            angle: 0.0,
            alpha: 255,
            scale_x: 1.0,  // Default: normal scaling
            scale_y: 1.0,  // Default: normal scaling
            use_tui: false, // Default: Sprite chars (16×16)
            asset_request: None,
            render_weight: 1,
        }
    }

    /// Create a new Sprite with TUI mode buffer (Unicode).
    pub fn new_tui(x: u16, y: u16, width: u16, height: u16) -> Self {
        let area = Rect::new(x, y, width, height);
        let buffer = Buffer::empty(area);  // TUI mode
        Self {
            content: buffer,
            angle: 0.0,
            alpha: 255,
            scale_x: 1.0,
            scale_y: 1.0,
            use_tui: true,  // TUI chars (16×32)
            asset_request: None,
            render_weight: 1,
        }
    }

    pub fn set_alpha(&mut self, a: u8) {
        self.alpha = a;
    }

    /// Set X-axis scaling (0.5 = half width, 1.0 = normal, 2.0 = double width)
    pub fn set_scale_x(&mut self, scale: f32) {
        self.scale_x = scale;
    }

    /// Set Y-axis scaling (0.5 = half height, 1.0 = normal, 2.0 = double height)
    pub fn set_scale_y(&mut self, scale: f32) {
        self.scale_y = scale;
    }

    /// Set uniform scaling for both X and Y axes
    pub fn set_scale(&mut self, scale: f32) {
        self.scale_x = scale;
        self.scale_y = scale;
    }

    /// Enable TUI character rendering (16×32) instead of Sprite chars (16×16)
    pub fn set_use_tui(&mut self, use_tui: bool) {
        self.use_tui = use_tui;
    }

    /// Create a half-width sprite (convenience method)
    pub fn new_half_width(x: u16, y: u16, width: u16, height: u16) -> Self {
        let mut sprite = Self::new(x, y, width, height);
        sprite.set_scale_x(0.5);  // Half width
        sprite
    }

    pub fn set_content_by_asset(
        &mut self,
        am: &mut AssetManager,
        atype: AssetType,
        location: &str,
        frame_idx: usize,
        off_x: u16,
        off_y: u16,
    ) {
        self.asset_request = Some((atype, location.to_string(), frame_idx, off_x, off_y));
        am.load(atype, location);
        self.check_asset_request(am);
    }

    pub fn check_asset_request(&mut self, am: &mut AssetManager) -> bool {
        if let Some(req) = &self.asset_request {
            if let Some(ast) = am.get(&req.1) {
                if ast.get_state() == AssetState::Ready {
                    ast.set_sprite(self, req.2, req.3, req.4);
                    self.asset_request = None;
                    return true;
                }
            }
        } else {
            return true;
        }
        false
    }

    pub fn set_angle(&mut self, a: f64) {
        self.angle = a;
    }

    pub fn get_center_point(&self) -> PointF32 {
        PointF32 {
            x: self.content.area.x as f32 + self.content.area.width as f32 / 2.0,
            y: self.content.area.y as f32 + self.content.area.height as f32 / 2.0,
        }
    }

    pub fn set_hidden(&mut self, flag: bool) {
        if flag {
            self.render_weight = -self.render_weight.abs();
        } else {
            self.render_weight = self.render_weight.abs();
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.render_weight < 0
    }

    pub fn copy_content(&mut self, sp: &Sprite) {
        let backup_area = self.content.area;
        //set the pos to (0,0) to merge with boxes
        self.content.area = Rect::new(0, 0, backup_area.width, backup_area.height);
        self.content.reset();
        self.content.merge(&sp.content, sp.alpha, false);

        //after merging, set back to its original pos
        self.content.area = backup_area;
    }

    pub fn set_pos(&mut self, x: u16, y: u16) {
        self.content.area = Rect::new(x, y, self.content.area.width, self.content.area.height);
    }

    /// Set sprite position in cell units.
    ///
    /// In text mode: positions sprite at the specified character grid cell.
    /// In graphics mode: automatically converts cell coordinates to pixel coordinates
    /// using PIXEL_SYM_WIDTH/HEIGHT and ratio_x/y.
    ///
    /// # Example
    /// ```ignore
    /// sprite.set_cell_pos(10, 5);  // Cell position (10, 5)
    /// // In graphics mode: converted to pixel position (10 * sym_w / rx, 5 * sym_h / ry)
    /// ```
    #[cfg(not(graphics_mode))]
    pub fn set_cell_pos(&mut self, x: u16, y: u16) {
        self.set_pos(x, y);
    }

    /// Set sprite position in cell units (graphics mode version).
    ///
    /// Automatically converts cell coordinates to pixel coordinates using:
    /// - PIXEL_SYM_WIDTH/HEIGHT: Symbol dimensions from texture
    /// - PIXEL_RATIO_X/Y: DPI scaling ratios
    ///
    /// Formula: pixel_pos = cell_pos * sym_size / ratio
    #[cfg(graphics_mode)]
    pub fn set_cell_pos(&mut self, x: u16, y: u16) {
        let sym_w = PIXEL_SYM_WIDTH.get().copied().unwrap_or(16.0);
        let sym_h = PIXEL_SYM_HEIGHT.get().copied().unwrap_or(16.0);
        let rx = get_ratio_x();
        let ry = get_ratio_y();

        let pixel_x = (x as f32 * sym_w / rx) as u16;
        let pixel_y = (y as f32 * sym_h / ry) as u16;
        self.set_pos(pixel_x, pixel_y);
    }

    /// Set sprite position in pixel units (graphics mode semantic).
    ///
    /// This is a semantic helper that makes graphics mode intent explicit.
    /// Accepts floating-point coordinates for sub-pixel precision, then rounds
    /// to integer values for storage.
    ///
    /// In graphics mode: enables pixel-perfect positioning.
    /// In text mode: coordinates are still aligned to character grid.
    ///
    /// # Example
    /// ```ignore
    /// sprite.set_pixel_pos(100.5, 50.25);  // Graphics mode: precise pixel positioning
    /// ```
    pub fn set_pixel_pos(&mut self, x: f32, y: f32) {
        self.set_pos(x.round() as u16, y.round() as u16);
    }

    /// Get sprite position as floating-point pixel coordinates.
    ///
    /// Returns the current position converted to f32 for graphics calculations.
    /// Useful when working with pixel-based positioning in graphics mode.
    ///
    /// # Example
    /// ```ignore
    /// let (px, py) = sprite.pixel_pos();
    /// sprite.set_pixel_pos(px + 10.5, py + 5.25);
    /// ```
    pub fn pixel_pos(&self) -> (f32, f32) {
        (self.content.area.x as f32, self.content.area.y as f32)
    }

}
