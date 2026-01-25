//! Unified Symbol Map Configuration
//!
//! This module provides a JSON-based configuration system for mapping characters
//! to texture coordinates across different regions (Sprite, TUI, Emoji, CJK).
//!
//! # Regions
//! - **Sprite** (Block 0-159): 16x16 game sprites, single color
//! - **TUI** (Block 160-169): 16x32 terminal UI characters, single color
//! - **Emoji** (Block 170-175): 32x32 color emoji
//! - **CJK** (y=3072-4095): 32x32 Chinese characters, single color
//!
//! # Usage
//! ```rust
//! let symbol_map = SymbolMap::default();
//! match symbol_map.lookup("😀") {
//!     SymbolIndex::Emoji(block, idx) => { /* render emoji */ }
//!     _ => { /* fallback */ }
//! }
//! ```

use serde::Deserialize;
use std::collections::HashMap;

/// Symbol index result from lookup
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolIndex {
    /// Sprite region: (block, index) - Block 0-159
    Sprite(u8, u8),
    /// TUI region: (block, index) - Block 160-169
    Tui(u8, u8),
    /// Emoji region: (block, index) - Block 170-175
    Emoji(u8, u8),
    /// CJK region: (pixel_x, pixel_y) - Direct texture coordinates
    Cjk(u16, u16),
    /// Symbol not found in any region
    NotFound,
}

/// Unified symbol mapping table
///
/// Loads character-to-texture mappings from JSON configuration.
/// Supports four texture regions: Sprite, TUI, Emoji, and CJK.
pub struct SymbolMap {
    sprite: HashMap<String, (u8, u8)>,
    tui: HashMap<String, (u8, u8)>,
    emoji: HashMap<String, (u8, u8)>,
    cjk: HashMap<char, (u16, u16)>,
}

