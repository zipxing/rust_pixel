use crate::{
    ai::{schema::NormalizedRegion, Critique, MultimodalCritic, RepairDirective},
    convert_image_styled, generate_config_variants, perceptual_tone_score, render_grid, score_grid,
    ConversionConfig, ConversionResult, EdgeDebugData, EdgeGrammarReport, OptimizationWeights,
    PetsciiGrid, ScoreBreakdown,
};
use image::DynamicImage;
use rust_pixel::render::style::ANSI_COLOR_RGB;

/// Half-glyph block size for the eye-averaged tone metric that drives loop selection.
const PERCEPTUAL_BLOCK: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct AiLoopBudget {
    pub max_iterations: usize,
    pub max_candidates: usize,
    pub preview_scale: u32,
    pub allowed_colors: Vec<u8>,
}

impl Default for AiLoopBudget {
    fn default() -> Self {
        Self {
            max_iterations: 4,
            max_candidates: 4,
            preview_scale: 2,
            allowed_colors: (0..16).collect(),
        }
    }
}

impl AiLoopBudget {
    fn validate(&self) -> Result<(), String> {
        if !(1..=8).contains(&self.max_iterations) {
            return Err("max_iterations must be between 1 and 8".to_string());
        }
        if !(1..=8).contains(&self.max_candidates) {
            return Err("max_candidates must be between 1 and 8".to_string());
        }
        if !(1..=8).contains(&self.preview_scale) {
            return Err("preview_scale must be between 1 and 8".to_string());
        }
        if self.allowed_colors.is_empty() || self.allowed_colors.len() > 256 {
            return Err("allowed_colors must contain 1..=256 palette entries".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AiLoopResult {
    pub grid: PetsciiGrid,
    pub deterministic_score: ScoreBreakdown,
    pub critic: Critique,
    pub iterations: usize,
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

/// Run the bounded critique/repair loop using an existing reference image.
/// Prompt-only reference generation remains a separate provider step so this
/// function is testable and replayable without network access.
pub fn run_with_reference(
    prompt: &str,
    reference: &DynamicImage,
    base_config: &ConversionConfig,
    critic: &dyn MultimodalCritic,
    budget: &AiLoopBudget,
    dither: bool,
    slopes: bool,
) -> Result<AiLoopResult, String> {
    budget.validate()?;
    if prompt.trim().is_empty() || prompt.len() > 4096 {
        return Err("prompt must contain 1..=4096 bytes".to_string());
    }

    let weights = OptimizationWeights::default();
    let mut states = Vec::new();
    for config in generate_config_variants(base_config)
        .into_iter()
        .take(budget.max_candidates)
    {
        // The styled conversion already makes the slope/dither/contour/repaint decisions; the loop
        // no longer runs the reconstruction-based Top-K optimizer (it would undo dithering). The
        // eye-averaged perceptual score, which tracks human preference far better than per-pixel
        // reconstruction, is the selection and repair-acceptance objective.
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

    let previews: Vec<_> = states.iter().map(|state| state.preview.clone()).collect();
    let mut warnings = Vec::new();
    let initial = match critic.critique(
        prompt,
        reference,
        &previews,
        base_config.width,
        base_config.height,
        &budget.allowed_colors,
    ) {
        Ok(critique) => match validate_critic_output(
            &critique,
            base_config,
            &budget.allowed_colors,
            states.len(),
        ) {
            Ok(()) => critique,
            Err(error) => {
                warnings.push(format!("critic response rejected: {error}"));
                fallback_critique()
            }
        },
        Err(error) => {
            warnings.push(format!("critic unavailable: {error}"));
            fallback_critique()
        }
    };
    let selected = initial.selected_candidate;
    let edge_grammar = states[selected].conversion.edge_grammar.clone();
    let edge_debug = states[selected].conversion.edge_debug.clone();
    let mut best_grid = states[selected].grid.clone();
    let mut best_score = states[selected].score;
    let mut best_perceptual = states[selected].perceptual;
    let mut best_critique = initial;
    let mut iterations = 1;

    while iterations < budget.max_iterations && !best_critique.repairs.is_empty() {
        let mut repaired = best_grid.clone();
        apply_repairs(
            &mut repaired,
            &states[selected].conversion,
            &best_critique.repairs,
            &budget.allowed_colors,
        );
        let repaired_reference = &states[selected].conversion.reference;
        let repaired_score = score_grid(&repaired, repaired_reference, weights)?;
        let repaired_perceptual =
            perceptual_tone_score(&repaired, repaired_reference, PERCEPTUAL_BLOCK)?;
        let best_preview = DynamicImage::ImageRgba8(render_grid(&best_grid, budget.preview_scale)?);
        let repaired_preview =
            DynamicImage::ImageRgba8(render_grid(&repaired, budget.preview_scale)?);
        let critique = match critic.critique(
            prompt,
            reference,
            &[best_preview, repaired_preview],
            base_config.width,
            base_config.height,
            &budget.allowed_colors,
        ) {
            Ok(critique) => {
                match validate_critic_output(&critique, base_config, &budget.allowed_colors, 2) {
                    Ok(()) => critique,
                    Err(error) => {
                        warnings.push(format!("critic repair response rejected: {error}"));
                        break;
                    }
                }
            }
            Err(error) => {
                warnings.push(format!("critic repair pass unavailable: {error}"));
                break;
            }
        };
        iterations += 1;

        // Candidate zero is the previous best. A repair can only replace it when
        // the critic explicitly selects candidate one, its semantic score is not
        // lower, and the perceptual (eye-averaged tone) objective does not regress.
        if critique.selected_candidate == 1
            && critique.scores.mean() >= best_critique.scores.mean()
            && repaired_perceptual <= best_perceptual
        {
            best_grid = repaired;
            best_score = repaired_score;
            best_perceptual = repaired_perceptual;
            best_critique = critique;
        } else {
            // Preserve monotonic best-so-far, but use an empty repair list to stop
            // if the critic failed to improve the proposal.
            best_critique.repairs.clear();
        }
    }

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
        critic: best_critique,
        iterations,
        submitted_candidates,
        candidates,
        warnings,
        edge_grammar,
        edge_debug,
    })
}

fn fallback_critique() -> Critique {
    Critique {
        selected_candidate: 0,
        scores: crate::ai::CritiqueScores {
            semantic_fidelity: 0.0,
            subject_readability: 0.0,
            composition: 0.0,
            palette_coherence: 0.0,
            contour_continuity: 0.0,
            petscii_authenticity: 0.0,
        },
        regions: Vec::new(),
        repairs: Vec::new(),
        explanation: "Deterministic fallback: no valid critic response was available.".to_string(),
    }
}

fn validate_critic_output(
    critique: &Critique,
    config: &ConversionConfig,
    allowed_colors: &[u8],
    candidate_count: usize,
) -> Result<(), String> {
    critique.validate(config.width, config.height, allowed_colors)?;
    critique.validate_candidate_count(candidate_count)
}

fn apply_repairs(
    grid: &mut PetsciiGrid,
    conversion: &ConversionResult,
    repairs: &[RepairDirective],
    allowed_colors: &[u8],
) {
    for repair in repairs {
        match repair {
            RepairDirective::ReplaceCell {
                x,
                y,
                glyph,
                fg,
                bg,
            } if *x < grid.width && *y < grid.height => {
                let index = grid.index(*x, *y);
                grid.cells[index].glyph = *glyph;
                grid.cells[index].fg = *fg;
                grid.cells[index].bg = *bg;
            }
            RepairDirective::SimplifyRegion { region, strength } => {
                mutate_region(grid, conversion, *region, |alternatives| {
                    let target = (1.0 - strength) * 0.5;
                    select_density(alternatives, target)
                });
            }
            RepairDirective::ReduceDensity { region, target } => {
                mutate_region(grid, conversion, *region, |alternatives| {
                    select_density(alternatives, *target)
                });
            }
            RepairDirective::ProtectSilhouette { region } => {
                mutate_region(grid, conversion, *region, |alternatives| {
                    alternatives[0].glyph
                });
            }
            RepairDirective::IncreaseContrast { region, amount } => {
                let (dark, light) = palette_extremes(allowed_colors);
                let (fg, bg) = if *amount >= 0.0 {
                    (light, dark)
                } else {
                    (dark, light)
                };
                for_each_region_cell(grid, *region, |cell| {
                    cell.fg = fg;
                    cell.bg = bg;
                });
            }
            RepairDirective::ChangePaletteRole { role, color } => {
                let normalized = role.to_ascii_lowercase();
                for cell in &mut grid.cells {
                    if normalized.contains("background") {
                        cell.bg = *color;
                    } else if normalized.contains("foreground") || normalized.contains("subject") {
                        cell.fg = *color;
                    }
                }
            }
            // Crop changes require regenerating the reference/candidate pool and are
            // deliberately deferred rather than approximated as destructive cell shifts.
            RepairDirective::ShiftCrop { .. } | RepairDirective::ReplaceCell { .. } => {}
        }
    }
}

fn mutate_region(
    grid: &mut PetsciiGrid,
    conversion: &ConversionResult,
    region: NormalizedRegion,
    choose_glyph: impl Fn(&[crate::GlyphCandidate]) -> u8,
) {
    let (x0, y0, x1, y1) = region_bounds(grid, region);
    for y in y0..y1 {
        for x in x0..x1 {
            let index = grid.index(x, y);
            grid.cells[index].glyph = choose_glyph(&conversion.alternatives[index]);
        }
    }
}

fn for_each_region_cell(
    grid: &mut PetsciiGrid,
    region: NormalizedRegion,
    mut operation: impl FnMut(&mut crate::PetsciiCell),
) {
    let (x0, y0, x1, y1) = region_bounds(grid, region);
    for y in y0..y1 {
        for x in x0..x1 {
            let index = grid.index(x, y);
            operation(&mut grid.cells[index]);
        }
    }
}

fn region_bounds(grid: &PetsciiGrid, region: NormalizedRegion) -> (u32, u32, u32, u32) {
    let x0 = (region.x * grid.width as f32).floor() as u32;
    let y0 = (region.y * grid.height as f32).floor() as u32;
    let x1 = ((region.x + region.width) * grid.width as f32).ceil() as u32;
    let y1 = ((region.y + region.height) * grid.height as f32).ceil() as u32;
    (x0, y0, x1.min(grid.width), y1.min(grid.height))
}

fn select_density(alternatives: &[crate::GlyphCandidate], target: f32) -> u8 {
    alternatives
        .iter()
        .min_by(|left, right| {
            (glyph_density(left.glyph) - target)
                .abs()
                .total_cmp(&(glyph_density(right.glyph) - target).abs())
                .then_with(|| left.distance.total_cmp(&right.distance))
        })
        .map(|candidate| candidate.glyph)
        .unwrap_or(0)
}

fn glyph_density(glyph: u8) -> f32 {
    let bitmap = &crate::c64::C64UP[(glyph % 128) as usize];
    let set: u32 = bitmap.iter().map(|row| row.count_ones()).sum();
    let density = set as f32 / 64.0;
    if glyph >= 128 {
        1.0 - density
    } else {
        density
    }
}

fn palette_extremes(allowed_colors: &[u8]) -> (u8, u8) {
    let luminance = |index: u8| {
        let color = ANSI_COLOR_RGB[index as usize];
        0.299 * color[0] as f32 + 0.587 * color[1] as f32 + 0.114 * color[2] as f32
    };
    let dark = *allowed_colors
        .iter()
        .min_by(|left, right| luminance(**left).total_cmp(&luminance(**right)))
        .unwrap_or(&0);
    let light = *allowed_colors
        .iter()
        .max_by(|left, right| luminance(**left).total_cmp(&luminance(**right)))
        .unwrap_or(&15);
    (dark, light)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{CritiqueScores, RegionCritique};
    use image::{ImageBuffer, Rgba};
    use std::cell::Cell;

    struct MockCritic {
        calls: Cell<usize>,
    }

    struct FailingCritic;

    impl MultimodalCritic for FailingCritic {
        fn critique(
            &self,
            _prompt: &str,
            _reference: &DynamicImage,
            _candidates: &[DynamicImage],
            _grid_width: u32,
            _grid_height: u32,
            _allowed_colors: &[u8],
        ) -> Result<Critique, String> {
            Err("simulated outage".to_string())
        }
    }

    impl MultimodalCritic for MockCritic {
        fn critique(
            &self,
            _prompt: &str,
            _reference: &DynamicImage,
            candidates: &[DynamicImage],
            _grid_width: u32,
            _grid_height: u32,
            _allowed_colors: &[u8],
        ) -> Result<Critique, String> {
            let call = self.calls.get();
            self.calls.set(call + 1);
            Ok(Critique {
                selected_candidate: if call == 0 {
                    0
                } else {
                    1.min(candidates.len() - 1)
                },
                scores: CritiqueScores {
                    semantic_fidelity: 70.0 + call as f32,
                    subject_readability: 70.0 + call as f32,
                    composition: 70.0 + call as f32,
                    palette_coherence: 70.0 + call as f32,
                    contour_continuity: 70.0 + call as f32,
                    petscii_authenticity: 70.0 + call as f32,
                },
                regions: Vec::<RegionCritique>::new(),
                repairs: if call == 0 {
                    vec![RepairDirective::SimplifyRegion {
                        region: NormalizedRegion {
                            x: 0.0,
                            y: 0.0,
                            width: 1.0,
                            height: 1.0,
                        },
                        strength: 0.5,
                    }]
                } else {
                    vec![]
                },
                explanation: "mock critique".to_string(),
            })
        }
    }

    #[test]
    fn loop_is_bounded_and_accepts_explicit_improvement() {
        let reference =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 8, Rgba([40, 20, 80, 255])));
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 1,
            top_k: 4,
            contrast: 0.0,
        };
        let critic = MockCritic {
            calls: Cell::new(0),
        };
        let budget = AiLoopBudget {
            max_iterations: 3,
            max_candidates: 2,
            preview_scale: 1,
            allowed_colors: (0..16).collect(),
        };
        let result =
            run_with_reference("a purple object", &reference, &config, &critic, &budget, true, true).unwrap();
        assert_eq!(result.iterations, 2);
        assert_eq!(critic.calls.get(), 2);
        assert!(result.critic.scores.mean() >= 70.0);
    }

    #[test]
    fn provider_failure_keeps_deterministic_candidate() {
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
            "a blue tile",
            &reference,
            &config,
            &FailingCritic,
            &AiLoopBudget {
                max_iterations: 2,
                max_candidates: 1,
                preview_scale: 1,
                allowed_colors: (0..16).collect(),
            },
        true,
        true,
    )
        .unwrap();
        assert_eq!(result.iterations, 1);
        assert_eq!(result.grid.cells.len(), 1);
        assert!(result.warnings[0].contains("simulated outage"));
    }
}
