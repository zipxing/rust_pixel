// RustPixel
// copyright zipxing@hotmail.com 2022～2026

//! Cell is the basic rendering unit in RustPixel.
//! Each cell stores a symbol string, foreground/background colors, modifier flags,
//! and a cached Tile (cell_w, cell_h, mips[3]) for graphics mode.
//!
//! The symbol string fully determines rendering: PUA-encoded for Sprite mode,
//! standard Unicode for TUI mode. Tile is cached on set_symbol() and read
//! directly during rendering via get_tile().
//!
//! Many Cells form a Buffer to manage rendering.
//! Symbol mappings are loaded from layered_symbol_map.json via the LayeredSymbolMap.

use crate::render::style::{Color, Modifier, Style};
#[cfg(graphics_mode)]
use crate::render::symbol_map::{Tile, get_layered_symbol_map};
use serde::{Deserialize, Serialize};

fn default_scale() -> f32 {
    1.0
}

/// Cell rendering information: (symbol_index, block_index, fg_color, bg_color, modifier)
///
/// Used for .pix file serialization. Block/idx are computed on-demand from the symbol string.
pub type CellInfo = (u8, u8, Color, Color, Modifier);

/// PUA (Private Use Area) encoding for Sprite/UI symbols.
///
/// Uses Supplementary PUA-A (U+F0000-U+FFFFD).
/// Encoding: codepoint = 0xF0000 + block * 256 + idx
pub const PUA_BASE: u32 = 0xF0000;
pub const PUA_END: u32 = 0xFFFFD;
pub const PUA_BLOCK_SIZE: u32 = 256;

