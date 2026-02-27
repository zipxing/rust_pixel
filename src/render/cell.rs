// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Cell is the basic rendering unit in RustPixel.
//! Each cell stores a symbol string, foreground/background colors, modifier flags,
//! and a cached Glyph (block, idx, width, height) for graphics mode.
//!
//! The symbol string fully determines rendering: PUA-encoded for Sprite mode,
//! standard Unicode for TUI mode. Glyph is cached on set_symbol() and read
//! directly during rendering via get_cell_info().
//!
//! Many Cells form a Buffer to manage rendering.
//! Symbol mappings are loaded from symbol_map.json via the SymbolMap module.

use crate::render::style::{Color, Modifier, Style};
use crate::render::symbol_map::get_symbol_map;
use serde::{Deserialize, Serialize};

fn default_scale() -> f32 {
    1.0
}

/// Cell rendering information: (symbol_index, block_index, fg_color, bg_color, modifier)
///
/// - symbol_index (u8): Index within the block (0-255)
/// - block_index (u8): Texture block index (0-255):
///   - 0-159: Sprite blocks
///   - 160-169: TUI blocks
///   - 170-175: Emoji blocks
///   - 176-239: CJK blocks
///   - 240-255: Reserved
/// - fg_color: Foreground color
/// - bg_color: Background color
/// - modifier: Text modifiers (bold, italic, etc.)
pub type CellInfo = (u8, u8, Color, Color, Modifier);

/// Glyph rendering information with size metadata.
///
/// This struct directly expresses character rendering info without needing
/// block range checks. The width and height are in multiples of base cell size:
/// - PIXEL_SYM_WIDTH (16 pixels) for width
/// - PIXEL_SYM_HEIGHT (16 pixels) for height
///
/// Size conventions:
/// - Sprite (block 0-159): 1x1 (16x16 pixels)
/// - TUI (block 160-169): 1x2 (16x32 pixels)
/// - Emoji (block 170-175): 2x2 (32x32 pixels)
/// - CJK (block 176-239): 2x2 (32x32 pixels)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    /// Texture block index (0-255)
    pub block: u8,
    /// Symbol index within the block (0-255)
    pub idx: u8,
    /// Width in multiples of PIXEL_SYM_WIDTH (1 or 2)
    pub width: u8,
    /// Height in multiples of PIXEL_SYM_HEIGHT (1 or 2)
    pub height: u8,
}

impl Glyph {
    /// Create a new Glyph with explicit dimensions.
    pub fn new(block: u8, idx: u8, width: u8, height: u8) -> Self {
        Self { block, idx, width, height }
    }

    /// Create a Sprite glyph (1x1, 16x16 pixels).
    pub fn sprite(block: u8, idx: u8) -> Self {
        Self { block, idx, width: 1, height: 1 }
    }

    /// Create a TUI glyph (1x2, 16x32 pixels).
    pub fn tui(block: u8, idx: u8) -> Self {
        Self { block, idx, width: 1, height: 2 }
    }

    /// Create an Emoji glyph (2x2, 32x32 pixels).
    pub fn emoji(block: u8, idx: u8) -> Self {
        Self { block, idx, width: 2, height: 2 }
    }

    /// Create a CJK glyph (2x2, 32x32 pixels).
    pub fn cjk(block: u8, idx: u8) -> Self {
        Self { block, idx, width: 2, height: 2 }
    }

    /// Check if this is a double-height glyph.
    pub fn is_double_height(&self) -> bool {
        self.height == 2
    }

    /// Check if this is a double-width glyph.
    pub fn is_double_width(&self) -> bool {
        self.width == 2
    }
}

impl Default for Glyph {
    fn default() -> Self {
        // Default: space character in Sprite mode (block 0, idx 32)
        Glyph::sprite(0, 32)
    }
}

/// PUA (Private Use Area) encoding for Sprite symbols.
///
/// Uses Supplementary PUA-A (U+F0000-U+F9FFF) to support all 160 sprite blocks:
/// - Block 0: U+F0000-U+F00FF
/// - Block 1: U+F0100-U+F01FF
/// - ...
/// - Block 159: U+F9F00-U+F9FFF
///
/// Encoding: codepoint = 0xF0000 + block * 256 + idx
pub const PUA_BASE: u32 = 0xF0000;
pub const PUA_END: u32 = 0xF9FFF;  // 160 blocks × 256 = 40960
pub const PUA_BLOCK_SIZE: u32 = 256;