impl SymbolMap {
    /// Load symbol map from JSON file
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Parse symbol map from JSON string
    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: SymbolMapConfig = serde_json::from_str(json)?;
        Ok(Self::from_config(config))
    }

    /// Load from embedded default configuration (backward compatible)
    pub fn default_map() -> Self {
        let json = include_str!("../../assets/pix/symbol_map.json");
        Self::from_json(json).expect("Invalid embedded symbol_map.json")
    }

    /// Create empty symbol map
    pub fn empty() -> Self {
        Self {
            sprite: HashMap::new(),
            tui: HashMap::new(),
            emoji: HashMap::new(),
            cjk: HashMap::new(),
        }
    }

    fn from_config(config: SymbolMapConfig) -> Self {
        let mut sprite = HashMap::new();
        let mut tui = HashMap::new();
        let mut emoji = HashMap::new();
        let mut cjk = HashMap::new();

        // Parse Sprite region
        if let Some(region) = config.regions.get("sprite") {
            Self::parse_block_region(region, &mut sprite);
        }

        // Parse TUI region
        if let Some(region) = config.regions.get("tui") {
            Self::parse_block_region(region, &mut tui);
        }

        // Parse Emoji region
        if let Some(region) = config.regions.get("emoji") {
            Self::parse_block_region(region, &mut emoji);
        }

        // Parse CJK region
        if let Some(region) = config.regions.get("cjk") {
            Self::parse_grid_region(region, &mut cjk);
        }

        Self { sprite, tui, emoji, cjk }
    }

    fn parse_block_region(region: &RegionConfig, map: &mut HashMap<String, (u8, u8)>) {
        let block_start = region.block_range.as_ref().map(|r| r[0]).unwrap_or(0);
        let chars_per_block = region.chars_per_block.unwrap_or(256) as u8;

        // Parse symbols string or array
        if let Some(symbols) = &region.symbols {
            let mut block = block_start;
            let mut idx = 0u8;

            match symbols {
                SymbolsValue::String(s) => {
                    for ch in s.chars() {
                        map.insert(ch.to_string(), (block, idx));
                        idx = idx.wrapping_add(1);
                        if idx == 0 {
                            // Wrapped around, move to next block
                            block += 1;
                        } else if idx == chars_per_block {
                            idx = 0;
                            block += 1;
                        }
                    }
                }
                SymbolsValue::Array(arr) => {
                    for s in arr {
                        map.insert(s.clone(), (block, idx));
                        idx = idx.wrapping_add(1);
                        if idx == 0 {
                            block += 1;
                        } else if idx == chars_per_block {
                            idx = 0;
                            block += 1;
                        }
                    }
                }
            }
        }

        // Parse extras (explicit mappings)
        if let Some(extras) = &region.extras {
            for (ch, coords) in extras {
                map.insert(ch.clone(), (coords[0], coords[1]));
            }
        }
    }

    fn parse_grid_region(region: &RegionConfig, map: &mut HashMap<char, (u16, u16)>) {
        let pixel_region = region.pixel_region.as_ref();
        let char_size = region.char_size.as_ref();

        if let (Some(pr), Some(cs)) = (pixel_region, char_size) {
            let base_x = pr[0] as u16;
            let base_y = pr[1] as u16;
            let char_w = cs[0] as u16;
            let char_h = cs[1] as u16;

            if let Some(mappings) = &region.mappings {
                for (ch, coords) in mappings {
                    if let Some(c) = ch.chars().next() {
                        let pixel_x = base_x + coords[0] as u16 * char_w;
                        let pixel_y = base_y + coords[1] as u16 * char_h;
                        map.insert(c, (pixel_x, pixel_y));
                    }
                }
            }
        }
    }

    /// Query Sprite region symbol
    /// Returns (block, index) if found
    pub fn sprite_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        self.sprite.get(symbol).copied()
    }

    /// Query TUI region symbol
    /// Returns (block, index) if found
    pub fn tui_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        self.tui.get(symbol).copied()
    }

    /// Query Emoji region symbol
    /// Returns (block, index) if found
    pub fn emoji_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        self.emoji.get(symbol).copied()
    }

    /// Query CJK region symbol
    /// Returns (pixel_x, pixel_y) texture coordinates if found
    pub fn cjk_coords(&self, ch: char) -> Option<(u16, u16)> {
        self.cjk.get(&ch).copied()
    }

    /// Query CJK region symbol and return (block, index) format
    /// Compatible with calc_linear_index for rendering
    /// Returns (block, idx) where block is 176-207 (row) and idx is 0-127 (column)
    pub fn cjk_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        let ch = symbol.chars().next()?;
        let (pixel_x, pixel_y) = self.cjk.get(&ch).copied()?;

        // Convert pixel coordinates to grid position
        // CJK region starts at y=3072, each cell is 32x32 pixels
        let col = pixel_x / 32;  // 0-127
        let row = (pixel_y - 3072) / 32;  // 0-31

        // Convert to block/index format
        // block = CJK_BLOCK_START + row = 176 + row
        // idx = col
        let block = (layout::CJK_BLOCK_START as u16 + row) as u8;
        let idx = col as u8;

        Some((block, idx))
    }

    /// Add CJK character mapping
    /// Used by CJK tool to update mappings at runtime
    pub fn add_cjk(&mut self, ch: char, pixel_x: u16, pixel_y: u16) {
        self.cjk.insert(ch, (pixel_x, pixel_y));
    }

    /// Unified lookup interface
    /// Priority: Emoji > TUI > Sprite > CJK
    pub fn lookup(&self, symbol: &str) -> SymbolIndex {
        // Check Emoji first (typically multi-byte)
        if let Some((block, idx)) = self.emoji.get(symbol) {
            return SymbolIndex::Emoji(*block, *idx);
        }

        // Check TUI
        if let Some((block, idx)) = self.tui.get(symbol) {
            return SymbolIndex::Tui(*block, *idx);
        }

        // Check Sprite
        if let Some((block, idx)) = self.sprite.get(symbol) {
            return SymbolIndex::Sprite(*block, *idx);
        }

        // Check CJK (single character lookup)
        if let Some(ch) = symbol.chars().next() {
            if let Some((x, y)) = self.cjk.get(&ch) {
                return SymbolIndex::Cjk(*x, *y);
            }
        }

        SymbolIndex::NotFound
    }

    /// Lookup with region hint for better performance
    /// Use this when you know which region to check
    pub fn lookup_in_region(&self, symbol: &str, region: SymbolRegion) -> SymbolIndex {
        match region {
            SymbolRegion::Sprite => {
                self.sprite.get(symbol)
                    .map(|(b, i)| SymbolIndex::Sprite(*b, *i))
                    .unwrap_or(SymbolIndex::NotFound)
            }
            SymbolRegion::Tui => {
                self.tui.get(symbol)
                    .map(|(b, i)| SymbolIndex::Tui(*b, *i))
                    .unwrap_or(SymbolIndex::NotFound)
            }
            SymbolRegion::Emoji => {
                self.emoji.get(symbol)
                    .map(|(b, i)| SymbolIndex::Emoji(*b, *i))
                    .unwrap_or(SymbolIndex::NotFound)
            }
            SymbolRegion::Cjk => {
                if let Some(ch) = symbol.chars().next() {
                    self.cjk.get(&ch)
                        .map(|(x, y)| SymbolIndex::Cjk(*x, *y))
                        .unwrap_or(SymbolIndex::NotFound)
                } else {
                    SymbolIndex::NotFound
                }
            }
        }
    }

    /// Get statistics about loaded symbols
    pub fn stats(&self) -> SymbolMapStats {
        SymbolMapStats {
            sprite_count: self.sprite.len(),
            tui_count: self.tui.len(),
            emoji_count: self.emoji.len(),
            cjk_count: self.cjk.len(),
        }
    }
}

