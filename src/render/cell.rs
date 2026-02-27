// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Cell is the basic unit for rendering in RustPixel.
//! Each cell stores a character/symbol, foreground color, background color,
//! and texture information for graphics mode.

//! Cell is the basic rendering data structure in RustPixel, represents a char
//! Cell stores some key data such as symbol, tex(graph mode only), fg, bg.
//! Many Cells form a buffer to manage rendering.
//!
//! A buffer comprises a cell vector with width * height elements
//!
//! Symbol mappings are now loaded from JSON configuration (symbol_map.json)
//! via the SymbolMap module for easier maintenance and customization.

use crate::render::style::{Color, Modifier, Style};
use crate::render::symbol_map::get_symbol_map;
use serde::{Deserialize, Serialize};

fn default_scale() -> f32 {
    1.0
}

/// Get TUI symbol index and block
/// Returns (block, index) for TUI region (Block 160-169)
pub fn tui_symidx(symbol: &str) -> Option<(u8, u8)> {
    get_symbol_map().tui_idx(symbol)
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

/// returns a cellsym string by index
/// 256 unicode chars mark the index of a symbol in a SDL texture
/// unicode: 0xE000 ~ 0xE0FF (Private Use Area)
/// maps to a 3 byte UTF8: 11101110 100000xx 10xxxxxx
/// an 8-bits index gets from the UTF8 code is used to mark the offset in its texture
///
/// Using Private Use Area ensures no conflict with standard Unicode characters,
/// allowing applications to display mathematical symbols (∀∃∈∞≈≤≥⊕⊗) or other
/// special characters in TUI mode without interference.
pub fn cellsym(idx: u8) -> String {
    // U+E000 + idx
    let codepoint = 0xE000u32 + idx as u32;
    char::from_u32(codepoint).unwrap().to_string()
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

/// get index idx from a symbol string
/// return idx, if it is a unicode char in Private Use Area (U+E000~U+E0FF)
/// otherwise get index from Sprite symbol map
fn symidx(symbol: &String) -> u8 {
    let sbts = symbol.as_bytes();
    // Private Use Area: U+E000~U+E0FF
    // UTF-8: 11101110 100000xx 10xxxxxx (0xEE 0x80~0x83 0x80~0xBF)
    if sbts.len() == 3 && sbts[0] == 0xEE && (sbts[1] >> 2 == 0x20) {
        let idx = ((sbts[1] & 3) << 6) + (sbts[2] & 0x3f);
        return idx;
    }
    // search in sprite symbol map for common ASCII chars
    if let Some((_, idx)) = get_symbol_map().sprite_idx(symbol) {
        return idx;
    }
    0u8
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    /// Texture block index (0-255) in the 4096x4096 unified texture.
    ///
    /// Block ranges:
    /// - 0-159: Sprite region
    /// - 160-169: TUI region
    /// - 170-175: Emoji region
    /// - 176-239: CJK region
    /// - 240-255: Reserved
    ///
    /// For special characters (Emoji, TUI, CJK), this value is overridden
    /// by `get_cell_info()` based on symbol_map lookups.
    pub tex: u8,
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
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }

    /// Get cell rendering information for graphics mode.
    ///
    /// Returns (symbol_index, block_index, fg, bg, modifier) where:
    /// - symbol_index: Index within the block (0-255)
    /// - block_index: Texture block index (0-255) in the unified 4096x4096 texture
    ///
    /// # Block Index Resolution
    ///
    /// The block_index is determined by priority:
    /// 1. **Emoji** (170-175): Checked first via symbol_map
    /// 2. **CJK** (176-239): Checked second via symbol_map
    /// 3. **Sprite** (0-159): Uses self.tex field as fallback
    ///
    /// # Special Cases
    ///
    /// - Space character in Sprite texture (tex=0): Returns index 32 instead of symbol_map's 33
    ///   to maintain compatibility with texture layout
    ///
    /// # See Also
    ///
    /// - `render::symbol_map` for block layout and character mappings
    /// - `CellInfo` type for return value structure
    pub fn get_cell_info(&self) -> CellInfo {
        // For Sprite texture (tex=0), handle space character specially
        // Sprite symbol map maps space to index 33, but in Sprite texture it should be 32
        if self.tex == 0 && self.symbol == " " {
            return (32, 0, self.fg, self.bg, self.modifier);
        }

        // Check for Emoji first
        if let Some((block, idx)) = get_symbol_map().emoji_idx(&self.symbol) {
            return (idx, block, self.fg, self.bg, self.modifier);
        }

        // Check for CJK characters (returns block 176-207, idx 0-127)
        if let Some((block, idx)) = get_symbol_map().cjk_idx(&self.symbol) {
            return (idx, block, self.fg, self.bg, self.modifier);
        }

        // Check sprite extras for non-zero block (e.g., underscore in block 2)
        // This allows characters to be mapped to different sprite blocks via symbol_map.json
        if self.tex == 0 {
            if let Some((block, idx)) = get_symbol_map().sprite_idx(&self.symbol) {
                if block != 0 {
                    return (idx, block, self.fg, self.bg, self.modifier);
                }
            }
        }

        (symidx(&self.symbol), self.tex, self.fg, self.bg, self.modifier)
    }

    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        self
    }

    /// Set the texture block index for this cell.
    ///
    /// # Arguments
    ///
    /// * `tex` - Block index (0-255) in the unified texture:
    ///   - 0-159: Sprite blocks
    ///   - 160-169: TUI blocks (typically auto-assigned by symbol_map)
    ///   - 170-175: Emoji blocks (typically auto-assigned by symbol_map)
    ///   - 176-239: CJK blocks (typically auto-assigned by symbol_map)
    ///   - 240-255: Reserved
    ///
    /// Note: For special characters (Emoji, TUI, CJK), `get_cell_info()` may
    /// override this value based on symbol_map lookups.
    pub fn set_texture(&mut self, tex: u8) -> &mut Cell {
        self.tex = tex;
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
    /// - symbol: " " (space)
    /// - colors: Reset
    /// - tex: 0 (Sprite block 0)
    /// - modifier: empty
    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.tex = 0; // Block 0 (Sprite region)
        self.modifier = Modifier::empty();
        self.scale_x = 1.0;
        self.scale_y = 1.0;
    }

    /// Check if this cell represents a blank space in graphics mode.
    ///
    /// A cell is considered blank if:
    /// - Symbol is space (" " or cellsym(32))
    /// - Block is 0 or 1 (Sprite region)
    /// - Background is Reset (transparent)
    #[cfg(graphics_mode)]
    pub fn is_blank(&self) -> bool {
        (self.symbol == " " || self.symbol == cellsym(32))
            && (self.tex == 0 || self.tex == 1)
            && self.bg == Color::Reset
    }

    #[cfg(not(graphics_mode))]
    pub fn is_blank(&self) -> bool {
        self.symbol == " " && self.fg == Color::Reset && self.bg == Color::Reset
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell {
            symbol: " ".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            tex: 0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}