/// Encode block and index to PUA character string.
///
/// # Arguments
/// * `block` - Block index (0-159 for sprite region)
/// * `idx` - Symbol index within block (0-255)
///
/// # Returns
/// A String containing a single Supplementary PUA-A character.
pub fn cellsym_block(block: u8, idx: u8) -> String {
    debug_assert!((block as u32) < 160, "block must be 0-159, got {}", block);
    let codepoint = PUA_BASE + (block as u32) * PUA_BLOCK_SIZE + idx as u32;
    char::from_u32(codepoint).unwrap().to_string()
}

/// Convenience function for block 0 (backward compatible).
///
/// Returns a cellsym string by index in block 0.
/// unicode: U+F0000 ~ U+F00FF (Supplementary Private Use Area-A)
///
/// Using Supplementary PUA-A ensures no conflict with standard Unicode characters,
/// including BMP PUA (U+E000-U+F8FF) used by NerdFont/Powerline.
pub fn cellsym(idx: u8) -> String {
    cellsym_block(0, idx)
}

/// Decode a Supplementary PUA-A character to (block, idx).
///
/// # Arguments
/// * `ch` - A character to decode
///
/// # Returns
/// Some((block, idx)) if the character is in PUA range (U+F0000-U+F9FFF),
/// None otherwise.
pub fn decode_pua(ch: char) -> Option<(u8, u8)> {
    let cp = ch as u32;
    if cp >= PUA_BASE && cp <= PUA_END {
        let offset = cp - PUA_BASE;
        let block = (offset / PUA_BLOCK_SIZE) as u8;
        let idx = (offset % PUA_BLOCK_SIZE) as u8;
        Some((block, idx))
    } else {
        None
    }
}

/// Check if a character is in PUA Sprite range.
pub fn is_pua_sprite(ch: char) -> bool {
    let cp = ch as u32;
    cp >= PUA_BASE && cp <= PUA_END
}

/// TUI character type for rendering mode detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TuiCharType {
    /// ASCII and standard TUI characters (Box Drawing, Block Elements, Braille, NerdFont)
    TuiChar,
    /// Pre-rendered Emoji
    Emoji,
    /// CJK characters
    CJK,
}

/// Check if a codepoint is a TUI character (ASCII, Box Drawing, Block Elements, Braille, NerdFont).
pub fn is_tui_char(cp: u32) -> bool {
    (0x0020..=0x007E).contains(&cp) ||    // ASCII printable
    (0x2500..=0x257F).contains(&cp) ||    // Box Drawing
    (0x2580..=0x259F).contains(&cp) ||    // Block Elements
    (0x2800..=0x28FF).contains(&cp) ||    // Braille Patterns
    (0xE000..=0xF8FF).contains(&cp)          // BMP PUA: NerdFont / Powerline (our PUA is in Supplementary PUA-A)
}

/// Check if a codepoint is CJK.
pub fn is_cjk(cp: u32) -> bool {
    (0x4E00..=0x9FFF).contains(&cp) ||    // CJK Unified Ideographs
    (0x3400..=0x4DBF).contains(&cp) ||    // CJK Unified Ideographs Extension A
    (0x20000..=0x2A6DF).contains(&cp) ||  // CJK Unified Ideographs Extension B
    (0x2A700..=0x2B73F).contains(&cp) ||  // CJK Unified Ideographs Extension C
    (0x2B740..=0x2B81F).contains(&cp) ||  // CJK Unified Ideographs Extension D
    (0x3000..=0x303F).contains(&cp) ||    // CJK Symbols and Punctuation
    (0xFF00..=0xFFEF).contains(&cp)       // Halfwidth and Fullwidth Forms
}

/// Detect TUI character type from a symbol string.
///
/// Returns the character type for TUI mode rendering.
pub fn detect_tui_char_type(symbol: &str) -> TuiCharType {
    // Check Emoji first (may be multi-codepoint)
    if is_prerendered_emoji(symbol) {
        return TuiCharType::Emoji;
    }

    // Check first codepoint
    if let Some(ch) = symbol.chars().next() {
        let cp = ch as u32;
        if is_cjk(cp) {
            return TuiCharType::CJK;
        }
    }

    TuiCharType::TuiChar
}

/// Check if a symbol is a pre-rendered Emoji
///
/// Returns true if the symbol exists in the Emoji mapping, meaning it has
/// a pre-rendered 32x32 RGBA image in the Emoji region of the texture.
pub fn is_prerendered_emoji(symbol: &str) -> bool {
    get_symbol_map().emoji_idx(symbol).is_some()
}