impl Default for SymbolMap {
    fn default() -> Self {
        Self::default_map()
    }
}

/// Symbol region hint for optimized lookup
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolRegion {
    Sprite,
    Tui,
    Emoji,
    Cjk,
}

/// Statistics about loaded symbol mappings
#[derive(Debug, Clone)]
pub struct SymbolMapStats {
    pub sprite_count: usize,
    pub tui_count: usize,
    pub emoji_count: usize,
    pub cjk_count: usize,
}

impl SymbolMapStats {
    pub fn total(&self) -> usize {
        self.sprite_count + self.tui_count + self.emoji_count + self.cjk_count
    }
}

// ============================================================================
// Texture Layout Constants and Utilities
// ============================================================================

/// Texture layout constants for 4096x4096 symbol texture
///
/// Linear symbol array layout:
/// - [0, 40959]: Sprite (160 blocks × 256 = 40960 symbols, 16×16px)
/// - [40960, 43519]: TUI (10 blocks × 256 = 2560 symbols, 16×32px)
/// - [43520, 44287]: Emoji (6 blocks × 128 = 768 symbols, 32×32px)
/// - [44288, 48383]: CJK (128 cols × 32 rows = 4096 symbols, 32×32px)
pub mod layout {
    #[cfg(graphics_mode)]
    use crate::render::graph::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};

    // Default base size (used when PIXEL_SYM_WIDTH/HEIGHT not initialized or in non-graphics mode)
    pub const DEFAULT_BASE_WIDTH: u32 = 16;
    pub const DEFAULT_BASE_HEIGHT: u32 = 16;

    // Region counts
    pub const SPRITE_BLOCKS: u32 = 160;
    pub const SPRITE_SYMBOLS_PER_BLOCK: u32 = 256;
    pub const SPRITE_TOTAL: u32 = SPRITE_BLOCKS * SPRITE_SYMBOLS_PER_BLOCK; // 40960

    pub const TUI_BLOCKS: u32 = 10;
    pub const TUI_SYMBOLS_PER_BLOCK: u32 = 256;
    pub const TUI_TOTAL: u32 = TUI_BLOCKS * TUI_SYMBOLS_PER_BLOCK; // 2560

    pub const EMOJI_BLOCKS: u32 = 6;
    pub const EMOJI_SYMBOLS_PER_BLOCK: u32 = 128;
    pub const EMOJI_TOTAL: u32 = EMOJI_BLOCKS * EMOJI_SYMBOLS_PER_BLOCK; // 768

    pub const CJK_COLS: u32 = 128;
    pub const CJK_ROWS: u32 = 32;
    pub const CJK_TOTAL: u32 = CJK_COLS * CJK_ROWS; // 4096

    // Linear index bases
    pub const SPRITE_BASE: usize = 0;
    pub const TUI_BASE: usize = SPRITE_TOTAL as usize; // 40960
    pub const EMOJI_BASE: usize = TUI_BASE + TUI_TOTAL as usize; // 43520
    pub const CJK_BASE: usize = EMOJI_BASE + EMOJI_TOTAL as usize; // 44288

    // Block index boundaries
    pub const SPRITE_BLOCK_START: usize = 0;
    pub const SPRITE_BLOCK_END: usize = 159;
    pub const TUI_BLOCK_START: usize = 160;
    pub const TUI_BLOCK_END: usize = 169;
    pub const EMOJI_BLOCK_START: usize = 170;
    pub const EMOJI_BLOCK_END: usize = 175;
    pub const CJK_BLOCK_START: usize = 176;
    pub const CJK_BLOCK_END: usize = 207; // 176 + 32 - 1 = 207 (32 rows)
    pub const CJK_SYMBOLS_PER_BLOCK: u32 = 128; // 128 columns per row

    // Size multipliers (relative to base PIXEL_SYM_WIDTH/HEIGHT)
    // Sprite: 1x width, 1x height
    // TUI: 1x width, 2x height
    // Emoji: 2x width, 2x height
    // CJK: 2x width, 2x height
    pub const TUI_HEIGHT_MULTIPLIER: u32 = 2;
    pub const EMOJI_SIZE_MULTIPLIER: u32 = 2;
    pub const CJK_SIZE_MULTIPLIER: u32 = 2;

    // Background fill symbol (Block 0, symbol 160 = row 10, col 0)
    pub const BG_FILL_SYMBOL: usize = 160;

    // ========== Runtime size functions ==========
    // In graphics mode: read from PIXEL_SYM_WIDTH/HEIGHT (with fallback to defaults)
    // In non-graphics mode: use default values

    /// Get base symbol width
    /// In graphics mode: from PIXEL_SYM_WIDTH if initialized, else default
    /// In non-graphics mode: default value
    #[inline]
    pub fn base_width() -> u32 {
        #[cfg(graphics_mode)]
        {
            PIXEL_SYM_WIDTH.get().map(|v| *v as u32).unwrap_or(DEFAULT_BASE_WIDTH)
        }
        #[cfg(not(graphics_mode))]
        {
            DEFAULT_BASE_WIDTH
        }
    }

    /// Get base symbol height
    /// In graphics mode: from PIXEL_SYM_HEIGHT if initialized, else default
    /// In non-graphics mode: default value
    #[inline]
    pub fn base_height() -> u32 {
        #[cfg(graphics_mode)]
        {
            PIXEL_SYM_HEIGHT.get().map(|v| *v as u32).unwrap_or(DEFAULT_BASE_HEIGHT)
        }
        #[cfg(not(graphics_mode))]
        {
            DEFAULT_BASE_HEIGHT
        }
    }

    /// Sprite symbol width (= base_width)
    #[inline]
    pub fn sprite_width() -> u32 {
        base_width()
    }

    /// Sprite symbol height (= base_height)
    #[inline]
    pub fn sprite_height() -> u32 {
        base_height()
    }

    /// TUI symbol width (= base_width)
    #[inline]
    pub fn tui_width() -> u32 {
        base_width()
    }

    /// TUI symbol height (= base_height * 2)
    #[inline]
    pub fn tui_height() -> u32 {
        base_height() * TUI_HEIGHT_MULTIPLIER
    }

    /// Emoji symbol width (= base_width * 2)
    #[inline]
    pub fn emoji_width() -> u32 {
        base_width() * EMOJI_SIZE_MULTIPLIER
    }

    /// Emoji symbol height (= base_height * 2)
    #[inline]
    pub fn emoji_height() -> u32 {
        base_height() * EMOJI_SIZE_MULTIPLIER
    }

    /// CJK symbol width (= base_width * 2)
    #[inline]
    pub fn cjk_width() -> u32 {
        base_width() * CJK_SIZE_MULTIPLIER
    }

    /// CJK symbol height (= base_height * 2)
    #[inline]
    pub fn cjk_height() -> u32 {
        base_height() * CJK_SIZE_MULTIPLIER
    }

    /// Sprite area Y start (= 0)
    #[inline]
    pub fn sprite_y_start() -> u32 {
        0
    }

    /// TUI area Y start (= SPRITE_BLOCKS / 16 * 16 * base_height = 10 * 16 * base_height)
    #[inline]
    pub fn tui_y_start() -> u32 {
        (SPRITE_BLOCKS / 16) * 16 * base_height()
    }

    /// Emoji area X start (= TUI_BLOCKS * 16 * base_width)
    #[inline]
    pub fn emoji_x_start() -> u32 {
        TUI_BLOCKS * 16 * base_width()
    }

    /// CJK area Y start (= tui_y_start + 16 * tui_height)
    #[inline]
    pub fn cjk_y_start() -> u32 {
        tui_y_start() + 16 * tui_height()
    }
}

