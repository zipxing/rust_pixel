use crate::c64::{C64LOW, C64UP};
use crate::types::{ConversionConfig, GlyphCandidate, PetsciiGrid};
use image::{DynamicImage, GrayImage};
use rust_pixel::render::symbols::{
    binarize_grayscale_block, calculate_mse, find_background_color, find_best_color,
    gen_charset_images, get_block_color, get_grayscale_block_at, get_petii_block_color,
    BlockGrayImage,
};

pub const GLYPH_WIDTH: u32 = 8;
pub const GLYPH_HEIGHT: u32 = 8;

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub grid: PetsciiGrid,
    /// Alternatives are row-major and always include the selected baseline first.
    pub alternatives: Vec<Vec<GlyphCandidate>>,
    /// Exact preprocessed image used by candidate generation and scoring.
    pub reference: DynamicImage,
}

/// Generate bounded deterministic preprocessing variants. The first item is always the
/// caller-provided baseline configuration.
pub fn generate_config_variants(base: &ConversionConfig) -> Vec<ConversionConfig> {
    let mut variants = vec![base.clone()];
    for contrast in [-18.0, 18.0, 36.0] {
        let mut config = base.clone();
        config.contrast = contrast;
        if !variants.contains(&config) {
            variants.push(config);
        }
    }
    variants
}

pub fn convert_image(
    image: &DynamicImage,
    config: &ConversionConfig,
) -> Result<ConversionResult, String> {
    config.validate()?;

    let adjusted = if config.contrast.abs() > f32::EPSILON {
        image.adjust_contrast(config.contrast)
    } else {
        image.clone()
    };
    let reference = adjusted.resize_exact(
        config.width * GLYPH_WIDTH,
        config.height * GLYPH_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );
    let gray = reference.clone().into_luma8();
    let charset = if config.mode == 2 {
        charset_without_alphanumeric()
    } else {
        gen_charset_images(
            false,
            GLYPH_WIDTH as usize,
            GLYPH_HEIGHT as usize,
            &C64LOW,
            &C64UP,
        )
    };

    let (background_gray, background_rgb) =
        find_background_color(&reference, &gray, reference.width(), reference.height());

    let mut cells = Vec::with_capacity((config.width * config.height) as usize);
    let mut alternatives = Vec::with_capacity(cells.capacity());

    for y in 0..config.height {
        for x in 0..config.width {
            let raw_block = get_grayscale_block_at(&gray, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
            let block = if config.mode == 1 {
                binarize_grayscale_block(
                    &raw_block,
                    background_gray,
                    GLYPH_WIDTH as usize,
                    GLYPH_HEIGHT as usize,
                )
            } else {
                raw_block
            };

            let (bg, fg) = colors_for_cell(&reference, &gray, x, y, config.mode, background_rgb);
            let ranked = rank_glyphs(&block, &charset, config.top_k, fg, bg);
            cells.push(ranked[0].cell());
            alternatives.push(ranked);
        }
    }

    Ok(ConversionResult {
        grid: PetsciiGrid::new(config.width, config.height, cells)?,
        alternatives,
        reference,
    })
}

fn colors_for_cell(
    reference: &DynamicImage,
    gray: &GrayImage,
    x: u32,
    y: u32,
    mode: u8,
    background_rgb: u32,
) -> (u8, u8) {
    if mode == 1 {
        let (bg, fg) = get_petii_block_color(
            reference,
            gray,
            x,
            y,
            background_rgb,
            GLYPH_WIDTH,
            GLYPH_HEIGHT,
        );
        (bg as u8, fg as u8)
    } else {
        let color = get_block_color(reference, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
        (0, find_best_color(color) as u8)
    }
}

fn rank_glyphs(
    input: &BlockGrayImage,
    charset: &[BlockGrayImage],
    top_k: usize,
    fg: u8,
    bg: u8,
) -> Vec<GlyphCandidate> {
    let mut ranked: Vec<_> = charset
        .iter()
        .enumerate()
        .map(|(glyph, bitmap)| GlyphCandidate {
            glyph: glyph as u8,
            distance: calculate_mse(input, bitmap, GLYPH_WIDTH as usize, GLYPH_HEIGHT as usize),
            fg,
            bg,
            texture: 1,
        })
        .collect();
    ranked.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then_with(|| a.glyph.cmp(&b.glyph))
    });
    ranked.truncate(top_k.min(ranked.len()));
    ranked
}

fn charset_without_alphanumeric() -> Vec<BlockGrayImage> {
    let mut charset = gen_charset_images(
        false,
        GLYPH_WIDTH as usize,
        GLYPH_HEIGHT as usize,
        &C64LOW,
        &C64UP,
    );
    // Match the legacy exclusion behavior for both normal and inverted glyphs.
    for glyph in 1usize..=26 {
        charset[glyph].fill(vec![0; GLYPH_WIDTH as usize]);
        charset[128 + glyph].fill(vec![0; GLYPH_WIDTH as usize]);
    }
    for glyph in 48usize..=57 {
        charset[glyph].fill(vec![0; GLYPH_WIDTH as usize]);
        charset[128 + glyph].fill(vec![0; GLYPH_WIDTH as usize]);
    }
    charset
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render_grid, PetsciiCell, PetsciiGrid};
    use image::{ImageBuffer, Rgba};

    #[test]
    fn top_k_is_sorted_and_deterministic() {
        let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255])));
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 1,
            top_k: 4,
            contrast: 0.0,
        };
        let first = convert_image(&image, &config).unwrap();
        let second = convert_image(&image, &config).unwrap();
        assert_eq!(first.grid, second.grid);
        assert_eq!(first.alternatives, second.alternatives);
        assert_eq!(first.alternatives[0].len(), 4);
        assert!(first.alternatives[0]
            .windows(2)
            .all(|w| w[0].distance <= w[1].distance));
    }

    #[test]
    fn generated_grid_is_valid_pix() {
        let image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 8, Rgba([20, 40, 80, 255])));
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 1,
            top_k: 2,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        let pix = result.grid.to_pix_string();
        assert!(pix.starts_with("width=2,height=1,texture=255\n"));
        assert_eq!(result.grid.cells.len(), 2);
    }

    #[test]
    fn exact_mode_preserves_a_known_glyph() {
        let source_grid = PetsciiGrid::new(
            1,
            1,
            vec![PetsciiCell {
                glyph: 65,
                fg: 15,
                bg: 0,
                texture: 1,
            }],
        )
        .unwrap();
        let source = DynamicImage::ImageRgba8(render_grid(&source_grid, 1).unwrap());
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 1,
            top_k: 1,
            contrast: 0.0,
        };
        let result = convert_image(&source, &config).unwrap();
        assert_eq!(result.grid.cells[0].glyph, 65);
    }
}
