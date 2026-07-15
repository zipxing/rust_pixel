use crate::preview::{render_grid, GLYPH_SIZE};
use crate::types::{GlyphCandidate, PetsciiGrid};
use image::{DynamicImage, GenericImageView, Pixel, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
