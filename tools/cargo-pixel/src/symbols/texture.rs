// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Texture assembly and symbol map generation

use super::config::{LayeredTextureConfig, TextureConfig};
use super::font::MipBitmaps;
use image::{imageops, ImageBuffer, RgbaImage};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Draw sprites to texture
pub fn draw_sprites(texture: &mut RgbaImage, sprites: &[RgbaImage], cfg: &TextureConfig) {
    let mut sprite_idx = 0;

    for block_idx in 0..cfg.sprite_blocks {
        if sprite_idx >= sprites.len() {
            break;
        }

        let block_row = block_idx / cfg.blocks_per_row;
        let block_col = block_idx % cfg.blocks_per_row;
        let block_x = block_col * cfg.sprite_block_size;
        let block_y = block_row * cfg.sprite_block_size;

        for row in 0..16 {
            for col in 0..16 {
                if sprite_idx >= sprites.len() {
                    break;
                }

                let x = block_x + col * cfg.sprite_char_size;
                let y = block_y + row * cfg.sprite_char_size;

                imageops::overlay(texture, &sprites[sprite_idx], x as i64, y as i64);
                sprite_idx += 1;
            }
        }

        if (block_idx + 1) % 16 == 0 {
            println!(
                "  Drawing sprites: {}/{} blocks",
                block_idx + 1,
                cfg.sprite_blocks
            );
        }
    }

    println!("  Drew {} sprites", sprite_idx);
}

/// Draw TUI characters to texture
pub fn draw_tui(texture: &mut RgbaImage, tui_images: &[RgbaImage], cfg: &TextureConfig) {
    let mut tui_idx = 0;

    for block_idx in 0..cfg.tui_blocks_count {
        if tui_idx >= tui_images.len() {
            break;
        }

        let block_x = block_idx * cfg.tui_block_width;
        let block_y = cfg.tui_area_start_y;

        for row in 0..16 {
            for col in 0..16 {
                if tui_idx >= tui_images.len() {
                    break;
                }

                let x = block_x + col * cfg.tui_char_width;
                let y = block_y + row * cfg.tui_char_height;

                imageops::overlay(texture, &tui_images[tui_idx], x as i64, y as i64);
                tui_idx += 1;
            }
        }

        println!(
            "  Drawing TUI block {}",
            cfg.tui_blocks_start + block_idx
        );
    }

    println!("  Drew {} TUI characters", tui_idx);
}

/// Draw emojis to texture
pub fn draw_emojis(texture: &mut RgbaImage, emoji_images: &[RgbaImage], cfg: &TextureConfig) {
    let mut emoji_idx = 0;

    for block_idx in 0..cfg.emoji_blocks_count {
        if emoji_idx >= emoji_images.len() {
            break;
        }

        let block_x = cfg.emoji_area_start_x + block_idx * cfg.emoji_block_width;
        let block_y = cfg.emoji_area_start_y;

        for row in 0..16 {
            for col in 0..8 {
                // 8 emojis per row (32px each = 256px block)
                if emoji_idx >= emoji_images.len() {
                    break;
                }

                let x = block_x + col * cfg.emoji_char_size;
                let y = block_y + row * cfg.emoji_char_size;

                imageops::overlay(texture, &emoji_images[emoji_idx], x as i64, y as i64);
                emoji_idx += 1;
            }
        }

        println!(
            "  Drawing Emoji block {}",
            cfg.emoji_blocks_start + block_idx
        );
    }

    println!("  Drew {} emojis", emoji_idx);
}

/// Draw CJK characters to texture
pub fn draw_cjk(texture: &mut RgbaImage, cjk_images: &[RgbaImage], cfg: &TextureConfig) {
    let mut cjk_idx = 0;

    for (i, img) in cjk_images.iter().enumerate() {
        let col = i as u32 % cfg.cjk_grid_cols;
        let row = i as u32 / cfg.cjk_grid_cols;

        let x = col * cfg.cjk_char_size;
        let y = cfg.cjk_area_start_y + row * cfg.cjk_char_size;

        imageops::overlay(texture, img, x as i64, y as i64);
        cjk_idx += 1;

        if (i + 1) % 512 == 0 {
            println!("  Drawing CJK: {}/{}", i + 1, cjk_images.len());
        }
    }

    println!("  Drew {} CJK characters", cjk_idx);
}

