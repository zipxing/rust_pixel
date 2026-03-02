//! Unified Symbol Map Configuration
//!
//! This module provides a JSON-based configuration system for mapping characters
//! to texture coordinates across different regions (Sprite, TUI, Emoji, CJK).
//!
//! # Regions
//! - **Sprite** (Block 0-159): 16x16 game sprites, single color
//! - **TUI** (Block 160-169): 16x32 terminal UI characters, single color
//! - **Emoji** (Block 170-175): 32x32 color emoji
//! - **CJK** (Block 176-239): 32x32 Chinese characters, single color, 64 blocks (16 cols × 4 rows)
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │ Sprite 区域（y=0-2559, 2560px 高）                         │
//! │ - 10 rows × 16 blocks/row = 160 blocks                     │
//! │ - 每 block: 256×256px (16×16 chars, 16×16px each)          │
//! │ - Block 0-159: 40,960 sprites                              │
//! ├────────────────────────────────────────────────────────────┤
//! │ TUI + Emoji 区域（y=2560-3071, 512px 高）                  │
//! │                                                            │
//! │ TUI 区域（x=0-2559）:                                      │
//! │ - 10 blocks (Block 160-169)                                │
//! │ - 每 block: 256×512px (16×16 chars, 16×32px each)          │
//! │ - 2560 TUI 字符                                            │
//! │                                                            │
//! │ Emoji 区域（x=2560-4095）:                                 │
//! │ - 6 blocks (Block 170-175)                                 │
//! │ - 每 block: 256×512px (8×16 emojis, 32×32px each)          │
//! │ - 768 Emoji                                                │
//! ├────────────────────────────────────────────────────────────┤
//! │ CJK 区域（y=3072-4095, 1024px 高）                         │
//! │ - 64 blocks (16 cols × 4 rows, Block 176-239)              │
//! │ - 每 block: 256×256px (8×8 chars, 32×32px each)            │
//! │ - 4096 CJK 字符                                            │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//! ```rust,ignore
//! let map = get_symbol_map();
//! if let Some((block, idx)) = map.emoji_idx("😀") { /* render emoji */ }
//! if let Some((block, idx)) = map.tui_idx("A") { /* render TUI char */ }
//! if let Some((block, idx)) = map.cjk_idx("中") { /* render CJK */ }
//! ```

use super::cell::cellsym_block;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Path to symbol_map.json file (relative to executable or assets directory)
/// This file should be in the same directory as symbols.png
pub const PIXEL_SYMBOL_MAP_FILE: &str = "assets/pix/symbol_map.json";

/// Path to layered_symbol_map.json file
pub const PIXEL_LAYERED_SYMBOL_MAP_FILE: &str = "assets/pix/layered_symbol_map.json";

/// Global symbol map instance, initialized lazily
/// For native mode: loaded from file on first access
/// For web mode: must be initialized via init_symbol_map() before use
static GLOBAL_SYMBOL_MAP: OnceLock<SymbolMap> = OnceLock::new();

/// Initialize the global symbol map from JSON string
/// This is the preferred method for web mode where JSON is loaded via JavaScript
pub fn init_symbol_map_from_json(json: &str) -> Result<(), String> {
    let map = SymbolMap::from_json(json)
        .map_err(|e| format!("Failed to parse symbol_map.json: {}", e))?;
    GLOBAL_SYMBOL_MAP
        .set(map)
        .map_err(|_| "Symbol map already initialized".to_string())
}

/// Initialize the global symbol map from file path
/// This should be called during adapter.init() after game_config is initialized
/// Uses get_game_config().project_path to load from app's assets directory
/// Only available in graphics mode (SDL/Glow/WGPU)
#[cfg(all(not(target_arch = "wasm32"), graphics_mode))]
pub fn init_symbol_map_from_file() -> Result<(), String> {
    let project_path = &crate::get_game_config().project_path;
    let symbol_map_path = format!(
        "{}{}{}",
        project_path,
        std::path::MAIN_SEPARATOR,
        PIXEL_SYMBOL_MAP_FILE
    );

    let map = SymbolMap::load(&symbol_map_path)
        .map_err(|e| format!("Failed to load symbol map from {}: {}", symbol_map_path, e))?;

    GLOBAL_SYMBOL_MAP
        .set(map)
        .map_err(|_| "Symbol map already initialized".to_string())?;

    log::info!("Loaded symbol map from {}", symbol_map_path);
    Ok(())
}