/// Get the texture index and symbol index for a pre-rendered Emoji
///
/// Returns Some((block_idx, sym_idx)) if the Emoji is in the mapping,
/// or None if the Emoji is not pre-rendered.
pub fn emoji_texidx(symbol: &str) -> Option<(u8, u8)> {
    get_symbol_map().emoji_idx(symbol)
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Symbol string determining the character to render.
    ///
    /// In Sprite mode: PUA encoded (U+F0000-U+F9FFF), block and idx derived from codepoint.
    /// In TUI mode: Standard Unicode (ASCII, Box Drawing, Emoji, CJK), mapped via symbol_map.
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    /// Per-cell X scale factor (1.0 = no scaling).
    /// Combined with sprite-level scale in graphics mode rendering.
    /// When different from 1.0, triggers cumulative width layout.
    #[serde(default = "default_scale")]
    pub scale_x: f32,
    /// Per-cell Y scale factor (1.0 = no scaling).
    /// Combined with sprite-level scale in graphics mode rendering.
    /// Cells are vertically centered within the row when scale differs.
    #[serde(default = "default_scale")]
    pub scale_y: f32,
    /// Cached glyph info (graphics mode only).
    /// Computed from symbol when set, returned directly by get_glyph().
    #[cfg(graphics_mode)]
    #[serde(skip)]
    glyph: Glyph,
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        #[cfg(graphics_mode)]
        {
            self.glyph = self.compute_glyph();
        }
        self
    }

    /// Compute glyph from symbol (graphics mode only).
    /// Called internally when symbol changes.
    #[cfg(graphics_mode)]
    fn compute_glyph(&self) -> Glyph {
        // 1. Check PUA (Sprite symbols) - 1x1
        if let Some(ch) = self.symbol.chars().next() {
            if let Some((block, idx)) = decode_pua(ch) {
                return Glyph::sprite(block, idx);
            }
        }

        // 2. Check Emoji - 2x2
        if let Some((block, idx)) = get_symbol_map().emoji_idx(&self.symbol) {
            return Glyph::emoji(block, idx);
        }

        // 3. Check CJK - 2x2
        if let Some((block, idx)) = get_symbol_map().cjk_idx(&self.symbol) {
            return Glyph::cjk(block, idx);
        }

        // 4. Check TUI (includes ASCII, Box Drawing, Braille, etc.) - 1x2
        if let Some((block, idx)) = get_symbol_map().tui_idx(&self.symbol) {
            return Glyph::tui(block, idx);
        }

        // Fallback: space character (Sprite 1x1)
        Glyph::sprite(0, 32)
    }

    /// Get cell rendering information for graphics mode.
    ///
    /// Returns (symbol_index, block_index, fg, bg, modifier) where:
    /// - symbol_index: Index within the block (0-255)
    /// - block_index: Texture block index (0-255) in the unified 4096x4096 texture
    ///
    /// Block is determined solely from symbol:
    /// - PUA (U+F0000-U+F9FFF): Sprite blocks 0-159
    /// - Emoji: Emoji blocks 170-175
    /// - CJK: CJK blocks 176-239
    /// - TUI chars: TUI blocks 160-169
    ///
    /// # See Also
    ///
    /// - `get_glyph()` for the full Glyph with size info
    /// - `render::symbol_map` for block layout and character mappings
    #[cfg(graphics_mode)]
    pub fn get_cell_info(&self) -> CellInfo {
        let glyph = self.glyph;
        (glyph.idx, glyph.block, self.fg, self.bg, self.modifier)
    }

    /// Get glyph rendering information (graphics mode only).
    ///
    /// Returns the cached glyph computed when symbol was set.
    /// This is O(1) and avoids repeated HashMap lookups during rendering.
    ///
    /// Returns Glyph with block, idx, width, and height.
    #[cfg(graphics_mode)]
    pub fn get_glyph(&self) -> Glyph {
        self.glyph
    }

    /// Get glyph info as (block, idx) tuple (graphics mode only).
    #[cfg(graphics_mode)]
    pub fn get_glyph_info(&self) -> (u8, u8) {
        (self.glyph.block, self.glyph.idx)
    }

    /// Get cell rendering information (non-graphics mode fallback).
    /// Computes glyph on the fly. Only used for serialization.
    #[cfg(not(graphics_mode))]
    pub fn get_cell_info(&self) -> CellInfo {
        // Compute glyph on the fly for non-graphics mode
        let (block, idx) = self.compute_glyph_fallback();
        (idx, block, self.fg, self.bg, self.modifier)
    }

    /// Compute glyph without symbol_map (non-graphics mode fallback).
    /// Only handles PUA decoding, returns default for other symbols.
    #[cfg(not(graphics_mode))]
    fn compute_glyph_fallback(&self) -> (u8, u8) {
        if let Some(ch) = self.symbol.chars().next() {
            if let Some((block, idx)) = decode_pua(ch) {
                return (block, idx);
            }
        }
        (0, 32)  // Default: space
    }

    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        #[cfg(graphics_mode)]
        {
            self.glyph = self.compute_glyph();
        }
        self
    }

    /// Set the texture block for this cell (Sprite mode only, graphics mode only).
    ///
    /// This updates the symbol to PUA encoding with the given block.
    /// The symbol index is preserved from the current symbol.
    ///
    /// # Arguments
    ///
    /// * `block` - Block index (0-3 for Sprite region)
    ///
    /// # Note
    ///
    /// This method is primarily for Sprite mode. For TUI/Emoji/CJK,
    /// the block is determined automatically from the symbol.
    #[cfg(graphics_mode)]
    pub fn set_texture(&mut self, block: u8) -> &mut Cell {
        // Get current idx from cached glyph
        let idx = self.glyph.idx;
        // Set symbol to PUA with new block
        self.symbol = cellsym_block(block, idx);
        // Update cached glyph
        self.glyph = Glyph::sprite(block, idx);
        self
    }

    pub fn set_fg(&mut self, color: Color) -> &mut Cell {
        self.fg = color;
        self
    }

    pub fn set_bg(&mut self, color: Color) -> &mut Cell {
        self.bg = color;
        self
    }

    /// Set per-cell scale factors.
    /// Combined with sprite-level scale during rendering:
    /// final_scale = sprite_scale * cell_scale
    pub fn set_scale(&mut self, sx: f32, sy: f32) -> &mut Cell {
        self.scale_x = sx;
        self.scale_y = sy;
        self
    }

    /// Set uniform per-cell scale (same for both axes).
    pub fn set_scale_uniform(&mut self, s: f32) -> &mut Cell {
        self.scale_x = s;
        self.scale_y = s;
        self
    }

    pub fn set_style(&mut self, style: Style) -> &mut Cell {
        if let Some(c) = style.fg {
            self.fg = c;
        }
        if let Some(c) = style.bg {
            self.bg = c;
        }
        self.modifier.insert(style.add_modifier);
        self.modifier.remove(style.sub_modifier);
        if let Some(sx) = style.scale_x {
            self.scale_x = sx;
        }
        if let Some(sy) = style.scale_y {
            self.scale_y = sy;
        }
        self
    }

    pub fn style(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg)
            .add_modifier(self.modifier)
            .scale(self.scale_x, self.scale_y)
    }

    /// Reset cell to blank state (space character).
    ///
    /// Sets:
    /// - symbol: " " (space, TUI mode) or PUA space (Sprite mode)
    /// - colors: Reset
    /// - modifier: empty
    /// - scale: 1.0
    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.modifier = Modifier::empty();
        self.scale_x = 1.0;
        self.scale_y = 1.0;
        #[cfg(graphics_mode)]
        {
            self.glyph = self.compute_glyph();
        }
    }

    /// Reset cell to blank state for Sprite mode.
    ///
    /// Uses PUA space character (block 0, idx 32).
    pub fn reset_sprite(&mut self) {
        self.symbol = cellsym(32);  // PUA space
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.modifier = Modifier::empty();
        self.scale_x = 1.0;
        self.scale_y = 1.0;
        #[cfg(graphics_mode)]
        {
            self.glyph = Glyph::sprite(0, 32);
        }
    }

    /// Check if this cell represents a blank space in graphics mode.
    ///
    /// A cell is considered blank if:
    /// - Symbol is space (" " or PUA space in block 0-1)
    /// - Background is Reset (transparent)
    #[cfg(graphics_mode)]
    pub fn is_blank(&self) -> bool {
        let is_space = self.symbol == " " || {
            if let Some(ch) = self.symbol.chars().next() {
                if let Some((block, idx)) = decode_pua(ch) {
                    idx == 32 && block <= 1
                } else {
                    false
                }
            } else {
                true // empty symbol is blank
            }
        };
        is_space && self.bg == Color::Reset
    }

    #[cfg(not(graphics_mode))]
    pub fn is_blank(&self) -> bool {
        self.symbol == " " && self.fg == Color::Reset && self.bg == Color::Reset
    }
}

impl Default for Cell {
    #[cfg(graphics_mode)]
    fn default() -> Cell {
        Cell {
            symbol: " ".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            scale_x: 1.0,
            scale_y: 1.0,
            glyph: Glyph::default(),
        }
    }

    #[cfg(not(graphics_mode))]
    fn default() -> Cell {
        Cell {
            symbol: " ".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}
