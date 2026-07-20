use crate::preview::{render_grid, GLYPH_SIZE};
use crate::types::{GlyphCandidate, PetsciiGrid};
use image::{DynamicImage, GenericImageView, Pixel, RgbaImage};
use lab::Lab;
use rust_pixel::render::symbols::{color_distance_rgb, RGB};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Weight on the global chroma-gap term of the perceptual objective. Per-block averaging washes out
/// chroma in both images symmetrically, so a block-level color metric cannot see the desaturation the
/// eye perceives and the mean-tone objective regresses to grey on muted palettes. This term instead
/// compares *global* mean LAB chroma (colorfulness averaged over every pixel), which directly tracks
/// "is the whole image as colorful as the reference". It is two-sided — a too-grey render and a
/// too-vivid render are both penalized for the same absolute gap — so it pulls selection toward the
/// reference's overall saturation without rewarding oversaturation.
const CHROMA_GAP_WEIGHT: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OptimizationWeights {
    pub reconstruction: f64,
    pub edges: f64,
    pub boundary_continuity: f64,
    pub palette: f64,
    pub density: f64,
}

impl Default for OptimizationWeights {
    fn default() -> Self {
        Self {
            reconstruction: 1.0,
            edges: 0.35,
            boundary_continuity: 0.2,
            palette: 0.03,
            density: 0.08,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub reconstruction: f64,
    pub edges: f64,
    pub boundary_continuity: f64,
    pub palette: f64,
    pub density: f64,
    /// Lower is better.
    pub total: f64,
}

/// Score the rendered grid against the exact preprocessed reference image.
pub fn score_grid(
    grid: &PetsciiGrid,
    reference: &DynamicImage,
    weights: OptimizationWeights,
) -> Result<ScoreBreakdown, String> {
    let rendered = render_grid(grid, 1)?;
    if rendered.dimensions() != reference.dimensions() {
        return Err(format!(
            "reference size {:?} does not match rendered size {:?}",
            reference.dimensions(),
            rendered.dimensions()
        ));
    }
    let target = reference.to_rgba8();
    Ok(score_images(grid, &rendered, &target, weights))
}

/// Eye-averaged tone distance between the rendered grid and the reference. Unlike per-pixel
/// reconstruction, this averages color over `block`x`block` regions before comparing in
/// CIEDE2000, so a dithered cell that resolves to the correct perceived tone scores well even
/// though no individual pixel matches. Lower is better. A `block` around half a glyph (4) keeps
/// cell-scale structure while rewarding sub-cell eye blending.
pub fn perceptual_tone_score(
    grid: &PetsciiGrid,
    reference: &DynamicImage,
    block: u32,
) -> Result<f64, String> {
    let rendered = render_grid(grid, 1)?;
    if rendered.dimensions() != reference.dimensions() {
        return Err(format!(
            "reference size {:?} does not match rendered size {:?}",
            reference.dimensions(),
            rendered.dimensions()
        ));
    }
    let target = reference.to_rgba8();
    Ok(perceptual_tone_distance(&rendered, &target, block))
}

/// Mean per-block perceptual cost between two equally sized images. Each block contributes its
/// CIEDE2000 tone distance plus a one-sided chroma-deficit penalty (see [`CHROMA_DEFICIT_WEIGHT`])
/// that discourages the metric's regression to grey on muted palettes. Lower is better.
pub fn perceptual_tone_distance(rendered: &RgbaImage, target: &RgbaImage, block: u32) -> f64 {
    let (tone, chroma_gap) = perceptual_tone_components(rendered, target, block);
    tone + CHROMA_GAP_WEIGHT * chroma_gap
}

/// The two competing objectives, split out so each can be inspected and the chroma weight calibrated
/// independently: `.0` is the mean per-block CIEDE2000 tone distance (rewards sub-cell eye blending),
/// `.1` is the absolute gap in global mean LAB chroma (rewards matching overall colorfulness).
pub fn perceptual_tone_components(
    rendered: &RgbaImage,
    target: &RgbaImage,
    block: u32,
) -> (f64, f64) {
    let block = block.max(1);
    let width = rendered.width();
    let height = rendered.height();
    let mut tone_total = 0.0f64;
    let mut samples = 0u32;
    let mut block_y = 0;
    while block_y < height {
        let mut block_x = 0;
        while block_x < width {
            let rendered_mean = block_mean_rgb(rendered, block_x, block_y, block);
            let target_mean = block_mean_rgb(target, block_x, block_y, block);
            tone_total += color_distance_rgb(&rendered_mean, &target_mean) as f64;
            samples += 1;
            block_x += block;
        }
        block_y += block;
    }
    let tone = tone_total / samples.max(1) as f64;
    let chroma_gap = (mean_chroma(rendered) - mean_chroma(target)).abs();
    (tone, chroma_gap)
}

/// Mean LAB chroma (colorfulness) over every pixel of an image. Near zero for a greyscale image,
/// larger the more saturated the image is overall.
fn mean_chroma(image: &RgbaImage) -> f64 {
    let mut total = 0.0f64;
    let mut count = 0u64;
    for pixel in image.pixels() {
        let [r, g, b, _] = pixel.0;
        total += lab_chroma(RGB { r, g, b });
        count += 1;
    }
    total / count.max(1) as f64
}

/// LAB chroma (colorfulness) of a color: the radius in the a*/b* plane, matching the color space
/// CIEDE2000 already scores in. Near zero for greys, larger for saturated colors.
fn lab_chroma(color: RGB) -> f64 {
    let lab = Lab::from_rgb(&[color.r, color.g, color.b]);
    ((lab.a as f64).powi(2) + (lab.b as f64).powi(2)).sqrt()
}

fn block_mean_rgb(image: &RgbaImage, x0: u32, y0: u32, block: u32) -> RGB {
    let mut sum = [0u32; 3];
    let mut count = 0u32;
    for y in y0..(y0 + block).min(image.height()) {
        for x in x0..(x0 + block).min(image.width()) {
            let pixel = image.get_pixel(x, y).0;
            sum[0] += pixel[0] as u32;
            sum[1] += pixel[1] as u32;
            sum[2] += pixel[2] as u32;
            count += 1;
        }
    }
    let count = count.max(1);
    RGB {
        r: (sum[0] / count) as u8,
        g: (sum[1] / count) as u8,
        b: (sum[2] / count) as u8,
    }
}

/// Bounded deterministic optimizer. It proposes one locally improved grid from the
/// top-K alternatives, then accepts it only when whole-image score does not regress.
pub fn optimize_grid(
    baseline: &PetsciiGrid,
    alternatives: &[Vec<GlyphCandidate>],
    reference: &DynamicImage,
    weights: OptimizationWeights,
) -> Result<(PetsciiGrid, ScoreBreakdown), String> {
    if alternatives.len() != baseline.cells.len() {
        return Err("alternative count does not match grid cell count".to_string());
    }
    let baseline_score = score_grid(baseline, reference, weights)?;
    let target = reference.to_rgba8();
    let mut proposed = baseline.clone();

    for y in 0..baseline.height {
        for x in 0..baseline.width {
            let index = baseline.index(x, y);
            let mut best = alternatives[index][0];
            let mut best_loss = f64::INFINITY;
            for candidate in &alternatives[index] {
                let loss = local_candidate_loss(x, y, *candidate, &target, weights);
                if loss < best_loss || (loss == best_loss && candidate.glyph < best.glyph) {
                    best = *candidate;
                    best_loss = loss;
                }
            }
            proposed.cells[index] = best.cell();
        }
    }

    let proposed_score = score_grid(&proposed, reference, weights)?;
    if proposed_score.total <= baseline_score.total {
        Ok((proposed, proposed_score))
    } else {
        Ok((baseline.clone(), baseline_score))
    }
}

fn score_images(
    grid: &PetsciiGrid,
    rendered: &RgbaImage,
    target: &RgbaImage,
    weights: OptimizationWeights,
) -> ScoreBreakdown {
    let pixel_count = (rendered.width() * rendered.height()) as f64;
    let mut reconstruction = 0.0;
    let mut edge_loss = 0.0;
    let mut boundary_loss = 0.0;
    let mut rendered_bright = 0usize;
    let mut target_bright = 0usize;

    for y in 0..rendered.height() {
        for x in 0..rendered.width() {
            let actual = rendered.get_pixel(x, y).to_luma()[0] as f64 / 255.0;
            let expected = target.get_pixel(x, y).to_luma()[0] as f64 / 255.0;
            reconstruction += (actual - expected).powi(2);
            rendered_bright += usize::from(actual >= 0.5);
            target_bright += usize::from(expected >= 0.5);

            if x > 0 {
                let a_edge =
                    (actual - rendered.get_pixel(x - 1, y).to_luma()[0] as f64 / 255.0).abs();
                let t_edge =
                    (expected - target.get_pixel(x - 1, y).to_luma()[0] as f64 / 255.0).abs();
                edge_loss += (a_edge - t_edge).abs();
                if x % GLYPH_SIZE == 0 {
                    boundary_loss += (a_edge - t_edge).abs();
                }
            }
            if y > 0 {
                let a_edge =
                    (actual - rendered.get_pixel(x, y - 1).to_luma()[0] as f64 / 255.0).abs();
                let t_edge =
                    (expected - target.get_pixel(x, y - 1).to_luma()[0] as f64 / 255.0).abs();
                edge_loss += (a_edge - t_edge).abs();
                if y % GLYPH_SIZE == 0 {
                    boundary_loss += (a_edge - t_edge).abs();
                }
            }
        }
    }

    reconstruction /= pixel_count;
    edge_loss /= (pixel_count * 2.0).max(1.0);
    let boundary_samples = ((grid.width.saturating_sub(1) * rendered.height())
        + (grid.height.saturating_sub(1) * rendered.width())) as f64;
    boundary_loss /= boundary_samples.max(1.0);

    let mut colors = BTreeSet::new();
    for cell in &grid.cells {
        colors.insert(cell.fg);
        colors.insert(cell.bg);
    }
    let palette_loss = colors.len().saturating_sub(16) as f64 / 240.0;
    let density_loss =
        ((rendered_bright as f64 / pixel_count) - (target_bright as f64 / pixel_count)).abs();
    let total = reconstruction * weights.reconstruction
        + edge_loss * weights.edges
        + boundary_loss * weights.boundary_continuity
        + palette_loss * weights.palette
        + density_loss * weights.density;

    ScoreBreakdown {
        reconstruction,
        edges: edge_loss,
        boundary_continuity: boundary_loss,
        palette: palette_loss,
        density: density_loss,
        total,
    }
}

fn local_candidate_loss(
    cell_x: u32,
    cell_y: u32,
    candidate: GlyphCandidate,
    target: &RgbaImage,
    weights: OptimizationWeights,
) -> f64 {
    // Render only the candidate cell. Rendering the complete 40x25 grid for
    // every alternative would turn a bounded local pass into quadratic work.
    let one = PetsciiGrid::new(1, 1, vec![candidate.cell()]).expect("valid cell grid");
    let rendered = render_grid(&one, 1).expect("validated scale");
    let x0 = cell_x * GLYPH_SIZE;
    let y0 = cell_y * GLYPH_SIZE;
    let mut reconstruction = 0.0;
    let mut edges = 0.0;
    let mut density_actual = 0usize;
    let mut density_target = 0usize;
    for y in y0..y0 + GLYPH_SIZE {
        for x in x0..x0 + GLYPH_SIZE {
            let local_x = x - x0;
            let local_y = y - y0;
            let actual = rendered.get_pixel(local_x, local_y).to_luma()[0] as f64 / 255.0;
            let expected = target.get_pixel(x, y).to_luma()[0] as f64 / 255.0;
            reconstruction += (actual - expected).powi(2);
            density_actual += usize::from(actual >= 0.5);
            density_target += usize::from(expected >= 0.5);
            if x > x0 {
                let ae = (actual
                    - rendered.get_pixel(local_x - 1, local_y).to_luma()[0] as f64 / 255.0)
                    .abs();
                let te = (expected - target.get_pixel(x - 1, y).to_luma()[0] as f64 / 255.0).abs();
                edges += (ae - te).abs();
            }
            if y > y0 {
                let ae = (actual
                    - rendered.get_pixel(local_x, local_y - 1).to_luma()[0] as f64 / 255.0)
                    .abs();
                let te = (expected - target.get_pixel(x, y - 1).to_luma()[0] as f64 / 255.0).abs();
                edges += (ae - te).abs();
            }
        }
    }
    let pixels = (GLYPH_SIZE * GLYPH_SIZE) as f64;
    let density = ((density_actual as f64 - density_target as f64) / pixels).abs();
    reconstruction / pixels * weights.reconstruction
        + edges / (pixels * 2.0) * weights.edges
        + density * weights.density
        + candidate.distance / 1_000_000.0 * 0.001
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{convert_image, ConversionConfig};
    use image::{ImageBuffer, Rgba};

    #[test]
    fn perceptual_tone_rewards_matching_average_over_pixel_exactness() {
        // A mid-gray target. A checkerboard of black and white matches its perceived tone but
        // no pixel matches; a flat mid-gray-rounded-to-black block matches half the pixels.
        let target = RgbaImage::from_pixel(4, 4, Rgba([128, 128, 128, 255]));
        let mut checker = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255]));
        for y in 0..4 {
            for x in 0..4 {
                if (x + y) % 2 == 0 {
                    checker.put_pixel(x, y, Rgba([255, 255, 255, 255]));
                }
            }
        }
        let flat_black = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255]));
        // Averaged over the whole 4x4 block, the checker's mean is exactly mid-gray.
        let checker_distance = perceptual_tone_distance(&checker, &target, 4);
        let flat_distance = perceptual_tone_distance(&flat_black, &target, 4);
        assert!(checker_distance < 1.0);
        assert!(checker_distance < flat_distance);
    }

    #[test]
    fn perceptual_tone_penalizes_desaturation_and_oversaturation() {
        // A warm, saturated target. The block-tone term alone regresses to grey on muted palettes
        // because block-averaging hides desaturation; the global chroma-gap term restores a match on
        // overall colorfulness. It is two-sided: both a greyed-out and an over-vivid render are
        // charged for departing from the target's chroma, so neither is rewarded over a faithful one.
        let target = RgbaImage::from_pixel(8, 8, Rgba([190, 110, 70, 255]));
        let faithful = RgbaImage::from_pixel(8, 8, Rgba([190, 110, 70, 255]));
        let greyed = RgbaImage::from_pixel(8, 8, Rgba([120, 120, 120, 255]));
        let oversaturated = RgbaImage::from_pixel(8, 8, Rgba([255, 40, 0, 255]));

        // The chroma-gap component is ~0 for a faithful match but clearly positive when the render
        // is too grey or too vivid.
        let (_, faithful_gap) = perceptual_tone_components(&faithful, &target, 4);
        let (_, greyed_gap) = perceptual_tone_components(&greyed, &target, 4);
        let (_, oversaturated_gap) = perceptual_tone_components(&oversaturated, &target, 4);
        assert!(faithful_gap < 1.0);
        assert!(greyed_gap > 5.0);
        assert!(oversaturated_gap > 5.0);

        // And the combined objective ranks the faithful render strictly best of the three.
        let faithful_distance = perceptual_tone_distance(&faithful, &target, 4);
        let greyed_distance = perceptual_tone_distance(&greyed, &target, 4);
        let oversaturated_distance = perceptual_tone_distance(&oversaturated, &target, 4);
        assert!(faithful_distance < greyed_distance);
        assert!(faithful_distance < oversaturated_distance);
    }

    #[test]
    fn optimizer_is_deterministic_and_monotonic() {
        let mut image = ImageBuffer::from_pixel(16, 8, Rgba([0, 0, 0, 255]));
        for y in 0..8 {
            for x in 0..8 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let image = DynamicImage::ImageRgba8(image);
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 1,
            top_k: 4,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        let weights = OptimizationWeights::default();
        let baseline = score_grid(&result.grid, &result.reference, weights).unwrap();
        let first = optimize_grid(
            &result.grid,
            &result.alternatives,
            &result.reference,
            weights,
        )
        .unwrap();
        let second = optimize_grid(
            &result.grid,
            &result.alternatives,
            &result.reference,
            weights,
        )
        .unwrap();
        assert_eq!(first, second);
        assert!(first.1.total <= baseline.total);
    }
}
