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
        .arg(
            Arg::new("custom")
                .short('c')
                .long("custom")
                .help("Directory containing custom PNG images to slice into sprites (32x32 tiles)"),
        )
        .arg(
            Arg::new("extract")
                .short('e')
                .long("extract")
                .help("Extract sprites from image(s) with adaptive binarization and deduplication. Accepts a single image file or a directory of PNGs."),
        )
        .arg(
            Arg::new("tile-size")
                .short('t')
                .long("tile-size")
                .help("Tile size for --extract mode (default: 8)")
                .default_value("8"),
        )
        .arg(
            Arg::new("dedup-threshold")
                .short('d')
                .long("dedup-threshold")
                .help("Hamming distance threshold for deduplication in --extract mode (default: 2). 0=exact match, 1-2=strict, 3-5=moderate, 6+=aggressive")
                .default_value("2"),
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
    let mut all_sprites = sprite::load_all_sprites(&c64_sources, sprite_output_size);
    println!("  Loaded {} C64 sprites", all_sprites.len());

    // Load custom sprites if --custom is specified
    let custom_dir = sub_m.get_one::<String>("custom").map(|s| s.as_str());
    let custom_sprite_sets = if let Some(custom_path) = custom_dir {
        let custom_dir_path = Path::new(custom_path);
        if custom_dir_path.exists() && custom_dir_path.is_dir() {
            println!("\nLoading custom sprites from {}...", custom_path);
            let (custom_sprites, sprite_sets) =
                sprite::load_custom_sprites(custom_dir_path, sprite_output_size, all_sprites.len());
            println!("  Loaded {} custom sprites from {} images", custom_sprites.len(), sprite_sets.len());
            all_sprites.extend(custom_sprites);
            sprite_sets
        } else {
            eprintln!("Warning: Custom directory '{}' not found or not a directory", custom_path);
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Extract sprites from image(s) with deduplication if --extract is specified
    let extract_path = sub_m.get_one::<String>("extract").map(|s| s.as_str());
    let tile_size: u32 = sub_m
        .get_one::<String>("tile-size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let dedup_threshold: u32 = sub_m
        .get_one::<String>("dedup-threshold")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    let extracted_sprite_sets = if let Some(extract_src) = extract_path {
        let extract_path = Path::new(extract_src);
        let mut image_files: Vec<std::path::PathBuf> = Vec::new();

        if extract_path.is_file() {
            image_files.push(extract_path.to_path_buf());
        } else if extract_path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(extract_path) {
                let mut files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext.to_ascii_lowercase() == "png")
                            .unwrap_or(false)
                    })
                    .collect();
                files.sort_by_key(|e| e.path());
                image_files.extend(files.into_iter().map(|e| e.path()));
            }
        } else {
            eprintln!("Warning: Extract path '{}' not found", extract_src);
        }

        if !image_files.is_empty() {
            println!(
                "\nExtracting sprites (tile={}x{}, dedup threshold={})...",
                tile_size, tile_size, dedup_threshold
            );
        }

        let mut sets = Vec::new();
        for img_path in &image_files {
            if let Some((sprites, sprite_set)) = sprite::load_extracted_sprites(
                img_path,
                tile_size,
                dedup_threshold,
                sprite_output_size,
                all_sprites.len(),
            ) {
                all_sprites.extend(sprites);
                sets.push(sprite_set);
            }
        }
        if !sets.is_empty() {
            let total_unique: usize = sets.iter().map(|s| s.unique_count).sum();
            let total_orig: usize = sets.iter().map(|s| s.original_count).sum();
            println!(
                "  Extracted {} unique sprites from {} total tiles across {} images",
                total_unique, total_orig, sets.len()
            );
        }
        sets
    } else {
        Vec::new()
    };

    println!("  Total sprites: {}", all_sprites.len());

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
    // Custom format: header pretty, symbols one per line for balance of size and readability
    let map = &pack_result.symbol_map;
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str(&format!("  \"version\": {},\n", map["version"]));
    output.push_str(&format!("  \"layer_count\": {},\n", map["layer_count"]));
    output.push_str(&format!("  \"layer_size\": {},\n", map["layer_size"]));
    output.push_str(&format!("  \"layer_files\": {},\n", serde_json::to_string(&map["layer_files"]).unwrap()));
    output.push_str("  \"symbols\": {\n");

    let symbols = map["symbols"].as_object().unwrap();
    let mut entries: Vec<_> = symbols.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (i, (k, v)) in entries.iter().enumerate() {
        let comma = if i < entries.len() - 1 { "," } else { "" };
        output.push_str(&format!("    {}: {}{}\n",
            serde_json::to_string(k).unwrap(),
            serde_json::to_string(v).unwrap(),
            comma
        ));
    }
    output.push_str("  }\n");
    output.push_str("}\n");

    if let Err(e) = std::fs::write(&json_path, output) {
        eprintln!("Error saving symbol map: {}", e);
        return;
    }

    // Generate .pix files for custom sprites
    if !custom_sprite_sets.is_empty() {
        println!("\nGenerating .pix files for custom sprites...");
        for sprite_set in &custom_sprite_sets {
            let pix_path = output_path.join(format!("{}.pix", sprite_set.name));
            match generate_pix_file(&pix_path, sprite_set) {
                Ok(_) => println!("  ✓ {}", pix_path.display()),
                Err(e) => eprintln!("  ✗ {}: {}", pix_path.display(), e),
            }
        }
    }

    // Generate .pix files for extracted sprites (with dedup tile_map)
    if !extracted_sprite_sets.is_empty() {
        println!("\nGenerating .pix files for extracted sprites...");
        for sprite_set in &extracted_sprite_sets {
            let pix_path = output_path.join(format!("{}.pix", sprite_set.name));
            match generate_extracted_pix_file(&pix_path, sprite_set) {
                Ok(_) => println!("  ✓ {} ({} unique / {} total)", pix_path.display(), sprite_set.unique_count, sprite_set.original_count),
                Err(e) => eprintln!("  ✗ {}: {}", pix_path.display(), e),
            }
        }
    }

    // Print summary
    println!("\n{}", "=".repeat(70));
    println!("Complete!");
    println!("{}", "=".repeat(70));
    println!("Layer size: {}x{}", lcfg.layer_size, lcfg.layer_size);
    println!("Total layers: {}", pack_result.layers.len());
    println!("Total symbols: {}", pack_result.symbol_count);
    println!("  Sprites: {}", all_sprites.len());
    if !extracted_sprite_sets.is_empty() {
        let total_unique: usize = extracted_sprite_sets.iter().map(|s| s.unique_count).sum();
        let total_orig: usize = extracted_sprite_sets.iter().map(|s| s.original_count).sum();
        println!("    (including {} extracted, {} unique / {} deduped)", extracted_sprite_sets.len(), total_unique, total_orig);
    }
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