/// Calculate linear texture symbol index from block and symbol index
///
/// This function converts (texidx, symidx) from symbol_map lookups
/// into a linear index for the symbols array in renderers.
///
/// # Arguments
/// * `texidx` - Block/texture index (0-159 for Sprite, 160-169 for TUI, 170-175 for Emoji)
/// * `symidx` - Symbol index within the block
///
/// # Returns
/// Linear index into the symbols array
///
/// # Example
/// ```
/// use rust_pixel::render::symbol_map::calc_linear_index;
///
/// // Sprite block 0, symbol 0 -> index 0
/// assert_eq!(calc_linear_index(0, 0), 0);
///
/// // TUI block 160, symbol 0 -> index 40960
/// assert_eq!(calc_linear_index(160, 0), 40960);
///
/// // Emoji block 170, symbol 0 -> index 43520
/// assert_eq!(calc_linear_index(170, 0), 43520);
/// ```
#[inline]
pub fn calc_linear_index(texidx: usize, symidx: usize) -> usize {
    if texidx >= layout::CJK_BLOCK_START {
        // CJK blocks (176-207): base 44288 + row offset + column index
        // Each block = one row (128 symbols), symidx = column (0-127)
        layout::CJK_BASE + (texidx - layout::CJK_BLOCK_START) * layout::CJK_SYMBOLS_PER_BLOCK as usize + symidx
    } else if texidx >= layout::EMOJI_BLOCK_START {
        // Emoji blocks (170-175): base 43520 + block offset + symbol index
        layout::EMOJI_BASE + (texidx - layout::EMOJI_BLOCK_START) * layout::EMOJI_SYMBOLS_PER_BLOCK as usize + symidx
    } else if texidx >= layout::TUI_BLOCK_START {
        // TUI blocks (160-169): base 40960 + block offset + symbol index
        layout::TUI_BASE + (texidx - layout::TUI_BLOCK_START) * layout::TUI_SYMBOLS_PER_BLOCK as usize + symidx
    } else {
        // Sprite blocks (0-159): direct linear index
        texidx * layout::SPRITE_SYMBOLS_PER_BLOCK as usize + symidx
    }
}