/// Build symbol_map.json content
pub fn build_symbol_map(
    tui_chars: &[char],
    emojis: &[String],
    cjk_chars: &[char],
    cfg: &TextureConfig,
) -> Value {
    // Build TUI symbols string
    let tui_symbols: String = tui_chars.iter().collect();

    // Sprite extras (special character mappings)
    let sprite_extras: HashMap<&str, [u32; 2]> = [
        ("▇", [1, 209]),
        ("▒", [1, 94]),
        ("∙", [1, 122]),
        ("│", [1, 93]),
        ("┐", [1, 110]),
        ("╮", [1, 73]),
        ("┌", [1, 112]),
        ("╭", [1, 85]),
        ("└", [1, 109]),
        ("╰", [1, 74]),
        ("┘", [1, 125]),
        ("╯", [1, 75]),
        ("_", [2, 30]),
    ]
    .into_iter()
    .collect();

    // Sprite symbols (C64 character set)
    let sprite_symbols =
        "@abcdefghijklmnopqrstuvwxyz[£]↑← !\"#$%&'()*+,-./0123456789:;<=>?─ABCDEFGHIJKLMNOPQRSTUVWXYZ┼";

    // Build CJK mappings
    let cjk_mappings: HashMap<String, [u32; 2]> = cjk_chars
        .iter()
        .enumerate()
        .map(|(i, &ch)| {
            let col = i as u32 % cfg.cjk_grid_cols;
            let row = i as u32 / cfg.cjk_grid_cols;
            (ch.to_string(), [col, row])
        })
        .collect();

    json!({
        "version": 1,
        "regions": {
            "sprite": {
                "type": "block",
                "block_range": [0, cfg.sprite_blocks - 1],
                "chars_per_block": cfg.sprite_chars_per_block,
                "symbols": sprite_symbols,
                "extras": sprite_extras
            },
            "tui": {
                "type": "block",
                "block_range": [cfg.tui_blocks_start, cfg.tui_blocks_start + cfg.tui_blocks_count - 1],
                "chars_per_block": cfg.tui_chars_per_block,
                "symbols": tui_symbols
            },
            "emoji": {
                "type": "block",
                "block_range": [cfg.emoji_blocks_start, cfg.emoji_blocks_start + cfg.emoji_blocks_count - 1],
                "chars_per_block": cfg.emoji_chars_per_block,
                "symbols": emojis
            },
            "cjk": {
                "type": "grid",
                "grid_cols": cfg.cjk_grid_cols,
                "mappings": cjk_mappings
            }
        },
        "linear_index": {
            "sprite_base": cfg.linear_sprite_base,
            "sprite_total": cfg.sprite_blocks * cfg.sprite_chars_per_block,
            "tui_base": cfg.linear_tui_base,
            "tui_total": cfg.tui_blocks_count * cfg.tui_chars_per_block,
            "emoji_base": cfg.linear_emoji_base,
            "emoji_total": cfg.emoji_blocks_count * cfg.emoji_chars_per_block,
            "cjk_base": cfg.linear_cjk_base,
            "cjk_total": cfg.cjk_grid_cols * cfg.cjk_grid_rows
        }
    })
}

// ============================================================================
// Layered mode: DP shelf-packing + Texture2DArray generation
// ============================================================================

/// Result of packing symbols into layers
pub struct PackResult {
    pub layers: Vec<RgbaImage>,
    pub symbol_map: Value,
    pub symbol_count: usize,
}

/// A symbol to be packed, with its mipmap bitmaps and metadata
struct PackSymbol {
    key: String,       // Symbol key (PUA or Unicode string)
    cell_w: u8,        // Cell width (1 or 2)
    cell_h: u8,        // Cell height (1 or 2)
    mips: [RgbaImage; 3],
}

/// Shelf assignment for one layer: (height_index, count)
/// height_index: 0=128px, 1=64px, 2=32px, 3=16px
struct LayerConfig {
    shelves: Vec<(usize, u32)>,
}

/// Pixel heights corresponding to each height index
const SHELF_HEIGHTS: [u32; 4] = [128, 64, 32, 16];
/// DP units for each height (height / 16)
const SHELF_UNITS: [u16; 4] = [8, 4, 2, 1];

