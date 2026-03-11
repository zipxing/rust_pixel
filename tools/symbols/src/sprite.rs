// RustPixel
// copyright zipxing@hotmail.com 2022～2026
//
// Sprite loading from C64 character set images and custom images
// Also supports extracting sprites from arbitrary images with deduplication

use image::{imageops, GenericImageView, RgbaImage};
use rust_pixel::render::symbols::{binarize_block, BinarizationConfig, RGB};
use std::collections::HashMap;
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

// ============================================================
// Image extraction with deduplication (merged from symbol tool)
// ============================================================

/// Result of extracting sprites from an image with deduplication
pub struct ExtractedSpriteSet {
    /// Source filename (without extension)
    pub name: String,
    /// Width in tiles
    pub width_tiles: u32,
    /// Height in tiles
    pub height_tiles: u32,
    /// Starting index in the sprites array
    pub start_index: usize,
    /// Number of unique sprites (after dedup)
    pub unique_count: usize,
    /// Original tile count (before dedup)
    pub original_count: usize,
    /// Mapping from (row, col) grid position to unique sprite index (relative to start_index)
    pub tile_map: Vec<Vec<usize>>,
}

/// Calculate Hamming distance between two binary pattern vectors
fn hamming_distance(v1: &[u8], v2: &[u8]) -> u32 {
    v1.iter()
        .zip(v2.iter())
        .map(|(a, b)| if a != b { 1 } else { 0 })
        .sum()
}

/// Flatten a 2D bitmap into a 1D vector for distance calculation
fn flatten_bitmap(bitmap: &[Vec<u8>]) -> Vec<u8> {
    bitmap.iter().flat_map(|row| row.iter().copied()).collect()
}

/// Extract sprites from an image with adaptive binarization and deduplication.
///
/// Pipeline:
/// 1. Slice image into tile_size × tile_size blocks
/// 2. Binarize each block (Otsu adaptive thresholding)
/// 3. Cluster similar patterns by Hamming distance
/// 4. Keep only unique representatives, build a tile_map for .pix generation
///
/// The output sprites are RGBA images at `output_size` resolution, using the
/// foreground/background colors extracted during binarization.
pub fn load_extracted_sprites(
    image_path: &Path,
    tile_size: u32,
    dedup_threshold: u32,
    output_size: u32,
    start_index: usize,
) -> Option<(Vec<RgbaImage>, ExtractedSpriteSet)> {
    let name = image_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("extracted")
        .to_string();

    let img = match image::open(image_path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Error loading {}: {}", image_path.display(), e);
            return None;
        }
    };

    let (img_w, img_h) = img.dimensions();
    let tiles_w = img_w / tile_size;
    let tiles_h = img_h / tile_size;

    if tiles_w == 0 || tiles_h == 0 {
        eprintln!(
            "Warning: {} is too small ({}x{}) for tile size {}",
            image_path.display(), img_w, img_h, tile_size
        );
        return None;
    }

    let config = BinarizationConfig {
        min_contrast_ratio: 0.1,
    };

    // Step 1 & 2: Slice and binarize
    struct TileData {
        bitmap_flat: Vec<u8>,
        rgba: RgbaImage,
    }

    let mut all_tiles: Vec<TileData> = Vec::new();

    for ty in 0..tiles_h {
        for tx in 0..tiles_w {
            let x0 = tx * tile_size;
            let y0 = ty * tile_size;

            // Extract pixel block as RGB for binarization
            let mut pixels: Vec<Vec<RGB>> = Vec::with_capacity(tile_size as usize);
            for dy in 0..tile_size {
                let mut row = Vec::with_capacity(tile_size as usize);
                for dx in 0..tile_size {
                    let px = img.get_pixel(x0 + dx, y0 + dy);
                    row.push(RGB {
                        r: px[0],
                        g: px[1],
                        b: px[2],
                    });
                }
                pixels.push(row);
            }

            let block = binarize_block(&pixels, &config);
            let bitmap_flat = flatten_bitmap(&block.bitmap);

            // Render as RGBA using extracted fg/bg colors
            let fg = block.foreground_color;
            let bg = block.background_color;
            let mut tile_img: RgbaImage =
                image::ImageBuffer::new(tile_size, tile_size);
            for dy in 0..tile_size as usize {
                for dx in 0..tile_size as usize {
                    let is_fg = block.bitmap[dy][dx] == 1;
                    let (r, g, b) = if is_fg {
                        (fg.r, fg.g, fg.b)
                    } else {
                        (bg.r, bg.g, bg.b)
                    };
                    tile_img.put_pixel(dx as u32, dy as u32, image::Rgba([r, g, b, 255]));
                }
            }

            // Resize to output_size
            let tile_img = if output_size != tile_size {
                imageops::resize(
                    &tile_img,
                    output_size,
                    output_size,
                    imageops::FilterType::Lanczos3,
                )
            } else {
                tile_img
            };

            all_tiles.push(TileData {
                bitmap_flat,
                rgba: tile_img,
            });
        }
    }

    let original_count = all_tiles.len();

    // Step 3: Deduplicate by Hamming distance clustering
    // Map each tile to its cluster representative index
    let mut cluster_rep: Vec<usize> = Vec::with_capacity(all_tiles.len()); // tile_idx -> representative tile_idx
    let mut representatives: Vec<usize> = Vec::new(); // indices into all_tiles

    for i in 0..all_tiles.len() {
        let mut found = false;
        for &rep_idx in &representatives {
            let dist = hamming_distance(&all_tiles[i].bitmap_flat, &all_tiles[rep_idx].bitmap_flat);
            if dist <= dedup_threshold {
                cluster_rep.push(rep_idx);
                found = true;
                break;
            }
        }
        if !found {
            representatives.push(i);
            cluster_rep.push(i);
        }
    }

    // Build unique sprite list and mapping from representative tile_idx -> output index
    let mut rep_to_output: HashMap<usize, usize> = HashMap::new();
    let mut unique_sprites: Vec<RgbaImage> = Vec::new();

    for &rep_idx in &representatives {
        let output_idx = unique_sprites.len();
        rep_to_output.insert(rep_idx, output_idx);
        unique_sprites.push(all_tiles[rep_idx].rgba.clone());
    }

    // Build tile_map: grid position -> output sprite index (relative, 0-based within this set)
    let mut tile_map: Vec<Vec<usize>> = Vec::with_capacity(tiles_h as usize);
    let mut tile_idx = 0;
    for _ty in 0..tiles_h {
        let mut row = Vec::with_capacity(tiles_w as usize);
        for _tx in 0..tiles_w {
            let rep_idx = cluster_rep[tile_idx];
            let output_idx = rep_to_output[&rep_idx];
            row.push(output_idx);
            tile_idx += 1;
        }
        tile_map.push(row);
    }

    let unique_count = unique_sprites.len();

    println!(
        "  ✓ {} ({}x{} -> {}x{} tiles, {} unique / {} total, {:.1}% dedup)",
        name,
        img_w,
        img_h,
        tiles_w,
        tiles_h,
        unique_count,
        original_count,
        (1.0 - unique_count as f32 / original_count as f32) * 100.0
    );

    Some((
        unique_sprites,
        ExtractedSpriteSet {
            name,
            width_tiles: tiles_w,
            height_tiles: tiles_h,
            start_index,
            unique_count,
            original_count,
            tile_map,
        },
    ))
}