/// Symbol frame descriptor for texture loading
///
/// Describes the position and size of a symbol in the texture atlas.
/// Used by renderers (GL/WGPU) when loading symbol textures.
#[derive(Debug, Clone, Copy)]
pub struct SymbolFrame {
    /// X position in texture (pixels)
    pub pixel_x: u32,
    /// Y position in texture (pixels)
    pub pixel_y: u32,
    /// Symbol width (pixels)
    pub width: u32,
    /// Symbol height (pixels)
    pub height: u32,
}

/// Iterator over all symbol frames for texture loading
///
/// Yields SymbolFrame for each symbol in order:
/// 1. Sprite region (40960 frames)
/// 2. TUI region (2560 frames)
/// 3. Emoji region (768 frames)
/// 4. CJK region (4096 frames)
///
/// Total: 48384 frames
pub struct SymbolFrameIterator {
    current: usize,
    total: usize,
}

impl SymbolFrameIterator {
    pub fn new() -> Self {
        let total = layout::SPRITE_TOTAL + layout::TUI_TOTAL + layout::EMOJI_TOTAL + layout::CJK_TOTAL;
        Self {
            current: 0,
            total: total as usize,
        }
    }
}

impl Default for SymbolFrameIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for SymbolFrameIterator {
    type Item = SymbolFrame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let frame = if self.current < layout::TUI_BASE {
            // Sprite region: 160 blocks × 256 symbols
            let idx = self.current;
            let block = idx / layout::SPRITE_SYMBOLS_PER_BLOCK as usize;
            let sym = idx % layout::SPRITE_SYMBOLS_PER_BLOCK as usize;

            let block_col = block % 16;
            let block_row = block / 16;
            let sym_col = sym % 16;
            let sym_row = sym / 16;

            let sprite_w = layout::sprite_width();
            let sprite_h = layout::sprite_height();

            SymbolFrame {
                pixel_x: ((block_col * 16 + sym_col) as u32) * sprite_w,
                pixel_y: ((block_row * 16 + sym_row) as u32) * sprite_h,
                width: sprite_w,
                height: sprite_h,
            }
        } else if self.current < layout::EMOJI_BASE {
            // TUI region: 10 blocks × 256 symbols
            let idx = self.current - layout::TUI_BASE;
            let block = idx / layout::TUI_SYMBOLS_PER_BLOCK as usize;
            let sym = idx % layout::TUI_SYMBOLS_PER_BLOCK as usize;

            let sym_col = sym % 16;
            let sym_row = sym / 16;

            let tui_w = layout::tui_width();
            let tui_h = layout::tui_height();

            SymbolFrame {
                pixel_x: ((block * 16 + sym_col) as u32) * tui_w,
                pixel_y: layout::tui_y_start() + (sym_row as u32) * tui_h,
                width: tui_w,
                height: tui_h,
            }
        } else if self.current < layout::CJK_BASE {
            // Emoji region: 6 blocks × 128 symbols
            let idx = self.current - layout::EMOJI_BASE;
            let block = idx / layout::EMOJI_SYMBOLS_PER_BLOCK as usize;
            let sym = idx % layout::EMOJI_SYMBOLS_PER_BLOCK as usize;

            let sym_col = sym % 8;
            let sym_row = sym / 8;

            let emoji_w = layout::emoji_width();
            let emoji_h = layout::emoji_height();

            SymbolFrame {
                pixel_x: layout::emoji_x_start() + ((block * 8 + sym_col) as u32) * emoji_w,
                pixel_y: layout::tui_y_start() + (sym_row as u32) * emoji_h,
                width: emoji_w,
                height: emoji_h,
            }
        } else {
            // CJK region: 128 cols × 32 rows
            let idx = self.current - layout::CJK_BASE;
            let col = idx % layout::CJK_COLS as usize;
            let row = idx / layout::CJK_COLS as usize;

            let cjk_w = layout::cjk_width();
            let cjk_h = layout::cjk_height();

            SymbolFrame {
                pixel_x: (col as u32) * cjk_w,
                pixel_y: layout::cjk_y_start() + (row as u32) * cjk_h,
                width: cjk_w,
                height: cjk_h,
            }
        };

