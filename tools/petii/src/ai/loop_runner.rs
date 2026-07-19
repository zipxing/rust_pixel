use crate::{
    convert_image_styled, generate_config_variants, perceptual_tone_score, render_grid, score_grid,
    ConversionConfig, ConversionResult, EdgeDebugData, EdgeGrammarReport, OptimizationWeights,
    PetsciiGrid, ScoreBreakdown,
};
use image::DynamicImage;

/// Half-glyph block size for the eye-averaged tone metric that drives candidate selection.
const PERCEPTUAL_BLOCK: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct AiLoopBudget {
    /// Number of contrast variants to render before keeping the perceptual-best one.
    pub max_candidates: usize,
    pub preview_scale: u32,
}

impl Default for AiLoopBudget {
    fn default() -> Self {
        Self {
            max_candidates: 4,
            preview_scale: 2,
        }
    }
}

impl AiLoopBudget {
    fn validate(&self) -> Result<(), String> {
        if !(1..=8).contains(&self.max_candidates) {
            return Err("max_candidates must be between 1 and 8".to_string());
        }
        if !(1..=8).contains(&self.preview_scale) {
            return Err("preview_scale must be between 1 and 8".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AiLoopResult {
    pub grid: PetsciiGrid,
    pub deterministic_score: ScoreBreakdown,
    pub submitted_candidates: usize,
    pub candidates: Vec<AiLoopCandidate>,
    pub warnings: Vec<String>,
    pub edge_grammar: EdgeGrammarReport,
    pub edge_debug: Option<EdgeDebugData>,
}

#[derive(Debug, Clone)]
pub struct AiLoopCandidate {
    pub grid: PetsciiGrid,
    pub deterministic_score: ScoreBreakdown,
    pub preview: DynamicImage,
    pub selected: bool,
}

struct CandidateState {
    conversion: ConversionResult,
    grid: PetsciiGrid,
    score: ScoreBreakdown,
    /// Eye-averaged tone distance (lower is better); the loop's selection objective.
    perceptual: f64,
    preview: DynamicImage,
}

/// Deterministically convert an existing reference image to PETSCII. Several contrast variants are
/// rendered with the styled pipeline (slopes + dither) and the one with the best eye-averaged
/// perceptual tone is kept. No model is called: selection is a pure function of the pixels, so the
/// result is fully reproducible. Prompt-only reference generation remains a separate provider step.
pub fn run_with_reference(
    reference: &DynamicImage,
    base_config: &ConversionConfig,
    budget: &AiLoopBudget,
    dither: bool,
    slopes: bool,
) -> Result<AiLoopResult, String> {
    budget.validate()?;

    let weights = OptimizationWeights::default();
    let mut states = Vec::new();
    for config in generate_config_variants(base_config)
        .into_iter()
        .take(budget.max_candidates)
    {
        // The styled conversion already makes the slope/dither/contour/repaint decisions; the
        // reconstruction-based Top-K optimizer is deliberately skipped (it would undo dithering).
        // The eye-averaged perceptual score, which tracks human preference far better than per-pixel
        // reconstruction, is the sole selection objective.
        let conversion = convert_image_styled(reference, &config, dither, slopes)?;
        let grid = conversion.grid.clone();
        let score = score_grid(&grid, &conversion.reference, weights)?;
        let perceptual = perceptual_tone_score(&grid, &conversion.reference, PERCEPTUAL_BLOCK)?;
        let preview = DynamicImage::ImageRgba8(render_grid(&grid, budget.preview_scale)?);
        states.push(CandidateState {
            conversion,
            grid,
            score,
            perceptual,
            preview,
        });
    }
    states.sort_by(|left, right| left.perceptual.total_cmp(&right.perceptual));

    // States are sorted by perceptual tone, so index 0 is the perceptual-best contrast variant.
    // This is the whole of the selection: deterministic, model-free, reproducible.
    let selected = 0;
    let edge_grammar = states[selected].conversion.edge_grammar.clone();
    let edge_debug = states[selected].conversion.edge_debug.clone();
    let best_grid = states[selected].grid.clone();
    let best_score = states[selected].score;

    let submitted_candidates = states.len();
    let mut candidates: Vec<_> = states
        .into_iter()
        .map(|state| AiLoopCandidate {
            grid: state.grid,
            deterministic_score: state.score,
            preview: state.preview,
            selected: false,
        })
        .collect();
    candidates.push(AiLoopCandidate {
        grid: best_grid.clone(),
        deterministic_score: best_score,
        preview: DynamicImage::ImageRgba8(render_grid(&best_grid, budget.preview_scale)?),
        selected: true,
    });

    Ok(AiLoopResult {
        grid: best_grid,
        deterministic_score: best_score,
        submitted_candidates,
        candidates,
        warnings: Vec::new(),
        edge_grammar,
        edge_debug,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn selection_is_deterministic_and_returns_the_full_candidate_pool() {
        let reference =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 8, Rgba([40, 20, 80, 255])));
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 1,
            top_k: 4,
            contrast: 0.0,
        };
        let budget = AiLoopBudget {
            max_candidates: 2,
            preview_scale: 1,
        };
        let result = run_with_reference(&reference, &config, &budget, true, true).unwrap();
        // The candidate pool is every submitted contrast variant plus the selected copy.
        assert_eq!(result.candidates.len(), result.submitted_candidates + 1);
        assert!(result.candidates.last().unwrap().selected);
        assert!(result.warnings.is_empty());
        // Re-running yields byte-identical selection: the pipeline is a pure function of the pixels.
        let again = run_with_reference(&reference, &config, &budget, true, true).unwrap();
        assert_eq!(result.grid.cells, again.grid.cells);
    }

    #[test]
    fn single_candidate_still_produces_a_selected_grid() {
        let reference =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8, 8, Rgba([30, 50, 70, 255])));
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 1,
            top_k: 2,
            contrast: 0.0,
        };
        let result = run_with_reference(
            &reference,
            &config,
            &AiLoopBudget {
                max_candidates: 1,
                preview_scale: 1,
            },
            true,
            true,
        )
        .unwrap();
        assert_eq!(result.grid.cells.len(), 1);
    }
}