/// DP-optimized shelf filling for a single layer.
/// Finds the optimal combination of shelf heights to maximize utilization.
///
/// remaining[i] = number of shelf rows still needed for height index i
/// layer_size: texture layer size (e.g., 4096)
/// Returns: Vec<(height_index, count)> of shelves assigned to this layer
pub fn dp_fill_layer(remaining: &mut [u32; 4], layer_size: u32) -> Vec<(usize, u32)> {
    let capacity: usize = (layer_size / 16) as usize; // layer_size / base_unit

    // dp[c] = max fill units achievable with exactly capacity c
    // alloc[c][i] = how many of item i are used in the optimal solution for capacity c
    let mut dp = vec![0u32; capacity + 1];
    let mut alloc = vec![[0u32; 4]; capacity + 1];

    // Simple bounded knapsack: iterate over all items, try adding 1..=avail of each
    // Since capacity is relatively small and item types are few (4), brute force is fine
    for i in 0..4 {
        let u = SHELF_UNITS[i] as usize;
        let max_fit = (capacity / u) as u32;
        let avail = remaining[i].min(max_fit);
        if avail == 0 {
            continue;
        }

        // Process in reverse capacity order to avoid reuse within same item type
        // Use binary grouping for efficiency
        let mut k = 1u32;
        let mut left = avail;
        while left > 0 {
            let batch = k.min(left);
            let batch_units = u * batch as usize;
            for c in (batch_units..=capacity).rev() {
                let prev = c - batch_units;
                let new_fill = dp[prev] + batch_units as u32;
                if new_fill > dp[c] {
                    dp[c] = new_fill;
                    alloc[c] = alloc[prev];
                    alloc[c][i] += batch;
                }
            }
            left -= batch;
            k *= 2;
        }
    }

    // Find the best capacity that has maximum fill
    let best_c = (0..=capacity)
        .max_by_key(|&c| dp[c])
        .unwrap_or(0);

    // Build result from allocation
    let mut result = vec![];
    for i in 0..4 {
        let count = alloc[best_c][i];
        if count > 0 {
            result.push((i, count));
            remaining[i] -= count;
        }
    }
    result
}

/// Pack all symbols into layers using iterative DP
fn pack_all_layers(demands: &[u32; 4], layer_size: u32) -> Vec<LayerConfig> {
    let mut remaining = *demands;
    let mut layers = vec![];

    while remaining.iter().any(|&r| r > 0) {
        let shelves = dp_fill_layer(&mut remaining, layer_size);
        if shelves.is_empty() {
            break; // Safety: shouldn't happen if demands are valid
        }
        layers.push(LayerConfig { shelves });
    }
    layers
}

/// Generate PUA symbol key for a sprite (block, idx)
fn pua_symbol(block: u32, idx: u32) -> String {
    let codepoint = 0xF0000 + block * 256 + idx;
    // Supplementary Private Use Area-A: U+F0000..U+FFFFF
    char::from_u32(codepoint)
        .map(|c| c.to_string())
        .unwrap_or_default()
}

