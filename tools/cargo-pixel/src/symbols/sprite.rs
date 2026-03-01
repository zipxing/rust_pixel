// RustPixel
// copyright zipxing@hotmail.com 2022～2025
//
// Sprite loading from C64 character set images

use super::config::TextureConfig;
use image::{imageops, RgbaImage};
use std::path::Path;

/// Load all sprites from C64 source images
pub fn load_all_sprites(sources: &[std::path::PathBuf], cfg: &TextureConfig) -> Vec<RgbaImage> {
    let mut all_sprites = Vec::new();

    for src in sources {
        let sprites = load_c64_block(src, cfg);
        all_sprites.extend(sprites);
    }

    all_sprites
}

/// Load a single C64 source file (16×16 symbols)
/// Source files are fixed at 16×16px with 1px spacing
fn load_c64_block(source_path: &Path, cfg: &TextureConfig) -> Vec<RgbaImage> {
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
            let symbol = if cfg.sprite_char_size != SRC_CHAR_SIZE {
                imageops::resize(
                    &symbol,
                    cfg.sprite_char_size,
                    cfg.sprite_char_size,
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
