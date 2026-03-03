// RustPixel
// copyright zipxing@hotmail.com 2022～2026
//
// Symbol texture generator - generates layered Texture2DArray with mipmap bitmaps
// Standalone tool, invoked by: cargo pixel symbols [options]

pub mod config;
pub mod edt;
pub mod font;
pub mod parser;
pub mod sprite;
pub mod texture;

use config::LayeredTextureConfig;
use std::path::Path;

use clap::{Arg, Command};

fn main() {
    // Internal subprocess mode for emoji rendering.
    // Apple Color Emoji + CTLineDraw can bus-error in certain terminal
    // process contexts on macOS, but works fine in a child process.
    #[cfg(target_os = "macos")]
    if std::env::var("_PIXEL_RENDER_EMOJI").is_ok() {
        font::emoji_subprocess_main();
        return;
    }

    let matches = Command::new("symbols")
        .about("Generate layered Texture2DArray with mipmap bitmaps for GPU rendering")
        .arg(
            Arg::new("padding")
                .long("padding")
                .help("Text character scale factor 0~1 (default: 0.92)")
                .default_value("0.92"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output directory (default: current directory)")
                .default_value("."),
        )
        .arg(
            Arg::new("font")
                .short('f')
                .long("font")
                .help("Path to TUI font file (TTF/OTF)"),
        )
        .get_matches();

    generate_symbols(&matches);
}

/// Generate layered Texture2DArray format (multi-resolution bitmap)
fn generate_symbols(sub_m: &clap::ArgMatches) {
    let text_padding = sub_m
        .get_one::<String>("padding")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.92);

    let output_dir = sub_m
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or(".");

    let font_path = sub_m.get_one::<String>("font").map(|s| s.as_str());

    let lcfg = LayeredTextureConfig::new();

    println!("\n{}", "=".repeat(70));
    println!(
        "Generating layered Texture2DArray ({}x{} layers)",
        lcfg.layer_size, lcfg.layer_size
    );
    println!("{}", "=".repeat(70));
    println!("  Layer size: {}x{}", lcfg.layer_size, lcfg.layer_size);
    println!("  Mipmap levels: 3 (high/mid/low)");
    println!("  Sprite: {:?}", lcfg.sprite.levels);
    println!("  TUI: {:?}", lcfg.tui.levels);
    println!("  Emoji: {:?}", lcfg.emoji.levels);
    println!("  CJK: {:?}", lcfg.cjk.levels);

    // Find input files
    let script_dir = find_symbols_dir();
    if script_dir.is_none() {
        eprintln!("Error: Cannot find tools/symbols directory");
        return;
    }
    let script_dir = script_dir.unwrap();

    println!("\nResource directory: {}", script_dir.display());
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
        println!("  ✓ {}", src.display());
    }

    // Check TUI file
    let tui_path = script_dir.join("tui.txt");
    if !tui_path.exists() {
        eprintln!("Error: Cannot find {}", tui_path.display());
        return;
    }
    println!("  ✓ {}", tui_path.display());

    // Check CJK file (optional)
    let cjk_path = script_dir.join("3500C.txt");
    let has_cjk = cjk_path.exists();
    if has_cjk {
        println!("  ✓ {}", cjk_path.display());
    } else {
        println!("  ⚠ {} (optional, not found)", cjk_path.display());
    }

    // Parse input files
    println!("\nParsing tui.txt...");
    let (tui_chars, emojis) = parser::parse_tui_txt(&tui_path);
    println!(
        "  Found {} TUI chars, {} emojis",
        tui_chars.len(),
        emojis.len()
    );

    let cjk_chars = if has_cjk {
        println!("\nParsing 3500C.txt...");
        let chars = parser::parse_cjk_txt(&cjk_path);
        println!("  Found {} CJK chars", chars.len());
        chars
    } else {
        Vec::new()
    };

    // Load sprites (use Level 0 sprite size as output size)
    println!("\nLoading Sprite symbols...");
    let sprite_output_size = lcfg.sprite.levels[0].width;
    let all_sprites = sprite::load_all_sprites(&c64_sources, sprite_output_size);
    println!("  Loaded {} sprites", all_sprites.len());

    // Render multi-resolution bitmaps
    println!("\nRendering multi-resolution bitmaps...");

    // Render TUI characters (bitmap, no SDF)
    println!("  Rendering TUI characters (bitmap, 3 mipmap levels)...");
    let tui_bitmaps = font::render_tui_bitmaps(&tui_chars, &lcfg, text_padding, font_path);
    println!("  Generated {} TUI bitmap sets", tui_bitmaps.len());

    // Render Emojis (bitmap)
    println!("  Rendering Emojis (bitmap, 3 mipmap levels)...");
    let emoji_bitmaps = font::render_emoji_bitmaps(&emojis, &lcfg);
    println!("  Generated {} emoji bitmap sets", emoji_bitmaps.len());

    // Render CJK characters (bitmap, no SDF)
    println!("  Rendering CJK characters (bitmap, 3 mipmap levels)...");
    let cjk_bitmaps = font::render_cjk_bitmaps(&cjk_chars, &lcfg, text_padding);
    println!("  Generated {} CJK bitmap sets", cjk_bitmaps.len());

    // Pack into layers using DP shelf-packing
    println!("\nPacking into layers (DP shelf-packing)...");
    let pack_result = texture::pack_layered(
        &all_sprites,
        &tui_bitmaps,
        &emoji_bitmaps,
        &cjk_bitmaps,
        &tui_chars,
        &emojis,
        &cjk_chars,
        &lcfg,
    );

    // Save output
    let output_path = Path::new(output_dir);
    let layers_dir = output_path.join("layers");
    std::fs::create_dir_all(&layers_dir).unwrap_or_else(|e| {
        eprintln!("Error creating layers directory: {}", e);
    });

    println!("\nSaving {} layer PNGs...", pack_result.layers.len());
    for (i, layer_img) in pack_result.layers.iter().enumerate() {
        let layer_path = layers_dir.join(format!("layer_{}.png", i));
        if let Err(e) = layer_img.save(&layer_path) {
            eprintln!("Error saving layer {}: {}", i, e);
            return;
        }
    }

    let json_path = output_path.join("layered_symbol_map.json");
    println!("Saving {}...", json_path.display());
    if let Err(e) = std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&pack_result.symbol_map).unwrap(),
    ) {
        eprintln!("Error saving symbol map: {}", e);
        return;
    }

    // Print summary
    println!("\n{}", "=".repeat(70));
    println!("Complete!");
    println!("{}", "=".repeat(70));
    println!("Layer size: {}x{}", lcfg.layer_size, lcfg.layer_size);
    println!("Total layers: {}", pack_result.layers.len());
    println!("Total symbols: {}", pack_result.symbol_count);
    println!("  Sprites: {}", all_sprites.len());
    println!("  TUI: {}", tui_bitmaps.len());
    println!("  Emoji: {}", emoji_bitmaps.len());
    println!("  CJK: {}", cjk_bitmaps.len());
    println!("\nOutput files:");
    for i in 0..pack_result.layers.len() {
        println!("  {}", layers_dir.join(format!("layer_{}.png", i)).display());
    }
    println!("  {}", json_path.display());
}

/// Find the tools/symbols directory (contains input data files: tui.txt, 3500C.txt, c64*.png)
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
