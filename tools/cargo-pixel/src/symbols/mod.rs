// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Symbol texture generator - generates SDF texture atlas for GPU rendering
// Replaces tools/symbols/generate_symbols.py

mod config;
mod edt;
mod font;
mod parser;
mod sprite;
mod texture;

pub use config::TextureConfig;

use clap::ArgMatches;
use image::{ImageBuffer, RgbaImage};
use std::path::Path;

/// Main entry point for the symbols command
pub fn generate_symbols(sub_m: &ArgMatches) {
    let size = sub_m
        .get_one::<String>("size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8192);

    let pxrange = sub_m
        .get_one::<String>("pxrange")
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let text_padding = sub_m
        .get_one::<String>("padding")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.92);

    let output_dir = sub_m
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or(".");

    let font_path = sub_m.get_one::<String>("font").map(|s| s.as_str());

    println!("\n{}", "=".repeat(70));
    println!(
        "Generating {}x{} symbols.png and symbol_map.json",
        size, size
    );
    println!("{}", "=".repeat(70));

    // Initialize configuration
    let cfg = match TextureConfig::new(size) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    println!("  Scale factor: {}x", cfg.scale);
    println!("  SDF pxrange: {}", pxrange);
    println!("  Text padding: {}", text_padding);

    // Find input files
    let script_dir = find_symbols_dir();
    if script_dir.is_none() {
        eprintln!("Error: Cannot find tools/symbols directory");
        return;
    }
    let script_dir = script_dir.unwrap();

    println!("\nChecking input files...");

    // Check C64 source files
    let c64_sources = [
        script_dir.join("c64l.png"),
        script_dir.join("c64u.png"),
        script_dir.join("c64e1.png"),
        script_dir.join("c64e2.png"),
    ];

    for src in &c64_sources {
        if !src.exists() {
            eprintln!("Error: Cannot find {}", src.display());
            return;
        }
        println!("  ✓ {}", src.file_name().unwrap().to_string_lossy());
    }

    // Check TUI file
    let tui_path = script_dir.join("tui.txt");
    if !tui_path.exists() {
        eprintln!("Error: Cannot find tui.txt");
        return;
    }
    println!("  ✓ tui.txt");

    // Check CJK file (optional)
    let cjk_path = script_dir.join("3500C.txt");
    let has_cjk = cjk_path.exists();
    if has_cjk {
        println!("  ✓ 3500C.txt");
    } else {
        println!("  ⚠ 3500C.txt (optional, not found)");
    }

    // Parse input files
    println!("\nParsing tui.txt...");
    let (tui_chars, emojis) = parser::parse_tui_txt(&tui_path);
    println!("  Found {} TUI chars, {} emojis", tui_chars.len(), emojis.len());

    let cjk_chars = if has_cjk {
        println!("\nParsing 3500C.txt...");
        let chars = parser::parse_cjk_txt(&cjk_path);
        println!("  Found {} CJK chars", chars.len());
        chars
    } else {
        Vec::new()
    };

    // Create texture
    println!("\nCreating {}x{} texture...", cfg.size, cfg.size);
    let mut texture: RgbaImage = ImageBuffer::new(cfg.size as u32, cfg.size as u32);

    // Load sprites
    println!("\nLoading Sprite symbols...");
    let all_sprites = sprite::load_all_sprites(&c64_sources, &cfg);
    println!("  Loaded {} sprites", all_sprites.len());

    // Render TUI characters
    println!("\nRendering TUI characters (SDF, pxrange={})...", pxrange);
    let tui_images = font::render_tui_chars(&tui_chars, &cfg, pxrange, text_padding, font_path);
    println!("  Generated {} TUI images", tui_images.len());

    // Render Emojis (bitmap mode for color emoji)
    println!("\nRendering Emojis (bitmap)...");
    let emoji_images = font::render_emojis(&emojis, &cfg);
    println!("  Generated {} emoji images", emoji_images.len());

    // Render CJK characters
    println!("\nRendering CJK characters (SDF, pxrange={})...", pxrange);
    let cjk_images = font::render_cjk_chars(&cjk_chars, &cfg, pxrange, text_padding);
    println!("  Generated {} CJK images", cjk_images.len());

    // Assemble texture
    println!("\nAssembling texture...");
    texture::draw_sprites(&mut texture, &all_sprites, &cfg);
    texture::draw_tui(&mut texture, &tui_images, &cfg);
    texture::draw_emojis(&mut texture, &emoji_images, &cfg);
    texture::draw_cjk(&mut texture, &cjk_images, &cfg);

    // Save output files
    let output_path = Path::new(output_dir);
    // Always use symbols.png and symbol_map.json
    let png_name = "symbols.png";
    let json_name = "symbol_map.json";

    let png_path = output_path.join(png_name);
    let json_path = output_path.join(json_name);

    println!("\nSaving texture to {}...", png_path.display());
    if let Err(e) = texture.save(&png_path) {
        eprintln!("Error saving texture: {}", e);
        return;
    }

    println!("Generating {}...", json_path.display());
    let symbol_map = texture::build_symbol_map(&tui_chars, &emojis, &cjk_chars, &cfg);
    if let Err(e) = std::fs::write(&json_path, serde_json::to_string_pretty(&symbol_map).unwrap()) {
        eprintln!("Error saving symbol map: {}", e);
        return;
    }

    // Print summary
    println!("\n{}", "=".repeat(70));
    println!("Complete!");
    println!("{}", "=".repeat(70));
    println!("Texture size: {}×{} (scale: {}x)", cfg.size, cfg.size, cfg.scale);
    println!("\nRegion layout:");
    println!(
        "  Sprite (Block 0-{}): {} chars",
        cfg.sprite_blocks - 1,
        all_sprites.len()
    );
    println!(
        "  TUI (Block {}-{}): {} chars",
        cfg.tui_blocks_start,
        cfg.tui_blocks_start + cfg.tui_blocks_count - 1,
        tui_images.len()
    );
    println!(
        "  Emoji (Block {}-{}): {} chars",
        cfg.emoji_blocks_start,
        cfg.emoji_blocks_start + cfg.emoji_blocks_count - 1,
        emoji_images.len()
    );
    println!(
        "  CJK (y={}-{}): {} chars",
        cfg.cjk_area_start_y,
        cfg.size - 1,
        cjk_images.len()
    );
    println!("\nOutput files:");
    println!("  {}", png_path.display());
    println!("  {}", json_path.display());
}

/// Find the tools/symbols directory
fn find_symbols_dir() -> Option<std::path::PathBuf> {
    // Try current directory first
    let cwd = std::env::current_dir().ok()?;

    // Check if we're in rust_pixel root
    let symbols_dir = cwd.join("tools/symbols");
    if symbols_dir.exists() {
        return Some(symbols_dir);
    }

    // Check parent directories
    let mut dir = cwd.as_path();
    while let Some(parent) = dir.parent() {
        let symbols_dir = parent.join("tools/symbols");
        if symbols_dir.exists() {
            return Some(symbols_dir);
        }
        dir = parent;
    }

    None
}