/// Get the global symbol map instance
/// - Graphics mode (SDL/Glow/WGPU): loads from app's assets directory
/// - Terminal mode: returns empty SymbolMap (not needed for text rendering)
/// - Web mode: must be initialized via init_symbol_map_from_json() first
///
/// # Panics
/// - Graphics mode: panics if symbol_map.json cannot be loaded
/// - Web mode: panics if init_symbol_map_from_json() was not called before this
pub fn get_symbol_map() -> &'static SymbolMap {
    GLOBAL_SYMBOL_MAP.get_or_init(|| {
        // For WASM builds, panic if not initialized
        // (wasm_init_symbol_map MUST be called before PixelGame.new())
        #[cfg(target_arch = "wasm32")]
        {
            panic!(
                "Symbol map not initialized! \
                Call wasm_init_symbol_map() before creating the game instance."
            );
        }

        // For native graphics mode, lazy load from app's assets directory
        #[cfg(all(not(target_arch = "wasm32"), graphics_mode))]
        {
            let project_path = &crate::get_game_config().project_path;
            let symbol_map_path = format!(
                "{}{}{}",
                project_path,
                std::path::MAIN_SEPARATOR,
                PIXEL_SYMBOL_MAP_FILE
            );
            match SymbolMap::load(&symbol_map_path) {
                Ok(map) => {
                    log::info!("Loaded symbol map from {}", symbol_map_path);
                    map
                }
                Err(e) => {
                    panic!(
                        "Failed to load symbol map from {}: {}. \
                        Ensure symbol_map.json exists in the app's assets directory.",
                        symbol_map_path, e
                    );
                }
            }
        }

        // For terminal mode (crossterm), symbol map is not needed
        // Return empty map to avoid file loading
        #[cfg(all(not(target_arch = "wasm32"), not(graphics_mode)))]
        {
            log::info!("Terminal mode: using empty symbol map (not needed for text rendering)");
            SymbolMap::empty()
        }
    })
}

// ============================================================================
// Layered Symbol Map (Texture2DArray with Mipmap Levels)
// ============================================================================

/// UV coordinates for one mipmap level in a texture array layer
#[derive(Debug, Clone, Copy, Default)]
pub struct MipUV {
    /// Layer index in Texture2DArray
    pub layer: u16,
    /// Normalized UV x coordinate (0.0-1.0)
    pub x: f32,
    /// Normalized UV y coordinate (0.0-1.0)
    pub y: f32,
    /// Normalized UV width (0.0-1.0)
    pub w: f32,
    /// Normalized UV height (0.0-1.0)
    pub h: f32,
}

/// Resolved symbol with 3 mipmap levels of UV coordinates.
/// Cached in Cell after first resolution — zero lookup on subsequent frames.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    /// Cell width in grid units (1 for normal, 2 for wide chars like CJK/Emoji)
    pub cell_w: u8,
    /// Cell height in grid units (1 for single-height, 2 for tall chars like TUI/CJK/Emoji)
    pub cell_h: u8,
    /// 3 mipmap levels: [Level 0 (high), Level 1 (mid), Level 2 (low)]
    pub mips: [MipUV; 3],
}

/// Layered symbol map — maps symbol strings to Tile with mip UV coordinates.
///
/// Loaded from `layered_symbol_map.json` generated by `cargo pixel symbols --layered`.
/// Used by Texture2DArray rendering path (WGPU adapter).
pub struct LayeredSymbolMap {
    /// Layer size in pixels (all layers are square, typically 2048)
    pub layer_size: u32,
    /// Number of layers
    pub layer_count: u32,
    /// Relative paths to layer PNG files
    pub layer_files: Vec<String>,
    /// Symbol string → Tile mapping
    symbols: HashMap<String, Tile>,
}

/// Default tile returned for unknown symbols (transparent/empty)
const DEFAULT_TILE: Tile = Tile {
    cell_w: 1,
    cell_h: 1,
    mips: [MipUV { layer: 0, x: 0.0, y: 0.0, w: 0.0, h: 0.0 }; 3],
};

/// Global layered symbol map instance
static GLOBAL_LAYERED_SYMBOL_MAP: OnceLock<LayeredSymbolMap> = OnceLock::new();

/// Initialize the global layered symbol map from JSON string
pub fn init_layered_symbol_map_from_json(json: &str) -> Result<(), String> {
    let map = LayeredSymbolMap::from_json(json)?;
    GLOBAL_LAYERED_SYMBOL_MAP
        .set(map)
        .map_err(|_| "Layered symbol map already initialized".to_string())
}

