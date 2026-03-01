// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Texture assembly and symbol map generation

use super::config::TextureConfig;
use image::{imageops, RgbaImage};
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