/// Generate a .pix file for a custom sprite set
/// PIX format: width=W,height=H,texture=255
/// Each cell: sym_idx,fg_color,block_idx,bg_color
fn generate_pix_file(
    path: &Path,
    sprite_set: &sprite::CustomSpriteSet,
) -> Result<(), std::io::Error> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;

    // Header
    writeln!(
        file,
        "width={},height={},texture=255",
        sprite_set.width_tiles, sprite_set.height_tiles
    )?;

    // Each tile references a sprite by (block, sym_idx)
    // Sprites are indexed globally, we need to convert to (block, sym) format
    // Block = sprite_index / 256, sym = sprite_index % 256
    for row in 0..sprite_set.height_tiles {
        for col in 0..sprite_set.width_tiles {
            let tile_idx = (row * sprite_set.width_tiles + col) as usize;
            let sprite_idx = sprite_set.start_index + tile_idx;
            let block = sprite_idx / 256;
            let sym = sprite_idx % 256;
            // Format: sym_idx, fg_color(15=white), block_idx, bg_color(0=black)
            write!(file, "{},{},{},{} ", sym, 15, block, 0)?;
        }
        writeln!(file)?;
    }

    Ok(())
}

/// Generate a .pix file for an extracted sprite set (with dedup tile_map)
/// Uses the tile_map to reference deduplicated sprites correctly
fn generate_extracted_pix_file(
    path: &Path,
    sprite_set: &sprite::ExtractedSpriteSet,
) -> Result<(), std::io::Error> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;

    // Header
    writeln!(
        file,
        "width={},height={},texture=255",
        sprite_set.width_tiles, sprite_set.height_tiles
    )?;

    // Use tile_map for correct dedup mapping
    for row in &sprite_set.tile_map {
        for &local_idx in row {
            let sprite_idx = sprite_set.start_index + local_idx;
            let block = sprite_idx / 256;
            let sym = sprite_idx % 256;
            write!(file, "{},{},{},{} ", sym, 15, block, 0)?;
        }
        writeln!(file)?;
    }

    Ok(())
}