/// Initialize the global layered symbol map from file path
#[cfg(all(not(target_arch = "wasm32"), graphics_mode))]
pub fn init_layered_symbol_map_from_file(path: &str) -> Result<(), String> {
    let map = LayeredSymbolMap::load(path)?;
    GLOBAL_LAYERED_SYMBOL_MAP
        .set(map)
        .map_err(|_| "Layered symbol map already initialized".to_string())?;
    log::info!("Loaded layered symbol map from {}", path);
    Ok(())
}

/// Get the global layered symbol map, if initialized
pub fn get_layered_symbol_map() -> Option<&'static LayeredSymbolMap> {
    GLOBAL_LAYERED_SYMBOL_MAP.get()
}

/// Check if layered symbol map is available
pub fn has_layered_symbol_map() -> bool {
    GLOBAL_LAYERED_SYMBOL_MAP.get().is_some()
}

impl LayeredSymbolMap {
    /// Load from JSON file path
    pub fn load(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;
        Self::from_json(&content)
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, String> {
        let root: Value = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let version = root["version"].as_u64().unwrap_or(0) as u32;
        if version != 2 {
            return Err(format!("Unsupported layered symbol map version: {} (expected 2)", version));
        }

        let layer_size = root["layer_size"].as_u64()
            .ok_or("Missing layer_size")? as u32;
        let layer_count = root["layer_count"].as_u64()
            .ok_or("Missing layer_count")? as u32;

        let layer_files: Vec<String> = root["layer_files"]
            .as_array()
            .ok_or("Missing layer_files")?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if layer_files.len() != layer_count as usize {
            return Err(format!(
                "layer_count ({}) != layer_files length ({})",
                layer_count, layer_files.len()
            ));
        }

        let symbols_obj = root["symbols"]
            .as_object()
            .ok_or("Missing symbols")?;

        let inv = 1.0 / layer_size as f32;
        let mut symbols = HashMap::with_capacity(symbols_obj.len());

        for (key, val) in symbols_obj {
            let cell_w = val["w"].as_u64().unwrap_or(1) as u8;
            let cell_h = val["h"].as_u64().unwrap_or(1) as u8;

            let mut mips = [MipUV::default(); 3];
            for (i, mip_key) in ["mip0", "mip1", "mip2"].iter().enumerate() {
                if let Some(m) = val.get(*mip_key) {
                    mips[i] = MipUV {
                        layer: m["layer"].as_u64().unwrap_or(0) as u16,
                        x: m["x"].as_u64().unwrap_or(0) as f32 * inv,
                        y: m["y"].as_u64().unwrap_or(0) as f32 * inv,
                        w: m["w"].as_u64().unwrap_or(0) as f32 * inv,
                        h: m["h"].as_u64().unwrap_or(0) as f32 * inv,
                    };
                }
            }

            symbols.insert(key.clone(), Tile { cell_w, cell_h, mips });
        }

        Ok(Self {
            layer_size,
            layer_count,
            layer_files,
            symbols,
        })
    }

    /// Resolve a symbol string to a Tile with 3 mipmap UV levels.
    /// Returns the default tile (empty/transparent) for unknown symbols.
    #[inline]
    pub fn resolve(&self, symbol: &str) -> &Tile {
        self.symbols.get(symbol).unwrap_or(&DEFAULT_TILE)
    }

    /// Check if a symbol exists in the map
    #[inline]
    pub fn contains(&self, symbol: &str) -> bool {
        self.symbols.contains_key(symbol)
    }

    /// Get total number of symbols
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Get statistics
    pub fn stats(&self) -> LayeredSymbolMapStats {
        LayeredSymbolMapStats {
            layer_size: self.layer_size,
            layer_count: self.layer_count,
            symbol_count: self.symbols.len(),
        }
    }
}

/// Statistics for layered symbol map
#[derive(Debug, Clone)]
pub struct LayeredSymbolMapStats {
    pub layer_size: u32,
    pub layer_count: u32,
    pub symbol_count: usize,
}

/// Build a `Vec<Tile>` indexed by linear texsym, mapping each to the corresponding
/// Tile in the LayeredSymbolMap. Uses the legacy SymbolMap for reverse key lookups.
///
/// This is the bridge between the legacy texsym-based pipeline and the new
/// LayeredSymbolMap: it lets `generate_instances_from_render_cells()` look up
/// UV + layer data by linear index without changing the RenderCell format.
pub fn build_layered_tiles(symbol_map: &SymbolMap, layered_map: &LayeredSymbolMap) -> Vec<Tile> {
    let total = layout::TOTAL_SYMBOLS as usize;
    let mut tiles = vec![Tile::default(); total];

    // Sprites: PUA encoding (block 0-159, 256 symbols each)
    for block in 0u8..160 {
        for idx in 0..=255u8 {
            let key = cellsym_block(block, idx);
            let linear = calc_linear_index(block as usize, idx as usize);
            if linear < total {
                let tile = layered_map.resolve(&key);
                if tile.mips[0].w > 0.0 || tile.mips[1].w > 0.0 {
                    tiles[linear] = *tile;
                }
            }
        }
    }

    // TUI: iterate SymbolMap.tui entries
    for (char_str, &(block, idx)) in symbol_map.iter_tui() {
        let linear = calc_linear_index(block as usize, idx as usize);
        if linear < total {
            let tile = layered_map.resolve(char_str);
            if tile.mips[0].w > 0.0 || tile.mips[1].w > 0.0 {
                tiles[linear] = *tile;
            }
        }
    }

    // Emoji: iterate SymbolMap.emoji entries
    for (char_str, &(block, idx)) in symbol_map.iter_emoji() {
        let linear = calc_linear_index(block as usize, idx as usize);
        if linear < total {
            let tile = layered_map.resolve(char_str);
            if tile.mips[0].w > 0.0 || tile.mips[1].w > 0.0 {
                tiles[linear] = *tile;
            }
        }
    }

    // CJK: iterate SymbolMap.cjk entries, convert to (block, idx) via cjk_idx
    for (ch, _) in symbol_map.iter_cjk() {
        let ch_str = ch.to_string();
        if let Some((block, idx)) = symbol_map.cjk_idx(&ch_str) {
            let linear = calc_linear_index(block as usize, idx as usize);
            if linear < total {
                let tile = layered_map.resolve(&ch_str);
                if tile.mips[0].w > 0.0 || tile.mips[1].w > 0.0 {
                    tiles[linear] = *tile;
                }
            }
        }
    }

    tiles
}

// ============================================================================
// Legacy Symbol Map (Single Texture Atlas)
// ============================================================================

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