/// Encode block and index to PUA character string.
///
/// # Arguments
/// * `block` - Block index (0-255)
/// * `idx` - Symbol index within block (0-255)
///
/// # Returns
/// A String containing a single Supplementary PUA-A character.
pub fn cellsym_block(block: u8, idx: u8) -> String {
    let codepoint = PUA_BASE + (block as u32) * PUA_BLOCK_SIZE + idx as u32;
    debug_assert!(
        codepoint <= PUA_END,
        "invalid PUA codepoint U+{:X} from block={}, idx={}",
        codepoint,
        block,
        idx
    );
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
/// Some((block, idx)) if the character is in PUA range (U+F0000-U+FFFFD),
/// None otherwise.
pub fn decode_pua(ch: char) -> Option<(u8, u8)> {
    let cp = ch as u32;
    if (PUA_BASE..=PUA_END).contains(&cp) {
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
    (PUA_BASE..=PUA_END).contains(&cp)
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
/// Returns true if the symbol is in the Emoji Unicode range AND exists
/// in the LayeredSymbolMap (meaning it has a pre-rendered image).
pub fn is_prerendered_emoji(symbol: &str) -> bool {
    if let Some(ch) = symbol.chars().next() {
        let cp = ch as u32;
        // Emoji Unicode ranges:
        // - U+1F000-U+1FAFF: Main emoji block (emoticons, symbols, hands 👉, etc.)
        // - U+2300-U+23FF: Miscellaneous Technical (⏰⌛ etc.)
        // - U+2600-U+26FF: Miscellaneous Symbols (⚓⚡⚽⛵ etc.)
        // - U+2700-U+27BF: Dingbats (✅✌✏ etc.)
        // - U+2B00-U+2BFF: Miscellaneous Symbols and Arrows (⭐⬛⬜ etc.)
        let is_emoji_range = (0x1F000..=0x1FAFF).contains(&cp)
            || (0x2300..=0x23FF).contains(&cp)
            || (0x2600..=0x26FF).contains(&cp)
            || (0x2700..=0x27BF).contains(&cp)
            || (0x2B00..=0x2BFF).contains(&cp);

        if is_emoji_range {
            #[cfg(graphics_mode)]
            {
                return get_layered_symbol_map()
                    .map(|m| m.contains(symbol))
                    .unwrap_or(false);
            }
            #[cfg(not(graphics_mode))]
            {
                return true; // Assume emoji is renderable in text mode
            }
        }
    }
    false
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
    /// Cached tile info (graphics mode only).
    /// Resolved from LayeredSymbolMap when symbol is set.
    /// Contains cell dimensions and 3 mipmap levels of UV + layer data.
    #[cfg(graphics_mode)]
    #[serde(skip)]
    tile: Tile,
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        #[cfg(graphics_mode)]
        {
            self.tile = self.compute_tile();
        }
        self
    }

    /// Compute tile from symbol (graphics mode only).
    /// Resolves the symbol string in LayeredSymbolMap to get UV + layer data.
    #[cfg(graphics_mode)]
    fn compute_tile(&self) -> Tile {
        get_layered_symbol_map()
            .map(|m| *m.resolve(&self.symbol))
            .unwrap_or_default()
    }

    /// Get cell rendering information for graphics mode.
    ///
    /// Returns (symbol_index, block_index, fg, bg, modifier).
    /// Block/idx are computed on-demand from the symbol string for .pix serialization.
    #[cfg(graphics_mode)]
    pub fn get_cell_info(&self) -> CellInfo {
        let (block, idx) = self.compute_block_idx();
        (idx, block, self.fg, self.bg, self.modifier)
    }

    /// Get cached tile (graphics mode only).
    ///
    /// Returns the Tile resolved from LayeredSymbolMap when symbol was set.
    /// Contains cell_w, cell_h, and 3 mipmap levels of UV + layer data.
    #[cfg(graphics_mode)]
    pub fn get_tile(&self) -> Tile {
        self.tile
    }

    /// Compute (block, idx) from symbol string (for .pix serialization).
    /// This is NOT on the hot render path — only called for file I/O.
    #[cfg(graphics_mode)]
    fn compute_block_idx(&self) -> (u8, u8) {
        // 1. Check PUA (Sprite symbols)
        if let Some(ch) = self.symbol.chars().next() {
            if let Some((block, idx)) = decode_pua(ch) {
                return (block, idx);
            }
        }
        // 2. Non-PUA: use LayeredSymbolMap reverse lookup
        if let Some(map) = get_layered_symbol_map() {
            if let Some((block, idx)) = map.reverse_lookup(&self.symbol) {
                return (block, idx);
            }
        }
        (0, 32)  // Default: space
    }

    /// Get cell rendering information (non-graphics mode fallback).
    /// Computes block/idx on the fly. Only used for serialization.
    #[cfg(not(graphics_mode))]
    pub fn get_cell_info(&self) -> CellInfo {
        let (block, idx) = self.compute_block_idx_fallback();
        (idx, block, self.fg, self.bg, self.modifier)
    }

    /// Compute block/idx without symbol_map (non-graphics mode fallback).
    /// Only handles PUA decoding, returns default for other symbols.
    #[cfg(not(graphics_mode))]
    fn compute_block_idx_fallback(&self) -> (u8, u8) {
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
            self.tile = self.compute_tile();
        }
        self
    }

    /// Set the texture block for this cell (Sprite mode only, graphics mode only).
    ///
    /// This updates the symbol to PUA encoding with the given block.
    /// The symbol index is preserved from the current PUA symbol.
    #[cfg(graphics_mode)]
    pub fn set_texture(&mut self, block: u8) -> &mut Cell {
        // Get current idx from PUA symbol
        let idx = self.symbol.chars().next()
            .and_then(decode_pua)
            .map(|(_, i)| i)
            .unwrap_or(32);
        // Set symbol to PUA with new block
        self.symbol = cellsym_block(block, idx);
        // Update cached tile
        self.tile = self.compute_tile();
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
            self.tile = self.compute_tile();
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
            self.tile = self.compute_tile();
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
            tile: Tile::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ====================================================================
    // PUA encoding/decoding round-trip tests
    // ====================================================================

    #[test]
    fn test_pua_roundtrip_block0() {
        for idx in [0u8, 1, 127, 128, 255] {
            let encoded = cellsym_block(0, idx);
            let ch = encoded.chars().next().unwrap();
            let (block, decoded_idx) = decode_pua(ch).expect("should decode");
            assert_eq!(block, 0);
            assert_eq!(decoded_idx, idx);
        }
    }

    #[test]
    fn test_pua_roundtrip_all_blocks() {
        // Test representative blocks across the range
        for block in [0u8, 1, 50, 100, 159, 160, 200, 254] {
            for idx in [0u8, 128, 255] {
                let encoded = cellsym_block(block, idx);
                let ch = encoded.chars().next().unwrap();
                let (dec_block, dec_idx) = decode_pua(ch).expect("should decode");
                assert_eq!(dec_block, block, "block mismatch for ({}, {})", block, idx);
                assert_eq!(dec_idx, idx, "idx mismatch for ({}, {})", block, idx);
            }
        }
    }

    #[test]
    fn test_pua_decode_non_pua() {
        // Normal ASCII should not decode as PUA
        assert_eq!(decode_pua('A'), None);
        assert_eq!(decode_pua(' '), None);
        // BMP PUA should not decode as Supplementary PUA
        assert_eq!(decode_pua('\u{E000}'), None);
        // Emoji should not decode
        assert_eq!(decode_pua('😀'), None);
    }

    #[test]
    fn test_pua_codepoint_range() {
        // First PUA character: U+F0000
        let first = cellsym_block(0, 0);
        let cp = first.chars().next().unwrap() as u32;
        assert_eq!(cp, PUA_BASE);

        // Last valid full-block codepoint before partial tail: block=255, idx=253 → U+FFFFD
        let last = cellsym_block(255, 253);
        let cp = last.chars().next().unwrap() as u32;
        assert_eq!(cp, PUA_END);
    }

    #[test]
    fn test_cellsym_convenience() {
        // cellsym(idx) == cellsym_block(0, idx)
        for idx in [0u8, 42, 160, 255] {
            assert_eq!(cellsym(idx), cellsym_block(0, idx));
        }
    }

    #[test]
    fn test_is_pua_sprite() {
        let pua_ch = cellsym_block(0, 0).chars().next().unwrap();
        assert!(is_pua_sprite(pua_ch));
        assert!(!is_pua_sprite('A'));
        assert!(!is_pua_sprite('😀'));
    }

    // ====================================================================
    // TuiCharType detection tests
    // ====================================================================

    #[test]
    fn test_tui_char_type_detection() {
        // ASCII characters
        assert_eq!(detect_tui_char_type("A"), TuiCharType::TuiChar);
        assert_eq!(detect_tui_char_type(" "), TuiCharType::TuiChar);

        // Box drawing characters (U+2500-U+257F)
        assert_eq!(detect_tui_char_type("─"), TuiCharType::TuiChar);
        assert_eq!(detect_tui_char_type("│"), TuiCharType::TuiChar);

        // CJK characters
        assert_eq!(detect_tui_char_type("中"), TuiCharType::CJK);
        assert_eq!(detect_tui_char_type("一"), TuiCharType::CJK);
    }
}
