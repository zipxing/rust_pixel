// RustPixel
// copyright zipxing@hotmail.com 2022～2026
//
// Sprite loading from C64 character set images and custom images

use image::{imageops, RgbaImage};
use std::path::Path;

/// Result of loading custom sprites from a directory
pub struct CustomSpriteSet {
    /// Source filename (without extension)
    pub name: String,
    /// Width in tiles (e.g., 2 for 64px image with 32px tiles)
    pub width_tiles: u32,
    /// Height in tiles
    pub height_tiles: u32,
    /// Starting index in the sprites array
    pub start_index: usize,
    /// Number of tiles
    pub tile_count: usize,
}

/// Load all sprites from C64 source images
/// output_size: desired output size per sprite (e.g., 64 for 64×64)
pub fn load_all_sprites(sources: &[std::path::PathBuf], output_size: u32) -> Vec<RgbaImage> {
    let mut all_sprites = Vec::new();

    for src in sources {
        let sprites = load_c64_block(src, output_size);
        all_sprites.extend(sprites);
    }

    all_sprites
}

/// Load custom sprites from a directory
/// Each image is sliced into 32×32 tiles, then resized to output_size
/// Returns (sprites, sprite_sets) where sprite_sets contains metadata for pix generation
pub fn load_custom_sprites(
    custom_dir: &Path,
    output_size: u32,
    start_index: usize,
) -> (Vec<RgbaImage>, Vec<CustomSpriteSet>) {
    let mut sprites = Vec::new();
    let mut sprite_sets = Vec::new();

    // Read directory and sort files for deterministic order
    let mut files: Vec<_> = match std::fs::read_dir(custom_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext.to_ascii_lowercase() == "png")
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("Error reading custom directory {}: {}", custom_dir.display(), e);
            return (sprites, sprite_sets);
        }
    };
    files.sort_by_key(|e| e.path());

    const TILE_SIZE: u32 = 32;

    for entry in files {
        let path = entry.path();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let img = match image::open(&path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                eprintln!("Error loading {}: {}", path.display(), e);
                continue;
            }
        };

        let (w, h) = img.dimensions();
        let tiles_w = w / TILE_SIZE;
        let tiles_h = h / TILE_SIZE;

        if tiles_w == 0 || tiles_h == 0 {
            eprintln!(
                "Warning: {} is too small ({}x{}), need at least {}x{}",
                path.display(),
                w,
                h,
                TILE_SIZE,
                TILE_SIZE
            );
            continue;
        }

        let start_idx = start_index + sprites.len();
        let mut tile_count = 0;

        // Slice into tiles (row-major order)
        for ty in 0..tiles_h {
            for tx in 0..tiles_w {
                let x = tx * TILE_SIZE;
                let y = ty * TILE_SIZE;

                let tile = image::imageops::crop_imm(&img, x, y, TILE_SIZE, TILE_SIZE).to_image();

                // Resize to output_size if different
                let tile = if output_size != TILE_SIZE {
                    imageops::resize(&tile, output_size, output_size, imageops::FilterType::Lanczos3)
                } else {
                    tile
                };

                sprites.push(tile);
                tile_count += 1;
            }
        }

        println!(
            "  ✓ {} ({}x{} -> {}x{} tiles, {} sprites)",
            name, w, h, tiles_w, tiles_h, tile_count
        );

        sprite_sets.push(CustomSpriteSet {
            name,
            width_tiles: tiles_w,
            height_tiles: tiles_h,
            start_index: start_idx,
            tile_count,
        });
    }

    (sprites, sprite_sets)
}

/// Load a single C64 source file (16×16 symbols)
/// Source files are fixed at 16×16px with 1px spacing
fn load_c64_block(source_path: &Path, output_size: u32) -> Vec<RgbaImage> {
    let img = match image::open(source_path) {
        Ok(img) => img.to_rgba8(),
        Err(e) => {
            eprintln!(
                "Error loading {}: {}",
                source_path.display(),
                e
            );
            return Vec::new();
        }
    };

    let mut symbols = Vec::new();
    const SRC_CHAR_SIZE: u32 = 16;

    for row in 0..16 {
        for col in 0..16 {
            let x = col * (SRC_CHAR_SIZE + 1);
            let y = row * (SRC_CHAR_SIZE + 1);

            // Crop the symbol
            let symbol = image::imageops::crop_imm(&img, x, y, SRC_CHAR_SIZE, SRC_CHAR_SIZE)
                .to_image();

            // Resize if needed
            let symbol = if output_size != SRC_CHAR_SIZE {
                imageops::resize(
                    &symbol,
                    output_size,
                    output_size,
                    imageops::FilterType::Lanczos3,
                )
            } else {
                symbol
            };

            symbols.push(symbol);
        }
    }

    symbols
}