    /// Load symbol map from default file path (using project_path)
    /// Prefer using get_symbol_map() for the global instance
    ///
    /// # Panics
    /// Panics if the symbol_map.json file cannot be loaded
    pub fn default_map() -> Self {
        // Use get_game_config().project_path to load from app's assets directory
        let project_path = &crate::get_game_config().project_path;
        let symbol_map_path = format!(
            "{}{}{}",
            project_path,
            std::path::MAIN_SEPARATOR,
            PIXEL_SYMBOL_MAP_FILE
        );
        Self::load(&symbol_map_path).expect("Failed to load symbol_map.json from default path")
    }

    /// Create empty symbol map (for testing only)
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

        Self {
            sprite,
            tui,
            emoji,
            cjk,
        }
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
        // Use layout module for dynamic size calculation instead of JSON fields
        // This allows the same symbol_map.json to work with different texture sizes
        let char_w = layout::cjk_width() as u16;
        let char_h = layout::cjk_height() as u16;
        let base_y = layout::cjk_y_start() as u16;

        if let Some(mappings) = &region.mappings {
            for (ch, coords) in mappings {
                if let Some(c) = ch.chars().next() {
                    // coords[0] = col, coords[1] = row in grid
                    let pixel_x = coords[0] as u16 * char_w;
                    let pixel_y = base_y + coords[1] as u16 * char_h;
                    map.insert(c, (pixel_x, pixel_y));
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
    /// Returns (block, idx) where:
    /// - block is 176-239 (64 blocks: 16 cols × 4 rows)
    /// - idx is 0-63 (8×8 chars per block)
    pub fn cjk_idx(&self, symbol: &str) -> Option<(u8, u8)> {
        let ch = symbol.chars().next()?;
        let (pixel_x, pixel_y) = self.cjk.get(&ch).copied()?;

        // Get dynamic sizes based on texture dimensions
        let cjk_char_size = layout::cjk_width() as u16;
        let cjk_y_start = layout::cjk_y_start() as u16;

        // Convert pixel coordinates to character grid position
        let char_col = pixel_x / cjk_char_size; // 0-127 (128 columns total)
        let char_row = (pixel_y - cjk_y_start) / cjk_char_size; // 0-31 (32 rows total)

        // Convert to block position (16 cols × 4 rows of blocks)
        let block_col = char_col / 8; // 0-15 (which column of blocks)
        let block_row = char_row / 8; // 0-3 (which row of blocks)
        let block = (block_row * 16 + block_col) as u8;

        // Calculate position within the block (8×8 grid)
        let in_block_col = char_col % 8; // 0-7
        let in_block_row = char_row % 8; // 0-7
        let idx = (in_block_row * 8 + in_block_col) as u8;

        // Return (block index 176-239, symbol index 0-63)
        Some((layout::CJK_BLOCK_START as u8 + block, idx))
    }

    /// Add CJK character mapping
    /// Used by CJK tool to update mappings at runtime
    pub fn add_cjk(&mut self, ch: char, pixel_x: u16, pixel_y: u16) {
        self.cjk.insert(ch, (pixel_x, pixel_y));
    }

    /// Iterate TUI entries: (symbol_string, (block, idx))
    pub fn iter_tui(&self) -> impl Iterator<Item = (&String, &(u8, u8))> {
        self.tui.iter()
    }

    /// Iterate Emoji entries: (symbol_string, (block, idx))
    pub fn iter_emoji(&self) -> impl Iterator<Item = (&String, &(u8, u8))> {
        self.emoji.iter()
    }

    /// Iterate CJK entries: (char, (pixel_x, pixel_y))
    pub fn iter_cjk(&self) -> impl Iterator<Item = (&char, &(u16, u16))> {
        self.cjk.iter()
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
// ASCII to PETSCII Conversion
// ============================================================================

/// Convert ASCII string to PETSCII PUA string.
///
/// Looks up each character in the sprite region's extras mapping.
/// If found, converts to PUA encoding; otherwise uses block 0 with ASCII code.
///
/// # Example
/// ```ignore
/// let petscii = ascii_to_petscii("HELLO");
/// buf.set_str(0, 0, petscii, style);  // Renders as PETSCII characters
/// ```
pub fn ascii_to_petscii(s: &str) -> String {
    let map = get_symbol_map();
    s.chars()
        .map(|ch| {
            // Look up in sprite extras mapping
            if let Some((block, idx)) = map.sprite_idx(&ch.to_string()) {
                cellsym_block(block, idx)
            } else {
                // Fallback: use block 0 with ASCII code as index
                cellsym_block(0, ch as u8)
            }
        })
        .collect()
}

// ============================================================================
// Texture Layout Constants and Utilities
// ============================================================================

/// Texture layout constants for 4096x4096 symbol texture
///
/// Linear symbol array layout:
/// - `[0, 40959]`: Sprite (160 blocks × 256 = 40960 symbols, 16×16px)
/// - `[40960, 43519]`: TUI (10 blocks × 256 = 2560 symbols, 16×32px)
/// - `[43520, 44287]`: Emoji (6 blocks × 128 = 768 symbols, 32×32px)
/// - `[44288, 48383]`: CJK (128 cols × 32 rows = 4096 symbols, 32×32px)
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

    pub const CJK_BLOCKS: u32 = 64; // 64 blocks (16 cols × 4 rows)
    pub const CJK_BLOCK_COLS: u32 = 16; // 16 columns of blocks
    pub const CJK_BLOCK_ROWS: u32 = 4; // 4 rows of blocks
    pub const CJK_SYMBOLS_PER_BLOCK: u32 = 64; // 8×8 chars per block
    pub const CJK_CHARS_PER_BLOCK_ROW: u32 = 8; // 8 rows of chars per block
    pub const CJK_CHARS_PER_BLOCK_COL: u32 = 8; // 8 columns of chars per block
    pub const CJK_TOTAL: u32 = CJK_BLOCKS * CJK_SYMBOLS_PER_BLOCK; // 4096

    // Total symbol count across all regions
    pub const TOTAL_SYMBOLS: u32 = SPRITE_TOTAL + TUI_TOTAL + EMOJI_TOTAL + CJK_TOTAL; // 48384

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
    pub const CJK_BLOCK_END: usize = 239; // 176 + 64 - 1 = 239 (64 blocks)

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
            PIXEL_SYM_WIDTH
                .get()
                .map(|v| *v as u32)
                .unwrap_or(DEFAULT_BASE_WIDTH)
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
            PIXEL_SYM_HEIGHT
                .get()
                .map(|v| *v as u32)
                .unwrap_or(DEFAULT_BASE_HEIGHT)
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
        layout::CJK_BASE
            + (texidx - layout::CJK_BLOCK_START) * layout::CJK_SYMBOLS_PER_BLOCK as usize
            + symidx
    } else if texidx >= layout::EMOJI_BLOCK_START {
        // Emoji blocks (170-175): base 43520 + block offset + symbol index
        layout::EMOJI_BASE
            + (texidx - layout::EMOJI_BLOCK_START) * layout::EMOJI_SYMBOLS_PER_BLOCK as usize
            + symidx
    } else if texidx >= layout::TUI_BLOCK_START {
        // TUI blocks (160-169): base 40960 + block offset + symbol index
        layout::TUI_BASE
            + (texidx - layout::TUI_BLOCK_START) * layout::TUI_SYMBOLS_PER_BLOCK as usize
            + symidx
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
        let total =
            layout::SPRITE_TOTAL + layout::TUI_TOTAL + layout::EMOJI_TOTAL + layout::CJK_TOTAL;
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
            // CJK region: 64 blocks (16 cols × 4 rows), each block 8×8 chars
            let idx = self.current - layout::CJK_BASE;
            let block = idx / layout::CJK_SYMBOLS_PER_BLOCK as usize; // 0-63
            let sym = idx % layout::CJK_SYMBOLS_PER_BLOCK as usize; // 0-63

            let block_col = block % layout::CJK_BLOCK_COLS as usize; // 0-15
            let block_row = block / layout::CJK_BLOCK_COLS as usize; // 0-3
            let sym_col = sym % layout::CJK_CHARS_PER_BLOCK_COL as usize; // 0-7
            let sym_row = sym / layout::CJK_CHARS_PER_BLOCK_COL as usize; // 0-7

            let cjk_w = layout::cjk_width();
            let cjk_h = layout::cjk_height();

            SymbolFrame {
                pixel_x: ((block_col * layout::CJK_CHARS_PER_BLOCK_COL as usize + sym_col) as u32)
                    * cjk_w,
                pixel_y: layout::cjk_y_start()
                    + ((block_row * layout::CJK_CHARS_PER_BLOCK_ROW as usize + sym_row) as u32)
                        * cjk_h,
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
    /// Deprecated: texture_size is now computed from actual loaded texture dimensions
    #[allow(dead_code)]
    texture_size: Option<u32>,
    regions: HashMap<String, RegionConfig>,
}

#[derive(Deserialize)]
struct RegionConfig {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    region_type: Option<String>,
    block_range: Option<[u8; 2]>,
    /// Deprecated: char_size is now computed dynamically from texture dimensions
    #[allow(dead_code)]
    char_size: Option<[u32; 2]>,
    chars_per_block: Option<u32>,
    symbols: Option<SymbolsValue>,
    extras: Option<HashMap<String, [u8; 2]>>,
    /// Deprecated: pixel_region is now computed dynamically from texture dimensions
    #[allow(dead_code)]
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
        assert_eq!(map.sprite_idx("@"), Some((0, 0)));
        // 'a' is the second
        assert_eq!(map.sprite_idx("a"), Some((0, 1)));
    }

    #[test]
    fn test_tui_lookup() {
        let map = SymbolMap::default();
        // Space is first in TUI
        assert_eq!(map.tui_idx(" "), Some((160, 0)));
    }

    #[test]
    fn test_emoji_lookup() {
        let map = SymbolMap::default();
        assert_eq!(map.emoji_idx("😀"), Some((170, 0)));
    }

    #[test]
    fn test_not_found() {
        let map = SymbolMap::default();
        // Use a rare character not in the 3500 common CJK set
        assert_eq!(map.cjk_idx("㊀"), None);
    }

    #[test]
    fn test_cjk_lookup() {
        let map = SymbolMap::default();
        // "一" is the first CJK character at grid position (0, 0)
        // block 176, idx 0
        assert_eq!(map.cjk_idx("一"), Some((176, 0)));

        // "中" is at grid position (26, 0)
        // block_col = 26/8 = 3, block_row = 0/8 = 0, block = 0*16+3 = 3
        // in_block_col = 26%8 = 2, in_block_row = 0%8 = 0, idx = 0*8+2 = 2
        // Result: block 176+3=179, idx 2
        assert_eq!(map.cjk_idx("中"), Some((179, 2)));

        // Test direct cjk_coords (still returns pixel coordinates)
        assert_eq!(map.cjk_coords('一'), Some((0, 3072)));
    }

    #[test]
    fn test_cjk_idx() {
        let map = SymbolMap::default();

        // "一" is at grid position (col=0, row=0)
        // New design: block = 176, idx = 0
        assert_eq!(map.cjk_idx("一"), Some((176, 0)));

        // "中" is at grid position (col=26, row=0)
        // block_col=3, block_row=0, block=3 -> 176+3=179
        // in_block_col=2, in_block_row=0, idx=2
        assert_eq!(map.cjk_idx("中"), Some((179, 2)));

        // Test that linear index calculation works for CJK
        // "一": linear index = CJK_BASE + (176-176) * 64 + 0 = 44288
        assert_eq!(calc_linear_index(176, 0), layout::CJK_BASE);

        // "中": linear index = CJK_BASE + (179-176) * 64 + 2 = 44288 + 192 + 2 = 44482
        assert_eq!(calc_linear_index(179, 2), layout::CJK_BASE + 194);
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
            panic!(
                "▇ should be in Sprite region via extras, sprite count: {}",
                map.stats().sprite_count
            );
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

        // CJK region (new design: 64 blocks, 64 symbols per block)
        assert_eq!(calc_linear_index(176, 0), 44288); // First CJK (block 176, idx 0)
        assert_eq!(calc_linear_index(176, 63), 44351); // End of first block
        assert_eq!(calc_linear_index(177, 0), 44352); // Start of second block
        assert_eq!(calc_linear_index(239, 63), 48383); // Last CJK (block 239, idx 63)
    }

    #[test]
    fn test_symbol_frame_iterator() {
        let frames: Vec<_> = iter_symbol_frames().collect();

        // Total count should be 48384
        let expected_total =
            layout::SPRITE_TOTAL + layout::TUI_TOTAL + layout::EMOJI_TOTAL + layout::CJK_TOTAL;
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
        assert_eq!(
            layout::tui_height(),
            layout::DEFAULT_BASE_HEIGHT * layout::TUI_HEIGHT_MULTIPLIER
        );
        assert_eq!(
            layout::emoji_width(),
            layout::DEFAULT_BASE_WIDTH * layout::EMOJI_SIZE_MULTIPLIER
        );
        assert_eq!(
            layout::emoji_height(),
            layout::DEFAULT_BASE_HEIGHT * layout::EMOJI_SIZE_MULTIPLIER
        );
        assert_eq!(
            layout::cjk_width(),
            layout::DEFAULT_BASE_WIDTH * layout::CJK_SIZE_MULTIPLIER
        );
        assert_eq!(
            layout::cjk_height(),
            layout::DEFAULT_BASE_HEIGHT * layout::CJK_SIZE_MULTIPLIER
        );

        // Test pixel positions
        assert_eq!(layout::sprite_y_start(), 0);
        assert_eq!(
            layout::tui_y_start(),
            (layout::SPRITE_BLOCKS / 16) * 16 * layout::DEFAULT_BASE_HEIGHT
        );
        assert_eq!(
            layout::emoji_x_start(),
            layout::TUI_BLOCKS * 16 * layout::DEFAULT_BASE_WIDTH
        );
    }

    // ====================================================================
    // LayeredSymbolMap tests
    // ====================================================================

    fn sample_layered_json() -> String {
        // Build JSON with actual Unicode characters (PUA U+F0000 needs surrogate pairs in JSON)
        let pua_char = '\u{F0000}'; // PUA sprite: block 0, idx 0
        let emoji_char = '\u{1F600}'; // 😀
        format!(
            r#"{{
            "version": 2,
            "layer_size": 2048,
            "layer_count": 2,
            "layer_files": ["layers/layer_0.png", "layers/layer_1.png"],
            "symbols": {{
                "A": {{
                    "w": 1, "h": 2,
                    "mip0": {{"layer": 0, "x": 64, "y": 0, "w": 64, "h": 128}},
                    "mip1": {{"layer": 0, "x": 1024, "y": 1024, "w": 32, "h": 64}},
                    "mip2": {{"layer": 1, "x": 0, "y": 0, "w": 16, "h": 32}}
                }},
                "{}": {{
                    "w": 1, "h": 1,
                    "mip0": {{"layer": 0, "x": 0, "y": 128, "w": 64, "h": 64}},
                    "mip1": {{"layer": 0, "x": 1024, "y": 1088, "w": 32, "h": 32}},
                    "mip2": {{"layer": 1, "x": 16, "y": 0, "w": 16, "h": 16}}
                }},
                "{}": {{
                    "w": 2, "h": 2,
                    "mip0": {{"layer": 0, "x": 0, "y": 192, "w": 128, "h": 128}},
                    "mip1": {{"layer": 0, "x": 1024, "y": 1120, "w": 64, "h": 64}},
                    "mip2": {{"layer": 1, "x": 32, "y": 0, "w": 32, "h": 32}}
                }},
                "中": {{
                    "w": 2, "h": 2,
                    "mip0": {{"layer": 0, "x": 128, "y": 192, "w": 128, "h": 128}},
                    "mip1": {{"layer": 0, "x": 1088, "y": 1120, "w": 64, "h": 64}},
                    "mip2": {{"layer": 1, "x": 64, "y": 0, "w": 32, "h": 32}}
                }}
            }}
        }}"#,
            pua_char, emoji_char
        )
    }

    #[test]
    fn test_layered_parse_basic() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        assert_eq!(map.layer_size, 2048);
        assert_eq!(map.layer_count, 2);
        assert_eq!(map.layer_files.len(), 2);
        assert_eq!(map.symbol_count(), 4);
    }

    #[test]
    fn test_layered_resolve_tui() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        let tile = map.resolve("A");
        assert_eq!(tile.cell_w, 1);
        assert_eq!(tile.cell_h, 2);
        // mip0: layer=0, x=64/2048, y=0/2048, w=64/2048, h=128/2048
        assert_eq!(tile.mips[0].layer, 0);
        assert!((tile.mips[0].x - 64.0 / 2048.0).abs() < 1e-6);
        assert!((tile.mips[0].y - 0.0).abs() < 1e-6);
        assert!((tile.mips[0].w - 64.0 / 2048.0).abs() < 1e-6);
        assert!((tile.mips[0].h - 128.0 / 2048.0).abs() < 1e-6);
    }

