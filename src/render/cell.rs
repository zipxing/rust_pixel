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
use crate::render::symbol_map::SymbolMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
    /// Global symbol map loaded from JSON configuration
    /// Contains mappings for Sprite, TUI, Emoji, and CJK regions
    static ref SYMBOL_MAP: SymbolMap = SymbolMap::default();
}

/// Get TUI symbol index and block
/// Returns (block, index) for TUI region (Block 160-169)
pub fn tui_symidx(symbol: &str) -> Option<(u8, u8)> {
    SYMBOL_MAP.tui_idx(symbol)
}

/// sym_index, texture_index, fg_color, bg_color, modifier
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
    SYMBOL_MAP.emoji_idx(symbol).is_some()
}

/// Get the texture index and symbol index for a pre-rendered Emoji
///
/// Returns Some((block_idx, sym_idx)) if the Emoji is in the mapping,
/// or None if the Emoji is not pre-rendered.
pub fn emoji_texidx(symbol: &str) -> Option<(u8, u8)> {
    SYMBOL_MAP.emoji_idx(symbol)
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
    if let Some((_, idx)) = SYMBOL_MAP.sprite_idx(symbol) {
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
    pub tex: u8,
}

impl Cell {
    pub fn set_symbol(&mut self, symbol: &str) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push_str(symbol);
        self
    }

    /// refers to the comments in buffer.rs, works in graphics mode
    /// returns offset, texture id, colors, and modifier
    ///
    /// maps to a 3 byte UTF8: 11100010 100010xx 10xxxxxx
    /// an 8-digits index gets from the UTF8 code is used to mark the offset in its texture
    ///
    /// refers to the flush method in panel.rs
    ///
    /// sym_index, texture_index, fg_color, bg_color, modifier
    pub fn get_cell_info(&self) -> CellInfo {
        // Check for Emoji first
        if let Some((block, idx)) = SYMBOL_MAP.emoji_idx(&self.symbol) {
            return (idx, block, self.fg, self.bg, self.modifier);
        }

        // For Sprite texture (tex=0), handle space character specially
        // Sprite symbol map maps space to index 33, but in Sprite texture it should be 32
        if self.tex == 0 && self.symbol == " " {
            return (32, 0, self.fg, self.bg, self.modifier);
        }

        (symidx(&self.symbol), self.tex, self.fg, self.bg, self.modifier)
    }

    pub fn set_char(&mut self, ch: char) -> &mut Cell {
        self.symbol.clear();
        self.symbol.push(ch);
        self
    }

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

    pub fn set_style(&mut self, style: Style) -> &mut Cell {
        if let Some(c) = style.fg {
            self.fg = c;
        }
        if let Some(c) = style.bg {
            self.bg = c;
        }
        self.modifier.insert(style.add_modifier);
        self.modifier.remove(style.sub_modifier);
        self
    }

    pub fn style(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg)
            .add_modifier(self.modifier)
    }

    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.tex = 0;
        self.modifier = Modifier::empty();
    }

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
        }
    }
}