        self.current += 1;
        Some(frame)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total - self.current;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for SymbolFrameIterator {}

/// Create an iterator over all symbol frames
///
/// # Example
/// ```ignore
/// for frame in iter_symbol_frames() {
///     let symbol = make_frame(frame.pixel_x, frame.pixel_y, frame.width, frame.height);
///     symbols.push(symbol);
/// }
/// ```
pub fn iter_symbol_frames() -> SymbolFrameIterator {
    SymbolFrameIterator::new()
}

// JSON configuration structures

#[derive(Deserialize)]
struct SymbolMapConfig {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    texture_size: u32,
    regions: HashMap<String, RegionConfig>,
}

#[derive(Deserialize)]
struct RegionConfig {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    region_type: Option<String>,
    block_range: Option<[u8; 2]>,
    char_size: Option<[u32; 2]>,
    chars_per_block: Option<u32>,
    symbols: Option<SymbolsValue>,
    extras: Option<HashMap<String, [u8; 2]>>,
    pixel_region: Option<[u32; 4]>,
    #[allow(dead_code)]
    grid_cols: Option<u32>,
    mappings: Option<HashMap<String, [u32; 2]>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SymbolsValue {
    String(String),
    Array(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_load() {
        let map = SymbolMap::default();
        let stats = map.stats();
        assert!(stats.sprite_count > 0, "Sprite symbols should be loaded");
        assert!(stats.tui_count > 0, "TUI symbols should be loaded");
        assert!(stats.emoji_count > 0, "Emoji symbols should be loaded");
    }

    #[test]
    fn test_sprite_lookup() {
        let map = SymbolMap::default();
        // '@' is the first character in sprite symbols
        // Use sprite_idx directly since '@' also exists in TUI
        assert_eq!(map.sprite_idx("@"), Some((0, 0)));
        // 'a' is the second
        assert_eq!(map.sprite_idx("a"), Some((0, 1)));
        // Test lookup_in_region for explicit region query
        assert!(matches!(
            map.lookup_in_region("@", SymbolRegion::Sprite),
            SymbolIndex::Sprite(0, 0)
        ));
    }

    #[test]
    fn test_tui_lookup() {
        let map = SymbolMap::default();
        // Space is first in TUI
        if let SymbolIndex::Tui(block, idx) = map.lookup(" ") {
            assert_eq!(block, 160);
            assert_eq!(idx, 0);
        } else {
            panic!("Space should be in TUI region");
        }
    }

    #[test]
    fn test_emoji_lookup() {
        let map = SymbolMap::default();
        if let SymbolIndex::Emoji(block, idx) = map.lookup("😀") {
            assert_eq!(block, 170);
            assert_eq!(idx, 0);
        } else {
            panic!("😀 should be in Emoji region");
        }
    }

    #[test]
    fn test_not_found() {
        let map = SymbolMap::default();
        // Use a rare character not in the 3500 common CJK set
        assert!(matches!(map.lookup("㊀"), SymbolIndex::NotFound));
    }

    #[test]
    fn test_cjk_lookup() {
        let map = SymbolMap::default();
        // "一" is the first CJK character at grid position (0, 0)
        // pixel_region starts at y=3072, char_size is 32x32
        if let SymbolIndex::Cjk(x, y) = map.lookup("一") {
            assert_eq!(x, 0);
            assert_eq!(y, 3072);
        } else {
            panic!("一 should be in CJK region");
        }

        // "中" is at grid position (26, 0)
        if let SymbolIndex::Cjk(x, y) = map.lookup("中") {
            assert_eq!(x, 26 * 32);
            assert_eq!(y, 3072);
        } else {
            panic!("中 should be in CJK region");
        }

        // Test direct cjk_coords
        assert_eq!(map.cjk_coords('一'), Some((0, 3072)));
    }

    #[test]
    fn test_cjk_idx() {
        let map = SymbolMap::default();

        // "一" is at grid position (col=0, row=0)
        // block = 176 + 0 = 176, idx = 0
        assert_eq!(map.cjk_idx("一"), Some((176, 0)));

        // "中" is at grid position (col=26, row=0)
        // block = 176 + 0 = 176, idx = 26
        assert_eq!(map.cjk_idx("中"), Some((176, 26)));

        // Test that linear index calculation works for CJK
        // "一": linear index = CJK_BASE + 0 * 128 + 0 = 44288
        assert_eq!(calc_linear_index(176, 0), layout::CJK_BASE);

        // "中": linear index = CJK_BASE + 0 * 128 + 26 = 44314
        assert_eq!(calc_linear_index(176, 26), layout::CJK_BASE + 26);
    }

    #[test]
    fn test_extras() {
        let map = SymbolMap::default();
        // '▇' is in extras with explicit coordinates
        // Use sprite_idx directly to check extras mapping
        if let Some((block, idx)) = map.sprite_idx("▇") {
            assert_eq!(block, 0);
            assert_eq!(idx, 209);
        } else {
            panic!("▇ should be in Sprite region via extras, sprite count: {}", map.stats().sprite_count);
        }
    }

    #[test]
    fn test_calc_linear_index() {
        // Sprite region
        assert_eq!(calc_linear_index(0, 0), 0);
        assert_eq!(calc_linear_index(0, 255), 255);
        assert_eq!(calc_linear_index(1, 0), 256);
        assert_eq!(calc_linear_index(159, 255), 40959);

        // TUI region
        assert_eq!(calc_linear_index(160, 0), 40960);
        assert_eq!(calc_linear_index(160, 255), 41215);
        assert_eq!(calc_linear_index(169, 255), 43519);

        // Emoji region
        assert_eq!(calc_linear_index(170, 0), 43520);
        assert_eq!(calc_linear_index(170, 127), 43647);
        assert_eq!(calc_linear_index(175, 127), 44287);

        // CJK region
        assert_eq!(calc_linear_index(176, 0), 44288);  // First CJK (row 0, col 0)
        assert_eq!(calc_linear_index(176, 127), 44415);  // End of first row
        assert_eq!(calc_linear_index(177, 0), 44416);  // Start of second row
        assert_eq!(calc_linear_index(207, 127), 48383);  // Last CJK (row 31, col 127)
    }

    #[test]
    fn test_symbol_frame_iterator() {
        let frames: Vec<_> = iter_symbol_frames().collect();

        // Total count should be 48384
        let expected_total = layout::SPRITE_TOTAL + layout::TUI_TOTAL + layout::EMOJI_TOTAL + layout::CJK_TOTAL;
        assert_eq!(frames.len(), expected_total as usize);

        // First frame should be Sprite at (0, 0) with base size
        assert_eq!(frames[0].pixel_x, 0);
        assert_eq!(frames[0].pixel_y, 0);
        assert_eq!(frames[0].width, layout::sprite_width());
        assert_eq!(frames[0].height, layout::sprite_height());

        // First TUI frame at index 40960
        let tui_first = &frames[layout::TUI_BASE];
        assert_eq!(tui_first.pixel_x, 0);
        assert_eq!(tui_first.pixel_y, layout::tui_y_start());
        assert_eq!(tui_first.width, layout::tui_width());
        assert_eq!(tui_first.height, layout::tui_height());

        // First Emoji frame at index 43520
        let emoji_first = &frames[layout::EMOJI_BASE];
        assert_eq!(emoji_first.pixel_x, layout::emoji_x_start());
        assert_eq!(emoji_first.pixel_y, layout::tui_y_start());
        assert_eq!(emoji_first.width, layout::emoji_width());
        assert_eq!(emoji_first.height, layout::emoji_height());

        // First CJK frame at index 44288
        let cjk_first = &frames[layout::CJK_BASE];
        assert_eq!(cjk_first.pixel_x, 0);
        assert_eq!(cjk_first.pixel_y, layout::cjk_y_start());
        assert_eq!(cjk_first.width, layout::cjk_width());
        assert_eq!(cjk_first.height, layout::cjk_height());
    }

    #[test]
    fn test_layout_constants() {
        // Verify layout constants
        assert_eq!(layout::SPRITE_TOTAL, 40960);
        assert_eq!(layout::TUI_TOTAL, 2560);
        assert_eq!(layout::EMOJI_TOTAL, 768);
        assert_eq!(layout::CJK_TOTAL, 4096);

        assert_eq!(layout::SPRITE_BASE, 0);
        assert_eq!(layout::TUI_BASE, 40960);
        assert_eq!(layout::EMOJI_BASE, 43520);
        assert_eq!(layout::CJK_BASE, 44288);
    }

    #[test]
    fn test_layout_runtime_functions() {
        // Test runtime size functions use default values when PIXEL_SYM_WIDTH/HEIGHT not initialized
        assert_eq!(layout::base_width(), layout::DEFAULT_BASE_WIDTH);
        assert_eq!(layout::base_height(), layout::DEFAULT_BASE_HEIGHT);

        // Test derived sizes
        assert_eq!(layout::sprite_width(), layout::DEFAULT_BASE_WIDTH);
        assert_eq!(layout::sprite_height(), layout::DEFAULT_BASE_HEIGHT);
        assert_eq!(layout::tui_width(), layout::DEFAULT_BASE_WIDTH);
        assert_eq!(layout::tui_height(), layout::DEFAULT_BASE_HEIGHT * layout::TUI_HEIGHT_MULTIPLIER);
        assert_eq!(layout::emoji_width(), layout::DEFAULT_BASE_WIDTH * layout::EMOJI_SIZE_MULTIPLIER);
        assert_eq!(layout::emoji_height(), layout::DEFAULT_BASE_HEIGHT * layout::EMOJI_SIZE_MULTIPLIER);
        assert_eq!(layout::cjk_width(), layout::DEFAULT_BASE_WIDTH * layout::CJK_SIZE_MULTIPLIER);
        assert_eq!(layout::cjk_height(), layout::DEFAULT_BASE_HEIGHT * layout::CJK_SIZE_MULTIPLIER);

        // Test pixel positions
        assert_eq!(layout::sprite_y_start(), 0);
        assert_eq!(layout::tui_y_start(), (layout::SPRITE_BLOCKS / 16) * 16 * layout::DEFAULT_BASE_HEIGHT);
        assert_eq!(layout::emoji_x_start(), layout::TUI_BLOCKS * 16 * layout::DEFAULT_BASE_WIDTH);
    }
}
