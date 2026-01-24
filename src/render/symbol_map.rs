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
        assert!(matches!(map.lookup("不存在"), SymbolIndex::NotFound));
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
}