    #[test]
    fn test_layered_resolve_sprite_pua() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        // U+F0000 = PUA sprite (block 0, idx 0)
        let pua = "\u{F0000}";
        let tile = map.resolve(pua);
        assert_eq!(tile.cell_w, 1);
        assert_eq!(tile.cell_h, 1);
        assert_eq!(tile.mips[0].layer, 0);
        assert_eq!(tile.mips[2].layer, 1);
    }

    #[test]
    fn test_layered_resolve_emoji() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        let tile = map.resolve("\u{1F600}");
        assert_eq!(tile.cell_w, 2);
        assert_eq!(tile.cell_h, 2);
        // mip0: 128x128 at layer 0
        assert!((tile.mips[0].w - 128.0 / 2048.0).abs() < 1e-6);
        assert!((tile.mips[0].h - 128.0 / 2048.0).abs() < 1e-6);
    }

    #[test]
    fn test_layered_resolve_cjk() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        let tile = map.resolve("中");
        assert_eq!(tile.cell_w, 2);
        assert_eq!(tile.cell_h, 2);
        assert_eq!(tile.mips[1].layer, 0);
        assert!((tile.mips[1].w - 64.0 / 2048.0).abs() < 1e-6);
    }

    #[test]
    fn test_layered_resolve_unknown() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        let tile = map.resolve("NONEXISTENT");
        // Should return default tile
        assert_eq!(tile.cell_w, 1);
        assert_eq!(tile.cell_h, 1);
        assert!((tile.mips[0].w).abs() < 1e-6); // zero width = empty
    }

    #[test]
    fn test_layered_uv_range() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        // All UV coordinates should be in [0.0, 1.0]
        for tile in map.symbols.values() {
            for mip in &tile.mips {
                assert!(mip.x >= 0.0 && mip.x <= 1.0, "x out of range: {}", mip.x);
                assert!(mip.y >= 0.0 && mip.y <= 1.0, "y out of range: {}", mip.y);
                assert!(mip.w >= 0.0 && mip.w <= 1.0, "w out of range: {}", mip.w);
                assert!(mip.h >= 0.0 && mip.h <= 1.0, "h out of range: {}", mip.h);
                assert!(mip.x + mip.w <= 1.0 + 1e-6, "x+w out of range");
                assert!(mip.y + mip.h <= 1.0 + 1e-6, "y+h out of range");
            }
        }
    }

    #[test]
    fn test_layered_layer_bounds() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        // All layer indices should be < layer_count
        for tile in map.symbols.values() {
            for mip in &tile.mips {
                assert!(
                    (mip.layer as u32) < map.layer_count,
                    "layer {} >= layer_count {}",
                    mip.layer, map.layer_count
                );
            }
        }
    }

    #[test]
    fn test_layered_version_check() {
        let bad_json = r#"{"version": 1, "layer_size": 2048, "layer_count": 0, "layer_files": [], "symbols": {}}"#;
        assert!(LayeredSymbolMap::from_json(bad_json).is_err());
    }

    #[test]
    fn test_layered_stats() {
        let map = LayeredSymbolMap::from_json(&sample_layered_json()).unwrap();
        let stats = map.stats();
        assert_eq!(stats.layer_size, 2048);
        assert_eq!(stats.layer_count, 2);
        assert_eq!(stats.symbol_count, 4);
    }

    #[test]
    fn test_tile_size() {
        // Tile should be compact — verify it fits in a reasonable size
        assert!(std::mem::size_of::<Tile>() <= 64, "Tile too large: {} bytes", std::mem::size_of::<Tile>());
        // MipUV should be Copy
        let m = MipUV::default();
        let m2 = m; // Copy
        assert_eq!(m2.layer, 0);
    }
}