/// Main entry point: pack all symbols into layered Texture2DArray format
pub fn pack_layered(
    sprites: &[RgbaImage],
    tui_bitmaps: &[MipBitmaps],
    emoji_bitmaps: &[MipBitmaps],
    cjk_bitmaps: &[MipBitmaps],
    tui_chars: &[char],
    emojis: &[String],
    cjk_chars: &[char],
    lcfg: &LayeredTextureConfig,
) -> PackResult {
    let layer_size = lcfg.layer_size;

    // Step 1: Build list of all symbol bitmaps with metadata
    let mut all_symbols: Vec<PackSymbol> = Vec::new();

    // Sprites: generate mip levels from 16×16 sources
    for (i, src) in sprites.iter().enumerate() {
        let block = (i / 256) as u32;
        let idx = (i % 256) as u32;
        let mips = super::font::generate_sprite_mips(src, &lcfg.sprite);
        all_symbols.push(PackSymbol {
            key: pua_symbol(block, idx),
            cell_w: 1,
            cell_h: 1,
            mips: mips.levels,
        });
    }

    // TUI
    for (i, mip) in tui_bitmaps.iter().enumerate() {
        if i < tui_chars.len() {
            all_symbols.push(PackSymbol {
                key: tui_chars[i].to_string(),
                cell_w: 1,
                cell_h: 2,
                mips: mip.levels.clone(),
            });
        }
    }

    // Emoji
    for (i, mip) in emoji_bitmaps.iter().enumerate() {
        if i < emojis.len() {
            all_symbols.push(PackSymbol {
                key: emojis[i].clone(),
                cell_w: 2,
                cell_h: 2,
                mips: mip.levels.clone(),
            });
        }
    }

    // CJK
    for (i, mip) in cjk_bitmaps.iter().enumerate() {
        if i < cjk_chars.len() {
            all_symbols.push(PackSymbol {
                key: cjk_chars[i].to_string(),
                cell_w: 2,
                cell_h: 2,
                mips: mip.levels.clone(),
            });
        }
    }

    let symbol_count = all_symbols.len();
    println!("  Total symbols to pack: {}", symbol_count);

    // Step 2: Group mip bitmaps by height for shelf demand calculation
    // Collect all (symbol_index, mip_level, bitmap) grouped by pixel height
    struct MipEntry {
        sym_idx: usize,
        mip_level: usize,
        width: u32,
        #[allow(dead_code)]
        height: u32,
    }

    let mut entries_by_height: [Vec<MipEntry>; 4] = [vec![], vec![], vec![], vec![]];

    for (sym_idx, sym) in all_symbols.iter().enumerate() {
        for mip_level in 0..3 {
            let (w, h) = sym.mips[mip_level].dimensions();
            let height_idx = match h {
                128 => 0,
                64 => 1,
                32 => 2,
                16 => 3,
                _ => panic!("Unexpected mip height: {}", h),
            };
            entries_by_height[height_idx].push(MipEntry {
                sym_idx,
                mip_level,
                width: w,
                height: h,
            });
        }
    }

    // Step 3: Calculate shelf demands
    // For each height, count how many shelf rows are needed
    let mut demands = [0u32; 4];
    for (hi, entries) in entries_by_height.iter().enumerate() {
        if entries.is_empty() {
            continue;
        }
        let h = SHELF_HEIGHTS[hi];
        // All entries with the same height have the same width within a type,
        // but different types may have different widths. Calculate total width needed.
        let mut total_width: u64 = 0;
        for e in entries {
            total_width += e.width as u64;
        }
        // Each row is layer_size wide
        demands[hi] = ((total_width + layer_size as u64 - 1) / layer_size as u64) as u32;
        println!(
            "  Height {}px: {} entries, {} rows needed",
            h,
            entries.len(),
            demands[hi]
        );
    }

    let total_height: u64 = demands
        .iter()
        .zip(SHELF_HEIGHTS.iter())
        .map(|(&d, &h)| d as u64 * h as u64)
        .sum();
    let min_layers = ((total_height + layer_size as u64 - 1) / layer_size as u64) as usize;
    println!(
        "  Total height: {} px, theoretical min layers: {}",
        total_height, min_layers
    );

    // Step 4: DP packing to determine layer configs
    let layer_configs = pack_all_layers(&demands, layer_size);
    println!("  DP result: {} layers", layer_configs.len());

    // Step 5: Create layer images and place symbols
    let mut layer_images: Vec<RgbaImage> = Vec::with_capacity(layer_configs.len());
    // symbol_placements[sym_idx][mip_level] = (layer_idx, x, y)
    let mut placements: Vec<[(u32, u32, u32); 3]> = vec![[(0, 0, 0); 3]; all_symbols.len()];

    // Track consumption cursors per height index
    let mut consumed: [usize; 4] = [0; 4]; // index into entries_by_height

    for (layer_idx, layer_cfg) in layer_configs.iter().enumerate() {
        let mut layer_img: RgbaImage = ImageBuffer::new(layer_size, layer_size);
        let mut cur_y: u32 = 0;

        for &(height_idx, row_count) in &layer_cfg.shelves {
            let h = SHELF_HEIGHTS[height_idx];
            let entries = &entries_by_height[height_idx];

            for _row in 0..row_count {
                let mut cur_x: u32 = 0;

                while consumed[height_idx] < entries.len() {
                    let e = &entries[consumed[height_idx]];
                    if cur_x + e.width > layer_size {
                        break; // row full
                    }

                    // Place this bitmap
                    let bitmap = &all_symbols[e.sym_idx].mips[e.mip_level];
                    imageops::overlay(&mut layer_img, bitmap, cur_x as i64, cur_y as i64);
                    placements[e.sym_idx][e.mip_level] =
                        (layer_idx as u32, cur_x, cur_y);

                    cur_x += e.width;
                    consumed[height_idx] += 1;
                }

                cur_y += h;
            }
        }

        layer_images.push(layer_img);
    }

    // Step 6: Generate layered_symbol_map.json
    let mut symbols_json = serde_json::Map::new();
    for (sym_idx, sym) in all_symbols.iter().enumerate() {
        let mut mip_entries = serde_json::Map::new();
        for mip_level in 0..3 {
            let (layer, x, y) = placements[sym_idx][mip_level];
            let (w, h) = sym.mips[mip_level].dimensions();
            mip_entries.insert(
                format!("mip{}", mip_level),
                json!({
                    "layer": layer,
                    "x": x,
                    "y": y,
                    "w": w,
                    "h": h,
                }),
            );
        }
        mip_entries.insert("w".to_string(), json!(sym.cell_w));
        mip_entries.insert("h".to_string(), json!(sym.cell_h));
        symbols_json.insert(sym.key.clone(), Value::Object(mip_entries));
    }

    let layer_files: Vec<String> = (0..layer_images.len())
        .map(|i| format!("layers/layer_{}.png", i))
        .collect();

    // cell_pixel_size: screen pixels per 1×1 cell at ratio=1.0
    // Derived from sprite mip1 width (the default rendering resolution)
    let cell_pixel_size = lcfg.sprite.levels[1].width;

    let symbol_map = json!({
        "version": 2,
        "layer_size": layer_size,
        "layer_count": layer_images.len(),
        "cell_pixel_size": cell_pixel_size,
        "layer_files": layer_files,
        "symbols": symbols_json,
    });

    PackResult {
        layers: layer_images,
        symbol_map,
        symbol_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================
    // dp_fill_layer: single layer DP tests
    // =====================================================

    // Test layer size for DP tests (use 2048 for original test cases)
    const TEST_LAYER_SIZE: u32 = 2048;

    #[test]
    fn test_dp_fill_single_type() {
        // Only h64 shelves, 16 rows = 16×4 = 64 units
        let mut remaining = [0u32, 16, 0, 0];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        let total_units: u16 = result
            .iter()
            .map(|&(idx, count)| SHELF_UNITS[idx] * count as u16)
            .sum();
        assert_eq!(total_units, 64);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_exact_capacity() {
        // 8×h128(64) + 16×h64(64) = 128 units exactly
        let mut remaining = [8u32, 16, 0, 0];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        let total_units: u16 = result
            .iter()
            .map(|&(idx, count)| SHELF_UNITS[idx] * count as u16)
            .sum();
        assert_eq!(total_units, 128);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_mixed_all_heights() {
        // 4×h128 + 8×h64 + 16×h32 + 32×h16 = 32+32+32+32 = 128
        let mut remaining = [4u32, 8, 16, 32];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        let total_units: u16 = result
            .iter()
            .map(|&(idx, count)| SHELF_UNITS[idx] * count as u16)
            .sum();
        assert_eq!(total_units, 128);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_overflow_to_next_layer() {
        // 20×h128 = 160 units > 128, should only take 16
        let mut remaining = [20u32, 0, 0, 0];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        let total_units: u16 = result
            .iter()
            .map(|&(idx, count)| SHELF_UNITS[idx] * count as u16)
            .sum();
        assert_eq!(total_units, 128);
        assert_eq!(remaining[0], 4);
    }

    #[test]
    fn test_dp_fill_prioritize_large_shelves() {
        // 1×h128(8) + 100×h16(100) = 108 units
        let mut remaining = [1u32, 0, 0, 100];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        let total_units: u16 = result
            .iter()
            .map(|&(idx, count)| SHELF_UNITS[idx] * count as u16)
            .sum();
        assert_eq!(total_units, 108);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    #[test]
    fn test_dp_fill_empty_demand() {
        let mut remaining = [0u32, 0, 0, 0];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        assert!(result.is_empty());
    }

    #[test]
    fn test_dp_fill_minimal() {
        // Only 1 row of h16
        let mut remaining = [0u32, 0, 0, 1];
        let result = dp_fill_layer(&mut remaining, TEST_LAYER_SIZE);
        let total_units: u16 = result
            .iter()
            .map(|&(idx, count)| SHELF_UNITS[idx] * count as u16)
            .sum();
        assert_eq!(total_units, 1);
        assert_eq!(remaining, [0, 0, 0, 0]);
    }

    // =====================================================
    // pack_all_layers: multi-layer packing tests
    // =====================================================

    #[test]
    fn test_pack_single_layer() {
        let demands = [16u32, 0, 0, 0]; // 16×h128 = 128 units = 1 layer
        let layers = pack_all_layers(&demands, TEST_LAYER_SIZE);
        assert_eq!(layers.len(), 1);
    }

    #[test]
    fn test_pack_two_layers() {
        let demands = [17u32, 0, 0, 0]; // 17×h128: layer1=16, layer2=1
        let layers = pack_all_layers(&demands, TEST_LAYER_SIZE);
        assert_eq!(layers.len(), 2);
    }

    #[test]
    fn test_pack_zero_waste() {
        // 10×8 + 20×4 + 40×2 + 80×1 = 320, ceil(320/128) = 3
        let demands = [10u32, 20, 40, 80];
        let layers = pack_all_layers(&demands, TEST_LAYER_SIZE);
        assert_eq!(layers.len(), 3);
    }

    #[test]
    fn test_pack_full_production_scenario() {
        // h128:384 h64:1472 h32:736 h16:320
        // For 2048 layer: capacity=128 units
        // total = 384×8+1472×4+736×2+320×1 = 10752, ceil(10752/128) = 84
        let demands = [384u32, 1472, 736, 320];
        let layers = pack_all_layers(&demands, TEST_LAYER_SIZE);
        assert_eq!(layers.len(), 84);
    }

    #[test]
    fn test_pack_level1_only() {
        // h64:192 h32:640 → 192×4+640×2 = 2048, ceil(2048/128) = 16
        let demands = [0u32, 192, 640, 0];
        let layers = pack_all_layers(&demands, TEST_LAYER_SIZE);
        assert_eq!(layers.len(), 16);
    }

    #[test]
    fn test_pack_typical_app() {
        // h128:84 h64:74 h32:37 h16:8
        // total = 84×8+74×4+37×2+8×1 = 672+296+74+8 = 1050
        // ceil(1050/128) = 9
        let demands = [84u32, 74, 37, 8];
        let layers = pack_all_layers(&demands, TEST_LAYER_SIZE);
        assert_eq!(layers.len(), 9);
    }

    // =====================================================
    // UV coordinate calculation tests
    // =====================================================

    #[test]
    fn test_shelf_placement_coordinates() {
        let layer_size = 2048u32;
        // 128×128 symbol at (0, 0)
        let uv_w = 128.0 / layer_size as f32;
        let uv_h = 128.0 / layer_size as f32;
        assert!((uv_w - 0.0625).abs() < 1e-6);
        assert!((uv_h - 0.0625).abs() < 1e-6);

        // 16×16 symbol
        let uv_w_16 = 16.0 / layer_size as f32;
        assert!((uv_w_16 - 0.0078125).abs() < 1e-6);
    }

    #[test]
    fn test_symbols_per_row() {
        assert_eq!(2048 / 128, 16);
        assert_eq!(2048 / 64, 32);
        assert_eq!(2048 / 32, 64);
        assert_eq!(2048 / 16, 128);
    }

    // =====================================================
    // PUA symbol key generation tests
    // =====================================================

    #[test]
    fn test_pua_symbol_generation() {
        // Block 0, idx 0 → U+F0000
        let s = pua_symbol(0, 0);
        assert_eq!(s.chars().next().unwrap() as u32, 0xF0000);

        // Block 0, idx 255 → U+F00FF
        let s = pua_symbol(0, 255);
        assert_eq!(s.chars().next().unwrap() as u32, 0xF00FF);

        // Block 1, idx 0 → U+F0100
        let s = pua_symbol(1, 0);
        assert_eq!(s.chars().next().unwrap() as u32, 0xF0100);

        // Block 159, idx 255 → U+F9FFF
        let s = pua_symbol(159, 255);
        assert_eq!(s.chars().next().unwrap() as u32, 0xF9FFF);
    }
}
