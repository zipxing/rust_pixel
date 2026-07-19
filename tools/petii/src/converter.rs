use crate::c64::{C64LOW, C64UP};
use crate::contour::ContourGraph;
use crate::corpus::CorpusPrior;
use crate::glyph_topology::{build_topology_catalog, GlyphRole, GlyphTopology, Side};
use crate::types::{ConversionConfig, GlyphCandidate, PetsciiGrid};
use image::{DynamicImage, GrayImage, Luma};
use rust_pixel::render::style::ANSI_COLOR_RGB;
use rust_pixel::render::symbols::{
    binarize_grayscale_block, calculate_mse, color_distance_rgb, find_background_color,
    find_best_color, find_best_color_u32, gen_charset_images, get_block_color,
    get_grayscale_block_at, get_petii_block_color, BlockGrayImage, RGB,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const GLYPH_WIDTH: u32 = 8;
pub const GLYPH_HEIGHT: u32 = 8;
const SPACE_GLYPH: u8 = 32;
const SOLID_GLYPH: u8 = SPACE_GLYPH + 128;
const FLAT_BACKGROUND_RANGE: u8 = 18;
const BACKGROUND_MEAN_TOLERANCE: i16 = 24;
const EDGE_CELL_MEAN_THRESHOLD: u32 = 18;
const EDGE_WEAK_THRESHOLD: u8 = 32;
const EDGE_STRONG_THRESHOLD: u8 = 96;
const EDGE_MIN_COMPONENT_PIXELS: usize = 8;
const EDGE_STRUCTURE_MIN_ACTIVE_PIXELS: usize = 12;
const EDGE_STRUCTURE_MIN_STRONG_PIXELS: usize = 4;
/// Selective-dithering thresholds. Distances are CIEDE2000 units; luma is 0-255.
/// The pipeline dithers sparingly: only cells whose single nearest palette color leaves visible
/// banding, and never dark cells, so silhouettes and shadows stay solid the way hand-drawn
/// PETSCII keeps them.
const DITHER_MIN_SOLID_ERROR: f64 = 9.0;
const DITHER_IMPROVEMENT_MARGIN: f64 = 2.5;
const DITHER_PARTNER_SHORTLIST: usize = 12;
const DITHER_DISPERSION_WEIGHT: f64 = 0.22;
const DITHER_MIN_RATIO: f64 = 0.06;
const DITHER_MAX_RATIO: f64 = 0.94;
const DITHER_QUADRANT_TOLERANCE: f64 = 0.19;
const DITHER_DARK_LUMA_FLOOR: f64 = 60.0;
const DITHER_MAX_COMPONENT: usize = 4;
/// Default weight of corpus layout cost against perceived-tone gain in the dither regularizer.
const DITHER_LAMBDA: f64 = 1.0;
/// Diagonal silhouette (slope) detection: a cell whose fill occupies a triangular region with a
/// diagonal hypotenuse renders as one of four triangle glyphs, so a slope becomes a consistent
/// run of diagonals instead of a stair-step of solid blocks. Thresholds operate on the 8x8 block.
const SLOPE_MIN_CONTRAST: u8 = 28;
const SLOPE_MIN_FILL: f64 = 0.14;
const SLOPE_MAX_FILL: f64 = 0.86;
/// Minimum fraction of the 64 pixels a fill-boundary glyph must reproduce for a silhouette cell to
/// be drawn with it rather than left to the general matcher. High enough that only clean bilevel
/// boundaries qualify; thin lines and textured cells fall through.
const SLOPE_MATCH_MIN: f64 = 0.86;
const EDGE_CONTINUITY_CANDIDATES: usize = 16;
const EDGE_APPEARANCE_CANDIDATES: usize = 6;
const EDGE_TOPOLOGY_CANDIDATES: usize = 4;
const EDGE_PARETO_CANDIDATES: usize = 4;
const EDGE_QUIET_CANDIDATES: usize = 2;
const EDGE_CHAIN_CANDIDATES: usize = 6;
const EDGE_LOOP_CANDIDATES: usize = 4;
const EDGE_JUNCTION_PASSES: usize = 2;
const EDGE_CONTINUITY_WEIGHT: f64 = 0.28;
const EDGE_PORT_CONTINUITY_WEIGHT: f64 = 0.32;
const EDGE_TARGET_TOPOLOGY_WEIGHT: f64 = 0.24;
const EDGE_TARGET_SIDE_WEIGHT: f64 = 0.22;
const EDGE_TARGET_CONNECTION_WEIGHT: f64 = 0.58;
const EDGE_BUDGET_SIDE_WEIGHT: f64 = 6.5;
const EDGE_BUDGET_BREAK_WEIGHT: f64 = 2.0;
const EDGE_PAIR_REPAIR_PASSES: usize = 2;
const EDGE_ORPHAN_WEIGHT: f64 = 0.8;
const EDGE_ORPHAN_BREAK_WEIGHT: f64 = 0.25;
const EDGE_ORPHAN_ROLLBACK_WEIGHT: f64 = 0.1;
const EDGE_ORPHAN_PASSES: usize = 1;
const EDGE_ORPHAN_MAX_REFERENCE_DELTA: f64 = 0.04;
const EDGE_SPUR_WEIGHT: f64 = 0.3;
const EDGE_NEIGHBORHOOD_SPUR_WEIGHT: f64 = 0.22;
const EDGE_CONTINUITY_PASSES: usize = 4;
const REPAINT_PALETTE_CANDIDATES: usize = 4;
const REPAINT_PASSES: usize = 2;
const REPAINT_CONTINUITY_WEIGHT: f64 = 0.32;
const REPAINT_REFERENCE_EDGE_TOLERANCE: f64 = 0.04;
const MODE2_EXCLUDED_PUNCTUATION: [u8; 3] = [33, 37, 38];

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub grid: PetsciiGrid,
    /// Alternatives are row-major and always include the selected baseline first.
    pub alternatives: Vec<Vec<GlyphCandidate>>,
    /// Exact preprocessed image used by candidate generation and scoring.
    pub reference: DynamicImage,
    pub edge_grammar: EdgeGrammarReport,
    pub edge_debug: Option<EdgeDebugData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeGateDecision {
    Disabled,
    Accepted,
    RejectedObjective,
    RejectedReferenceLoss,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EdgeGrammarMetrics {
    pub objective: f64,
    pub reference_loss: f64,
    pub target_port_loss: f64,
    pub shared_port_break_rate: f64,
    pub unexpected_endpoint_rate: f64,
    pub contour_coverage: f64,
    pub false_junction_count: usize,
    pub spur_cell_count: usize,
    pub orphan_excursion_count: usize,
    pub edited_cells: usize,
    pub edited_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeGrammarReport {
    pub decision: EdgeGateDecision,
    pub edge_cells: usize,
    pub contour_connections: usize,
    pub open_chains: usize,
    pub closed_loops: usize,
    pub junctions: usize,
    pub protected_structure_cells: usize,
    pub structure_rollbacks: usize,
    pub baseline: EdgeGrammarMetrics,
    pub proposed: EdgeGrammarMetrics,
    pub final_metrics: EdgeGrammarMetrics,
}

#[derive(Debug, Clone)]
pub struct EdgeDebugData {
    pub width: u32,
    pub height: u32,
    pub target_topologies: Vec<Option<GlyphTopology>>,
    pub baseline_topologies: Vec<GlyphTopology>,
    pub final_topologies: Vec<GlyphTopology>,
    pub edited_cells: Vec<bool>,
    pub spur_cells: Vec<bool>,
    pub orphan_cells: Vec<bool>,
    pub connections: Vec<(usize, usize, Side)>,
    pub junctions: Vec<usize>,
}

struct CellCandidates {
    ranked: Vec<GlyphCandidate>,
    edge_aware: bool,
    target_topology: Option<GlyphTopology>,
    structure_protected: bool,
    dithered: bool,
    /// Perceived-tone improvement (CIEDE2000) this cell's dither buys over a single solid color.
    /// Zero for non-dithered cells. Used by the corpus-regularized acceptance pass.
    dither_gain: f64,
}

struct CandidateGenerator<'a> {
    config: &'a ConversionConfig,
    reference: &'a DynamicImage,
    gray: &'a GrayImage,
    edge_image: &'a GrayImage,
    charset: &'a [BlockGrayImage],
    topology_catalog: &'a [GlyphTopology],
    dither_ladder: &'a DitherLadder,
    fill_boundary: &'a FillBoundaryCatalog,
    background_gray: u8,
    background_rgb: u32,
    background_color: u8,
    edge_grammar_enabled: bool,
    dither: bool,
    slopes: bool,
}

impl CandidateGenerator<'_> {
    fn generate(&self, x: u32, y: u32) -> CellCandidates {
        let raw_block = get_grayscale_block_at(self.gray, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
        let edge_block = get_grayscale_block_at(self.edge_image, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
        let flat_mean = (self.config.mode != 1)
            .then(|| uniform_block_mean(&raw_block))
            .flatten();
        // A clean diagonal silhouette boundary is drawn with a single triangle glyph, overriding
        // both the flat and thin-line contour paths so slopes read as continuous diagonals.
        if self.slopes && self.config.mode != 1 && flat_mean.is_none() {
            if let Some(candidate) = self.detect_slope_fill(x, y, &raw_block) {
                return CellCandidates {
                    ranked: vec![candidate],
                    edge_aware: false,
                    target_topology: None,
                    structure_protected: false,
                    dithered: false,
                    dither_gain: 0.0,
                };
            }
        }
        let edge_aware = self.config.mode != 1 && flat_mean.is_none() && is_edge_cell(&edge_block);
        let color_mode = if edge_aware { 1 } else { self.config.mode };
        let (bg, fg) = select_cell_colors(
            self.reference,
            self.gray,
            x,
            y,
            color_mode,
            self.background_rgb,
        );
        let edge_target = edge_aware.then(|| EdgeTarget::new(&raw_block, fg, bg));
        let structure_protected = edge_aware && is_structure_protected(&edge_block);
        let target_topology = (self.config.mode == 2)
            .then(|| edge_target.as_ref())
            .flatten()
            .map(|target| GlyphTopology::from_bitmap(&target.mask));
        // Exact extraction binarizes known PETSCII artwork. General-image
        // modes retain grayscale structure for nearest-glyph matching.
        let match_block = if self.config.mode == 1 {
            binarize_grayscale_block(
                &raw_block,
                self.background_gray,
                GLYPH_WIDTH as usize,
                GLYPH_HEIGHT as usize,
            )
        } else {
            raw_block
        };
        let mut dithered = false;
        let mut dither_gain = 0.0;
        let ranked = match flat_mean {
            Some(mean)
                if (mean as i16 - self.background_gray as i16).abs()
                    <= BACKGROUND_MEAN_TOLERANCE =>
            {
                vec![solid_candidate(
                    SPACE_GLYPH,
                    self.background_color,
                    self.background_color,
                )]
            }
            Some(_) => self
                .dither
                .then(|| self.dither_flat_cell(x, y))
                .flatten()
                .map(|(candidates, gain)| {
                    dithered = true;
                    dither_gain = gain;
                    candidates
                })
                .unwrap_or_else(|| vec![solid_candidate(SOLID_GLYPH, fg, bg)]),
            None => rank_glyphs(
                &match_block,
                self.charset,
                self.topology_catalog,
                self.config.mode,
                if edge_aware {
                    if self.edge_grammar_enabled {
                        self.config.top_k.max(EDGE_CONTINUITY_CANDIDATES)
                    } else {
                        1
                    }
                } else {
                    self.config.top_k
                },
                fg,
                bg,
                edge_target.as_ref(),
            ),
        };

        CellCandidates {
            ranked,
            edge_aware,
            target_topology,
            structure_protected,
            dithered,
            dither_gain,
        }
    }

    /// Choose a two-color dither representation for a flat cell whose average color falls
    /// between palette entries. Returns the candidate list and the perceived-tone gain over a
    /// single solid color, or `None` when a solid color is already a good match or no admissible
    /// dither glyph improves on it. The primary candidate is the dither glyph; a solid fallback is
    /// retained at index 1 for the optimizer and the corpus-regularized acceptance pass.
    fn dither_flat_cell(&self, x: u32, y: u32) -> Option<(Vec<GlyphCandidate>, f64)> {
        let average = get_block_color(self.reference, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
        // Keep dark cells solid: shadows and silhouettes read as clean shapes in hand-drawn work.
        let luma = 0.299 * average.r as f64 + 0.587 * average.g as f64 + 0.114 * average.b as f64;
        if luma < DITHER_DARK_LUMA_FLOOR {
            return None;
        }
        let shortlist = nearest_palette_shortlist(average, DITHER_PARTNER_SHORTLIST);
        let base = shortlist[0];
        let solid_error = palette_distance(average, base);
        if solid_error < DITHER_MIN_SOLID_ERROR {
            return None;
        }
        let mut best: Option<(f64, u8, u8)> = None;
        for &partner in shortlist.iter().skip(1) {
            let ratio = blend_ratio(average, base, partner);
            if !(DITHER_MIN_RATIO..=DITHER_MAX_RATIO).contains(&ratio) {
                continue;
            }
            let glyph = self.dither_ladder.pick(ratio);
            let fill = self.dither_ladder.fill[glyph as usize];
            if !(DITHER_MIN_RATIO..=DITHER_MAX_RATIO).contains(&fill) {
                continue;
            }
            let blended = blend_palette_rgb(base, partner, fill);
            let error = color_distance_rgb(&average, &blended) as f64;
            let key = (error, glyph, partner);
            if best.is_none_or(|(be, bg, bp)| key < (be, bg, bp)) {
                best = Some((error, glyph, partner));
            }
        }
        let (error, glyph, partner) = best?;
        if error + DITHER_IMPROVEMENT_MARGIN >= solid_error {
            return None;
        }
        let candidates = vec![
            GlyphCandidate {
                glyph,
                distance: error,
                fg: partner,
                bg: base,
                texture: 1,
            },
            solid_candidate(SOLID_GLYPH, base, base),
        ];
        Some((candidates, solid_error - error))
    }

    /// Draw a clean silhouette boundary with the fill-boundary glyph that best reproduces it.
    /// The block is split into a darker foreground region and a lighter background region; the
    /// catalog of horizontal, vertical, and diagonal fill glyphs is searched for the closest
    /// match. Shallow slopes resolve to a stepped run of horizontal fills, steep ones to vertical
    /// fills, and 45-degree ones to triangles, so a slope reads as a continuous edge. Returns
    /// `None` for low-contrast, thin-line, or textured cells that no clean split reproduces.
    fn detect_slope_fill(&self, x: u32, y: u32, block: &BlockGrayImage) -> Option<GlyphCandidate> {
        let mut minimum = 255u8;
        let mut maximum = 0u8;
        for row in block.iter() {
            for &pixel in row.iter() {
                minimum = minimum.min(pixel);
                maximum = maximum.max(pixel);
            }
        }
        if maximum.saturating_sub(minimum) < SLOPE_MIN_CONTRAST {
            return None;
        }
        let threshold = ((minimum as u16 + maximum as u16) / 2) as u8;
        let mut foreground_mask = [[false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        let mut total = 0u32;
        for (py, row) in block.iter().enumerate().take(GLYPH_HEIGHT as usize) {
            for (px, &pixel) in row.iter().enumerate().take(GLYPH_WIDTH as usize) {
                if pixel <= threshold {
                    foreground_mask[py][px] = true;
                    total += 1;
                }
            }
        }
        let fill = total as f64 / (GLYPH_WIDTH * GLYPH_HEIGHT) as f64;
        if !(SLOPE_MIN_FILL..=SLOPE_MAX_FILL).contains(&fill) {
            return None;
        }
        let (glyph, agreement) = self.fill_boundary.best_match(&foreground_mask)?;
        if agreement < SLOPE_MATCH_MIN {
            return None;
        }
        let (foreground, background) = self.slope_colors(x, y, block, threshold);
        Some(GlyphCandidate {
            glyph,
            distance: 0.0,
            fg: foreground,
            bg: background,
            texture: 1,
        })
    }

    /// Nearest palette colors for the darker (foreground) and lighter (background) regions of a
    /// slope cell, split at the same threshold used for detection.
    fn slope_colors(&self, x: u32, y: u32, block: &BlockGrayImage, threshold: u8) -> (u8, u8) {
        let reference = self.reference.to_rgb8();
        let mut on = [0u32; 3];
        let mut off = [0u32; 3];
        let mut on_count = 0u32;
        let mut off_count = 0u32;
        for (py, row) in block.iter().enumerate().take(GLYPH_HEIGHT as usize) {
            for (px, &pixel) in row.iter().enumerate().take(GLYPH_WIDTH as usize) {
                let rgb = reference
                    .get_pixel(x * GLYPH_WIDTH + px as u32, y * GLYPH_HEIGHT + py as u32)
                    .0;
                let target = if pixel <= threshold {
                    on_count += 1;
                    &mut on
                } else {
                    off_count += 1;
                    &mut off
                };
                for channel in 0..3 {
                    target[channel] += rgb[channel] as u32;
                }
            }
        }
        let nearest = |sum: [u32; 3], count: u32, fallback: u8| {
            if count == 0 {
                fallback
            } else {
                find_best_color(RGB {
                    r: (sum[0] / count) as u8,
                    g: (sum[1] / count) as u8,
                    b: (sum[2] / count) as u8,
                }) as u8
            }
        };
        let foreground = nearest(on, on_count, self.background_color);
        let background = nearest(off, off_count, self.background_color);
        (foreground, background)
    }
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
    convert_image_internal(image, config, true, false, None, false)
}

/// Run the full pipeline with the product enhancements toggled individually. `dither` recovers
/// intermediate tones with selective two-color dithering; `slopes` draws diagonal silhouette
/// boundaries with the fill-boundary glyph family so slopes read as continuous edges instead of
/// stair-stepped solids. Both default off in the plain [`convert_image`] entry point.
pub fn convert_image_styled(
    image: &DynamicImage,
    config: &ConversionConfig,
    dither: bool,
    slopes: bool,
) -> Result<ConversionResult, String> {
    convert_image_internal(image, config, true, dither, None, slopes)
}

/// Experimental: run the full pipeline with selective two-color dithering enabled.
/// Flat mid-tone cells that would otherwise collapse into a single solid block are
/// represented by a dither glyph mixing the two bracketing palette colors, recovering
/// the perceived intermediate tone the way hand-drawn PETSCII art does. Dithered cells
/// are exempt from the reference repaint so it cannot merge their two colors back.
pub fn convert_image_dithered(
    image: &DynamicImage,
    config: &ConversionConfig,
) -> Result<ConversionResult, String> {
    convert_image_internal(image, config, true, true, None, false)
}

/// Selective dithering regularized by a corpus prior. Each proposed dither cell is kept only
/// when its perceived-tone gain outweighs the corpus cost of turning it from a flat cell into a
/// textured one in its neighborhood, shrinking large marginal dither fields to their
/// highest-contrast core while preserving the strongest tonal recoveries.
pub fn convert_image_dithered_prior(
    image: &DynamicImage,
    config: &ConversionConfig,
    prior: &CorpusPrior,
) -> Result<ConversionResult, String> {
    convert_image_internal(image, config, true, true, Some(prior), false)
}

/// Run the local single-cell matcher without cross-cell contour candidate expansion.
/// The final reference-constrained repaint is retained so benchmark comparisons isolate
/// glyph/contour selection rather than using different color pipelines.
pub fn convert_image_top1(
    image: &DynamicImage,
    config: &ConversionConfig,
) -> Result<ConversionResult, String> {
    let mut baseline_config = config.clone();
    baseline_config.top_k = 1;
    convert_image_internal(image, &baseline_config, false, false, None, false)
}

fn convert_image_internal(
    image: &DynamicImage,
    config: &ConversionConfig,
    edge_grammar_enabled: bool,
    dither: bool,
    dither_prior: Option<&CorpusPrior>,
    slopes: bool,
) -> Result<ConversionResult, String> {
    config.validate()?;
    let mut profile_mark = Instant::now();

    // 1. Normalize the source once for every downstream scoring stage.
    let (reference, gray, edge_image) = prepare_reference(image, config);
    let charset = gen_charset_images(
        false,
        GLYPH_WIDTH as usize,
        GLYPH_HEIGHT as usize,
        &C64LOW,
        &C64UP,
    );
    let topology_catalog = build_topology_catalog(&charset);
    let dither_ladder = DitherLadder::build(&charset);
    let fill_boundary = FillBoundaryCatalog::build(&charset);
    profile_stage("reference+catalog", &mut profile_mark);

    let (background_gray, background_rgb) =
        find_background_color(&reference, &gray, reference.width(), reference.height());
    let background_color = find_best_color_u32(background_rgb) as u8;

    // 2. Generate bounded candidates independently for each character cell.
    let generator = CandidateGenerator {
        config,
        reference: &reference,
        gray: &gray,
        edge_image: &edge_image,
        charset: &charset,
        topology_catalog: &topology_catalog,
        dither_ladder: &dither_ladder,
        fill_boundary: &fill_boundary,
        background_gray,
        background_rgb,
        background_color,
        edge_grammar_enabled,
        dither,
        slopes,
    };
    let capacity = (config.width * config.height) as usize;
    let mut alternatives = Vec::with_capacity(capacity);
    let mut edge_cells = Vec::with_capacity(capacity);
    let mut target_topologies = Vec::with_capacity(capacity);
    let mut strong_structure_cells = Vec::with_capacity(capacity);
    let mut dither_cells = Vec::with_capacity(capacity);
    let mut dither_gains = Vec::with_capacity(capacity);

    for y in 0..config.height {
        for x in 0..config.width {
            let candidates = generator.generate(x, y);
            alternatives.push(candidates.ranked);
            edge_cells.push(candidates.edge_aware);
            target_topologies.push(candidates.target_topology);
            strong_structure_cells.push(candidates.structure_protected);
            dither_cells.push(candidates.dithered);
            dither_gains.push(candidates.dither_gain);
        }
    }
    profile_stage("candidate-generation", &mut profile_mark);

    // 3. Re-rank only edge cells using generic cross-cell coherence losses.
    let contour_graph = ContourGraph::from_targets(config.width, config.height, &target_topologies);
    let structure_protected = strong_structure_cells;
    let mut chain_selected = optimize_contour_chains(
        &contour_graph,
        &alternatives,
        &target_topologies,
        &topology_catalog,
    );
    coordinate_junctions(
        &contour_graph,
        &alternatives,
        &target_topologies,
        &topology_catalog,
        &mut chain_selected,
    );
    profile_stage("chain+junction", &mut profile_mark);
    let baseline = vec![0usize; alternatives.len()];
    let unconstrained_proposed = refine_edge_continuity(
        config.width,
        config.height,
        &contour_graph,
        &alternatives,
        &edge_cells,
        &target_topologies,
        &chain_selected,
        &charset,
        &topology_catalog,
    );
    profile_stage("continuity-refine", &mut profile_mark);
    let budgeted_proposed = constrain_reference_loss(
        &contour_graph,
        &alternatives,
        &edge_cells,
        &target_topologies,
        &unconstrained_proposed,
        &charset,
        &topology_catalog,
    );
    profile_stage("reference-budget", &mut profile_mark);
    let connected_proposed = repair_target_connection_pairs(
        &contour_graph,
        &alternatives,
        &edge_cells,
        &target_topologies,
        &budgeted_proposed,
        &charset,
        &topology_catalog,
    );
    profile_stage("pair-repair", &mut profile_mark);
    let cleaned_proposed = cleanup_orphan_excursions(
        config.width,
        config.height,
        &contour_graph,
        &alternatives,
        &edge_cells,
        &target_topologies,
        &connected_proposed,
        &charset,
        &topology_catalog,
    );
    profile_stage("orphan-cleanup", &mut profile_mark);
    let (proposed, structure_rollbacks) = preserve_reference_structure(
        &contour_graph,
        &alternatives,
        &structure_protected,
        &target_topologies,
        &cleaned_proposed,
        &topology_catalog,
    );
    profile_stage("structure-protection", &mut profile_mark);
    let edge_enabled = target_topologies.iter().any(Option::is_some);
    let (mut selected, edge_grammar, edge_debug) = if edge_enabled {
        let baseline_score = edge_grammar_objective(
            config.width,
            config.height,
            &contour_graph,
            &alternatives,
            &edge_cells,
            &target_topologies,
            &baseline,
            &charset,
            &topology_catalog,
        );
        let proposed_score = edge_grammar_objective(
            config.width,
            config.height,
            &contour_graph,
            &alternatives,
            &edge_cells,
            &target_topologies,
            &proposed,
            &charset,
            &topology_catalog,
        );
        let reference_limit = baseline_score.reference_loss * 1.05 + f64::EPSILON;
        let decision = edge_gate_decision(baseline_score, proposed_score, reference_limit);
        let selected = if decision == EdgeGateDecision::Accepted {
            proposed.clone()
        } else {
            baseline.clone()
        };
        let baseline_metrics = measure_edge_grammar(
            &contour_graph,
            &alternatives,
            &edge_cells,
            &target_topologies,
            &baseline,
            &charset,
            &topology_catalog,
            baseline_score,
        );
        let proposed_metrics = measure_edge_grammar(
            &contour_graph,
            &alternatives,
            &edge_cells,
            &target_topologies,
            &proposed,
            &charset,
            &topology_catalog,
            proposed_score,
        );
        let final_metrics = if decision == EdgeGateDecision::Accepted {
            proposed_metrics.clone()
        } else {
            baseline_metrics.clone()
        };
        let report = EdgeGrammarReport {
            decision,
            edge_cells: target_topologies
                .iter()
                .filter(|target| target.is_some())
                .count(),
            contour_connections: contour_graph.connections().len(),
            open_chains: contour_graph.open_chains().len(),
            closed_loops: contour_graph.closed_loops().len(),
            junctions: contour_graph.junction_cells().len(),
            protected_structure_cells: structure_protected
                .iter()
                .filter(|protected| **protected)
                .count(),
            structure_rollbacks,
            baseline: baseline_metrics,
            proposed: proposed_metrics,
            final_metrics,
        };
        let debug = EdgeDebugData {
            width: config.width,
            height: config.height,
            target_topologies: target_topologies.clone(),
            baseline_topologies: alternatives
                .iter()
                .map(|candidates| topology_catalog[candidates[0].glyph as usize])
                .collect(),
            final_topologies: selected
                .iter()
                .enumerate()
                .map(|(index, selected)| {
                    topology_catalog[alternatives[index][*selected].glyph as usize]
                })
                .collect(),
            edited_cells: selected
                .iter()
                .enumerate()
                .map(|(index, selected)| {
                    alternatives[index][*selected].glyph != alternatives[index][0].glyph
                })
                .collect(),
            spur_cells: selected
                .iter()
                .enumerate()
                .map(|(index, selected)| {
                    let candidate = alternatives[index][*selected];
                    bitmap_spur_penalty(&charset[candidate.glyph as usize]) > 0.0
                })
                .collect(),
            orphan_cells: selected
                .iter()
                .enumerate()
                .map(|(index, selected)| {
                    let candidate = alternatives[index][*selected];
                    orphan_excursion_penalty(
                        index,
                        topology_catalog[candidate.glyph as usize],
                        &contour_graph,
                        config.width,
                        config.height,
                    ) > 0.0
                })
                .collect(),
            connections: contour_graph.connections(),
            junctions: contour_graph.junction_cells(),
        };
        (selected, report, Some(debug))
    } else {
        (
            proposed,
            EdgeGrammarReport {
                decision: EdgeGateDecision::Disabled,
                edge_cells: 0,
                contour_connections: 0,
                open_chains: 0,
                closed_loops: 0,
                junctions: 0,
                protected_structure_cells: 0,
                structure_rollbacks: 0,
                baseline: EdgeGrammarMetrics::default(),
                proposed: EdgeGrammarMetrics::default(),
                final_metrics: EdgeGrammarMetrics::default(),
            },
            None,
        )
    };
    profile_stage("metrics+gate", &mut profile_mark);

    if let Some(prior) = dither_prior {
        regularize_dither(
            prior,
            config.width,
            config.height,
            &alternatives,
            &mut selected,
            &mut dither_cells,
            &dither_gains,
        );
        profile_stage("dither-regularization", &mut profile_mark);
    }

    if config.mode == 2 {
        repaint_selected_colors(
            &reference,
            config.width,
            config.height,
            &selected,
            &mut alternatives,
            &charset,
            &dither_cells,
        );
        profile_stage("reference-repaint", &mut profile_mark);
    }

    // 4. Materialize the selected grid and keep the selected candidate first.
    let mut cells = Vec::with_capacity(capacity);
    for (index, selected_index) in selected.into_iter().enumerate() {
        let selected_candidate = alternatives[index][selected_index];
        cells.push(selected_candidate.cell());
        if selected_index != 0 {
            alternatives[index].swap(0, selected_index);
        }
        alternatives[index].truncate(config.top_k);
    }
    profile_stage("materialize", &mut profile_mark);

    Ok(ConversionResult {
        grid: PetsciiGrid::new(config.width, config.height, cells)?,
        alternatives,
        reference,
        edge_grammar,
        edge_debug,
    })
}

fn profile_stage(name: &str, mark: &mut Instant) {
    if std::env::var_os("PETII_PROFILE").is_some() {
        eprintln!("petii-profile {name}: {:.3}s", mark.elapsed().as_secs_f64());
    }
    *mark = Instant::now();
}

#[derive(Debug, Clone, Default)]
struct RepaintRegionStats {
    count: u64,
    channels: [Vec<u8>; 3],
    /// Robust central color cached by [`RepaintRegionStats::finalize`]; `None` until finalized or
    /// when the region is empty.
    center: Option<RGB>,
}

impl RepaintRegionStats {
    fn add(&mut self, rgb: [u8; 3]) {
        self.count += 1;
        for channel in 0..3 {
            self.channels[channel].push(rgb[channel]);
        }
    }

    /// Compute and cache the region's representative color as the per-channel median rather than
    /// the mean. A minority of contrast pixels — the anti-aliased dark outline bleeding into a
    /// bright flat field, say — cannot drag the median toward a muddy tone the way it drags the
    /// mean. On a moon-edge cell the bright moon pixels are the majority, so the median stays
    /// bright where the mean would darken it into an olive yellow. Must be called once after all
    /// pixels are added; the per-channel scratch is released afterward.
    fn finalize(&mut self) {
        if self.count == 0 {
            self.center = None;
            return;
        }
        let median = |values: &mut Vec<u8>| -> u8 {
            values.sort_unstable();
            values[values.len() / 2]
        };
        self.center = Some(RGB {
            r: median(&mut self.channels[0]),
            g: median(&mut self.channels[1]),
            b: median(&mut self.channels[2]),
        });
        for channel in &mut self.channels {
            channel.clear();
            channel.shrink_to_fit();
        }
    }

    fn error(&self, color_index: u8) -> f64 {
        let Some(center) = self.center else {
            return 0.0;
        };
        let color = ANSI_COLOR_RGB[color_index as usize];
        let candidate = RGB {
            r: color[0],
            g: color[1],
            b: color[2],
        };
        color_distance_rgb(&center, &candidate) as f64 / 100.0
    }
}

#[derive(Debug, Clone)]
struct RepaintCellStats {
    foreground: RepaintRegionStats,
    background: RepaintRegionStats,
    foreground_candidates: Vec<u8>,
    background_candidates: Vec<u8>,
}

fn repaint_selected_colors(
    reference: &DynamicImage,
    width: u32,
    height: u32,
    selected: &[usize],
    alternatives: &mut [Vec<GlyphCandidate>],
    charset: &[BlockGrayImage],
    protected: &[bool],
) {
    let reference = reference.to_rgb8();
    let mut stats = Vec::with_capacity(selected.len());
    let mut painted = Vec::with_capacity(selected.len());

    for (index, selected_index) in selected.iter().copied().enumerate() {
        let mut candidate = alternatives[index][selected_index];
        let bitmap = &charset[candidate.glyph as usize];
        let cell_x = index as u32 % width;
        let cell_y = index as u32 / width;
        let mut foreground = RepaintRegionStats::default();
        let mut background = RepaintRegionStats::default();
        for pixel_y in 0..GLYPH_HEIGHT as usize {
            for pixel_x in 0..GLYPH_WIDTH as usize {
                let pixel = reference.get_pixel(
                    cell_x * GLYPH_WIDTH + pixel_x as u32,
                    cell_y * GLYPH_HEIGHT + pixel_y as u32,
                );
                let rgb = pixel.0;
                if bitmap[pixel_y][pixel_x] >= 128 {
                    foreground.add(rgb);
                } else {
                    background.add(rgb);
                }
            }
        }
        foreground.finalize();
        background.finalize();
        let foreground_candidates = repaint_palette_candidates(&foreground, candidate.fg);
        let background_candidates = repaint_palette_candidates(&background, candidate.bg);
        // Dithered cells intentionally hold two bracketing palette colors whose per-region
        // averages coincide; refitting from those averages would merge them back into a solid.
        let exempt = protected.get(index).copied().unwrap_or(false);
        if !exempt && foreground.count > 0 {
            candidate.fg = foreground_candidates[0];
        }
        if !exempt && background.count > 0 {
            candidate.bg = background_candidates[0];
        }
        stats.push(RepaintCellStats {
            foreground,
            background,
            foreground_candidates,
            background_candidates,
        });
        painted.push(candidate);
    }

    for pass in 0..REPAINT_PASSES {
        for step in 0..painted.len() {
            let index = if pass % 2 == 0 {
                step
            } else {
                painted.len() - 1 - step
            };
            if protected.get(index).copied().unwrap_or(false) {
                continue;
            }
            let mut foreground_candidates = stats[index].foreground_candidates.clone();
            let mut background_candidates = stats[index].background_candidates.clone();
            for neighbor in grid_neighbors(index, width, height) {
                push_unique_color(&mut foreground_candidates, painted[neighbor].fg);
                push_unique_color(&mut foreground_candidates, painted[neighbor].bg);
                push_unique_color(&mut background_candidates, painted[neighbor].fg);
                push_unique_color(&mut background_candidates, painted[neighbor].bg);
            }
            if stats[index].foreground.count == 0 {
                foreground_candidates.clear();
                foreground_candidates.push(painted[index].fg);
            }
            if stats[index].background.count == 0 {
                background_candidates.clear();
                background_candidates.push(painted[index].bg);
            }

            let mut best = painted[index];
            let mut best_score = repaint_local_score(
                index, best, width, height, &stats, &painted, &reference, charset,
            );
            for foreground in foreground_candidates.iter().copied() {
                for background in background_candidates.iter().copied() {
                    let trial = GlyphCandidate {
                        fg: foreground,
                        bg: background,
                        ..painted[index]
                    };
                    let score = repaint_local_score(
                        index, trial, width, height, &stats, &painted, &reference, charset,
                    );
                    if score < best_score
                        || (score == best_score && (foreground, background) < (best.fg, best.bg))
                    {
                        best = trial;
                        best_score = score;
                    }
                }
            }
            painted[index] = best;
        }
    }

    for (index, selected_index) in selected.iter().copied().enumerate() {
        alternatives[index][selected_index] = painted[index];
    }
}

fn repaint_palette_candidates(stats: &RepaintRegionStats, fallback: u8) -> Vec<u8> {
    if stats.count == 0 {
        return vec![fallback];
    }
    let mut ranked: Vec<_> = (0..ANSI_COLOR_RGB.len())
        .map(|color| (stats.error(color as u8), color as u8))
        .collect();
    ranked.sort_by(|first, second| {
        first
            .0
            .total_cmp(&second.0)
            .then_with(|| first.1.cmp(&second.1))
    });
    ranked
        .into_iter()
        .take(REPAINT_PALETTE_CANDIDATES)
        .map(|(_, color)| color)
        .collect()
}

fn push_unique_color(colors: &mut Vec<u8>, color: u8) {
    if !colors.contains(&color) {
        colors.push(color);
    }
}

fn grid_neighbors(index: usize, width: u32, height: u32) -> Vec<usize> {
    let x = index as u32 % width;
    let y = index as u32 / width;
    let mut neighbors = Vec::with_capacity(4);
    if y > 0 {
        neighbors.push(index - width as usize);
    }
    if x + 1 < width {
        neighbors.push(index + 1);
    }
    if y + 1 < height {
        neighbors.push(index + width as usize);
    }
    if x > 0 {
        neighbors.push(index - 1);
    }
    neighbors
}

#[allow(clippy::too_many_arguments)]
fn repaint_local_score(
    index: usize,
    candidate: GlyphCandidate,
    width: u32,
    height: u32,
    stats: &[RepaintCellStats],
    painted: &[GlyphCandidate],
    reference: &image::RgbImage,
    charset: &[BlockGrayImage],
) -> f64 {
    let reference_error = stats[index].foreground.error(candidate.fg)
        * stats[index].foreground.count as f64
        / (GLYPH_WIDTH * GLYPH_HEIGHT) as f64
        + stats[index].background.error(candidate.bg) * stats[index].background.count as f64
            / (GLYPH_WIDTH * GLYPH_HEIGHT) as f64;
    let mut continuity = 0.0;
    let mut boundary_count = 0usize;
    for neighbor in grid_neighbors(index, width, height) {
        continuity += repaint_boundary_excess(
            index,
            candidate,
            neighbor,
            painted[neighbor],
            width,
            reference,
            charset,
        );
        boundary_count += 1;
    }
    reference_error + REPAINT_CONTINUITY_WEIGHT * continuity / boundary_count.max(1) as f64
}

fn repaint_boundary_excess(
    first_index: usize,
    first: GlyphCandidate,
    second_index: usize,
    second: GlyphCandidate,
    width: u32,
    reference: &image::RgbImage,
    charset: &[BlockGrayImage],
) -> f64 {
    let first_x = first_index as u32 % width;
    let first_y = first_index as u32 / width;
    let second_x = second_index as u32 % width;
    let second_y = second_index as u32 / width;
    let (first_side, second_side) = if first_x + 1 == second_x {
        (Side::Right, Side::Left)
    } else if second_x + 1 == first_x {
        (Side::Left, Side::Right)
    } else if first_y + 1 == second_y {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    };
    let mut excess = 0.0;
    for offset in 0..GLYPH_WIDTH as usize {
        let (first_pixel_x, first_pixel_y) = side_pixel(first_side, offset);
        let (second_pixel_x, second_pixel_y) = side_pixel(second_side, offset);
        let first_output = rendered_pixel_rgb(first, first_pixel_x, first_pixel_y, charset);
        let second_output = rendered_pixel_rgb(second, second_pixel_x, second_pixel_y, charset);
        let first_reference = reference
            .get_pixel(
                first_x * GLYPH_WIDTH + first_pixel_x as u32,
                first_y * GLYPH_HEIGHT + first_pixel_y as u32,
            )
            .0;
        let second_reference = reference
            .get_pixel(
                second_x * GLYPH_WIDTH + second_pixel_x as u32,
                second_y * GLYPH_HEIGHT + second_pixel_y as u32,
            )
            .0;
        let output_delta = rgb_delta(first_output, second_output);
        let reference_delta = rgb_delta(first_reference, second_reference);
        excess += (output_delta - reference_delta - REPAINT_REFERENCE_EDGE_TOLERANCE).max(0.0);
    }
    excess / GLYPH_WIDTH as f64
}

fn side_pixel(side: Side, offset: usize) -> (usize, usize) {
    match side {
        Side::Top => (offset, 0),
        Side::Right => (GLYPH_WIDTH as usize - 1, offset),
        Side::Bottom => (offset, GLYPH_HEIGHT as usize - 1),
        Side::Left => (0, offset),
    }
}

fn rendered_pixel_rgb(
    candidate: GlyphCandidate,
    x: usize,
    y: usize,
    charset: &[BlockGrayImage],
) -> [u8; 3] {
    let color = rendered_color_index(candidate, &charset[candidate.glyph as usize], x, y);
    ANSI_COLOR_RGB[color as usize]
}

fn rgb_delta(first: [u8; 3], second: [u8; 3]) -> f64 {
    first
        .iter()
        .zip(second)
        .map(|(first, second)| (*first as f64 - second as f64).abs())
        .sum::<f64>()
        / (3.0 * 255.0)
}

fn prepare_reference(
    image: &DynamicImage,
    config: &ConversionConfig,
) -> (DynamicImage, GrayImage, GrayImage) {
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
    let edge_image = if config.mode == 1 {
        GrayImage::new(gray.width(), gray.height())
    } else {
        clean_edge_image(&sobel_image(&gray))
    };
    (reference, gray, edge_image)
}

fn select_cell_colors(
    reference: &DynamicImage,
    gray: &GrayImage,
    x: u32,
    y: u32,
    mode: u8,
    background_rgb: u32,
) -> (u8, u8) {
    if mode == 1 || mode == 2 {
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
        (
            find_best_color_u32(background_rgb) as u8,
            find_best_color(color) as u8,
        )
    }
}

fn rank_glyphs(
    input: &BlockGrayImage,
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
    mode: u8,
    top_k: usize,
    fg: u8,
    bg: u8,
    edge_target: Option<&EdgeTarget>,
) -> Vec<GlyphCandidate> {
    let mut ranked: Vec<_> = charset
        .iter()
        .enumerate()
        .filter(|(glyph, _)| glyph_allowed(mode, *glyph as u8))
        .map(|(glyph, bitmap)| {
            let distance = edge_target.map_or_else(
                || calculate_mse(input, bitmap, GLYPH_WIDTH as usize, GLYPH_HEIGHT as usize),
                |target| target.distance(bitmap),
            );
            GlyphCandidate {
                glyph: glyph as u8,
                distance,
                fg,
                bg,
                texture: 1,
            }
        })
        .collect();
    ranked.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then_with(|| a.glyph.cmp(&b.glyph))
    });
    let baseline_count = top_k.min(ranked.len());
    if baseline_count == 1 {
        ranked.truncate(1);
        return ranked;
    }
    let Some(edge_target) = edge_target else {
        ranked.truncate(baseline_count);
        return ranked;
    };

    // Keep the unchanged appearance Top-1 at index zero, then admit a small,
    // bounded set of topology-compatible glyphs before chain optimization.
    // A useful PETSCII corner or diagonal can otherwise sit outside appearance
    // Top-16 even though it is the only candidate with the required ports.
    let target_topology = GlyphTopology::from_bitmap(&edge_target.mask);
    let topology_scores: Vec<_> = topology_catalog
        .iter()
        .map(|topology| topology_candidate_cost(*topology, target_topology))
        .collect();
    let selection_scores: Vec<_> = topology_catalog
        .iter()
        .map(|topology| {
            EDGE_TARGET_TOPOLOGY_WEIGHT * topology.target_distance(target_topology)
                + EDGE_TARGET_SIDE_WEIGHT * target_side_mismatch(*topology, target_topology)
        })
        .collect();
    let baseline = ranked[0];
    let topology_order = |first: &GlyphCandidate, second: &GlyphCandidate| {
        topology_scores[first.glyph as usize]
            .total_cmp(&topology_scores[second.glyph as usize])
            .then_with(|| first.distance.total_cmp(&second.distance))
            .then_with(|| first.glyph.cmp(&second.glyph))
    };
    let mut topology_ranked = Vec::with_capacity(EDGE_TOPOLOGY_CANDIDATES);
    for candidate in ranked.iter().copied() {
        let insert_at = topology_ranked
            .binary_search_by(|existing| topology_order(existing, &candidate))
            .unwrap_or_else(|index| index);
        if insert_at < EDGE_TOPOLOGY_CANDIDATES {
            topology_ranked.insert(insert_at, candidate);
            topology_ranked.truncate(EDGE_TOPOLOGY_CANDIDATES);
        }
    }
    let baseline_topology_score = topology_scores[baseline.glyph as usize];
    let pareto_order = |first: &GlyphCandidate, second: &GlyphCandidate| {
        let score = |candidate: &GlyphCandidate| {
            let topology_improvement =
                baseline_topology_score - topology_scores[candidate.glyph as usize];
            if topology_improvement > f64::EPSILON {
                (candidate.distance - baseline.distance).max(0.0) / topology_improvement
            } else {
                f64::INFINITY
            }
        };
        score(first)
            .total_cmp(&score(second))
            .then_with(|| {
                topology_scores[first.glyph as usize]
                    .total_cmp(&topology_scores[second.glyph as usize])
            })
            .then_with(|| first.distance.total_cmp(&second.distance))
            .then_with(|| first.glyph.cmp(&second.glyph))
    };
    let mut pareto_ranked = Vec::with_capacity(EDGE_PARETO_CANDIDATES);
    for candidate in ranked.iter().copied() {
        let insert_at = pareto_ranked
            .binary_search_by(|existing| pareto_order(existing, &candidate))
            .unwrap_or_else(|index| index);
        if insert_at < EDGE_PARETO_CANDIDATES {
            pareto_ranked.insert(insert_at, candidate);
            pareto_ranked.truncate(EDGE_PARETO_CANDIDATES);
        }
    }
    let quiet_ranked: Vec<_> = ranked
        .iter()
        .copied()
        .filter(|candidate| {
            matches!(
                topology_catalog[candidate.glyph as usize].role(),
                GlyphRole::Blank | GlyphRole::Solid
            )
        })
        .take(EDGE_QUIET_CANDIDATES)
        .collect();

    let mut retained = ranked[..baseline_count.min(EDGE_APPEARANCE_CANDIDATES)].to_vec();
    for candidate in topology_ranked
        .into_iter()
        .chain(pareto_ranked)
        .chain(quiet_ranked)
    {
        if !retained
            .iter()
            .any(|existing| existing.glyph == candidate.glyph)
        {
            retained.push(candidate);
        }
    }
    let mut pool = ranked.clone();
    pool.sort_by(|first, second| {
        (first.distance + selection_scores[first.glyph as usize])
            .total_cmp(&(second.distance + selection_scores[second.glyph as usize]))
            .then_with(|| first.glyph.cmp(&second.glyph))
    });
    for candidate in pool {
        if retained.len() >= baseline_count {
            break;
        }
        if !retained
            .iter()
            .any(|existing| existing.glyph == candidate.glyph)
        {
            retained.push(candidate);
        }
    }
    retained[1..].sort_by(|first, second| {
        (first.distance + selection_scores[first.glyph as usize])
            .total_cmp(&(second.distance + selection_scores[second.glyph as usize]))
            .then_with(|| first.glyph.cmp(&second.glyph))
    });
    debug_assert_eq!(retained[0], baseline);
    debug_assert!(retained.len() <= baseline_count);
    retained
}

fn topology_candidate_cost(candidate: GlyphTopology, target: GlyphTopology) -> f64 {
    candidate.target_distance(target) + target_side_mismatch(candidate, target)
}

fn target_side_mismatch(candidate: GlyphTopology, target: GlyphTopology) -> f64 {
    Side::ALL
        .iter()
        .filter(|side| (candidate.edge_ports(**side) != 0) != (target.edge_ports(**side) != 0))
        .count() as f64
        / Side::ALL.len() as f64
}

fn glyph_allowed(mode: u8, glyph: u8) -> bool {
    if mode != 2 {
        return true;
    }
    let base = glyph % 128;
    !((1..=26).contains(&base)
        || (48..=57).contains(&base)
        || MODE2_EXCLUDED_PUNCTUATION.contains(&base))
}

fn solid_candidate(glyph: u8, fg: u8, bg: u8) -> GlyphCandidate {
    GlyphCandidate {
        glyph,
        distance: 0.0,
        fg,
        bg,
        texture: 1,
    }
}

fn uniform_block_mean(block: &BlockGrayImage) -> Option<u8> {
    let mut min = u8::MAX;
    let mut max = u8::MIN;
    let mut sum = 0u32;
    let mut count = 0u32;
    for pixel in block.iter().flatten().copied() {
        min = min.min(pixel);
        max = max.max(pixel);
        sum += pixel as u32;
        count += 1;
    }
    if count == 0 || max.saturating_sub(min) > FLAT_BACKGROUND_RANGE {
        return None;
    }
    Some((sum / count) as u8)
}

fn sobel_image(gray: &GrayImage) -> GrayImage {
    let mut edges = GrayImage::new(gray.width(), gray.height());
    if gray.width() < 3 || gray.height() < 3 {
        return edges;
    }
    for y in 1..gray.height() - 1 {
        for x in 1..gray.width() - 1 {
            let sample = |dx: i32, dy: i32| {
                gray.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                    .0[0] as i32
            };
            let gx = -sample(-1, -1) + sample(1, -1) - 2 * sample(-1, 0) + 2 * sample(1, 0)
                - sample(-1, 1)
                + sample(1, 1);
            let gy = -sample(-1, -1) - 2 * sample(0, -1) - sample(1, -1)
                + sample(-1, 1)
                + 2 * sample(0, 1)
                + sample(1, 1);
            edges.put_pixel(x, y, Luma([(gx.abs() + gy.abs()).min(255) as u8]));
        }
    }
    edges
}

/// Keep weak Sobel pixels only when they belong to a meaningful component that
/// also contains a strong edge. This is performed before splitting the image
/// into character cells, so real thin contours may continue across cell borders.
fn clean_edge_image(edges: &GrayImage) -> GrayImage {
    let width = edges.width();
    let height = edges.height();
    let mut cleaned = GrayImage::new(width, height);
    let mut visited = vec![false; (width * height) as usize];

    for start_y in 0..height {
        for start_x in 0..width {
            let start = (start_y * width + start_x) as usize;
            if visited[start] || edges.get_pixel(start_x, start_y).0[0] < EDGE_WEAK_THRESHOLD {
                continue;
            }

            let mut stack = vec![(start_x, start_y)];
            let mut component = Vec::new();
            let mut has_strong_edge = false;
            visited[start] = true;

            while let Some((x, y)) = stack.pop() {
                component.push((x, y));
                has_strong_edge |= edges.get_pixel(x, y).0[0] >= EDGE_STRONG_THRESHOLD;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let nx = nx as u32;
                        let ny = ny as u32;
                        let index = (ny * width + nx) as usize;
                        if !visited[index] && edges.get_pixel(nx, ny).0[0] >= EDGE_WEAK_THRESHOLD {
                            visited[index] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }

            if has_strong_edge && component.len() >= EDGE_MIN_COMPONENT_PIXELS {
                for (x, y) in component {
                    cleaned.put_pixel(x, y, *edges.get_pixel(x, y));
                }
            }
        }
    }
    cleaned
}

fn is_edge_cell(edge_block: &BlockGrayImage) -> bool {
    let (sum, count) = edge_block
        .iter()
        .flatten()
        .fold((0u32, 0u32), |(sum, count), pixel| {
            (sum + *pixel as u32, count + 1)
        });
    count > 0 && sum / count >= EDGE_CELL_MEAN_THRESHOLD
}

fn is_structure_protected(edge_block: &BlockGrayImage) -> bool {
    let (active, strong) =
        edge_block
            .iter()
            .flatten()
            .fold((0usize, 0usize), |(active, strong), pixel| {
                (
                    active + (*pixel >= EDGE_WEAK_THRESHOLD) as usize,
                    strong + (*pixel >= EDGE_STRONG_THRESHOLD) as usize,
                )
            });
    active >= EDGE_STRUCTURE_MIN_ACTIVE_PIXELS && strong >= EDGE_STRUCTURE_MIN_STRONG_PIXELS
}

/// Per-glyph area-fill fraction and spatial dispersion, used to pick a dither glyph whose
/// coverage approximates a target blend ratio. Higher dispersion means the foreground pixels
/// are finely interleaved (a true hatch/checker) rather than one contiguous half-block, so
/// the eye averages the two colors into an intermediate tone.
struct DitherLadder {
    fill: Vec<f64>,
    dispersion: Vec<f64>,
    admissible: Vec<bool>,
}

impl DitherLadder {
    fn build(charset: &[BlockGrayImage]) -> Self {
        let pixels = (GLYPH_WIDTH * GLYPH_HEIGHT) as f64;
        let mut fill = vec![0.0; charset.len()];
        let mut transitions = vec![0u32; charset.len()];
        // Admit a glyph as a dither only when its lit pixels are spread evenly across the cell,
        // so the eye reads it as a uniform intermediate tone. Even spread rejects letters,
        // strokes, corners, and half-blocks (all of which cluster their pixels) while admitting
        // true checkers and hatches regardless of their catalog role. Blank and solid endpoints
        // are always admissible.
        let admissible: Vec<bool> = charset.iter().map(|bitmap| is_even_stipple(bitmap)).collect();
        for (glyph, bitmap) in charset.iter().enumerate() {
            let mut on = 0u32;
            let mut edges = 0u32;
            for y in 0..GLYPH_HEIGHT as usize {
                for x in 0..GLYPH_WIDTH as usize {
                    let lit = bitmap[y][x] >= 128;
                    on += lit as u32;
                    if x + 1 < GLYPH_WIDTH as usize {
                        edges += (lit != (bitmap[y][x + 1] >= 128)) as u32;
                    }
                    if y + 1 < GLYPH_HEIGHT as usize {
                        edges += (lit != (bitmap[y + 1][x] >= 128)) as u32;
                    }
                }
            }
            fill[glyph] = on as f64 / pixels;
            transitions[glyph] = edges;
        }
        let max_transitions = transitions.iter().copied().max().unwrap_or(1).max(1) as f64;
        let dispersion: Vec<f64> = transitions
            .iter()
            .map(|count| *count as f64 / max_transitions)
            .collect();
        if std::env::var_os("PETII_DITHER_DEBUG").is_some() {
            let mut rows: Vec<_> = (0..charset.len())
                .filter(|glyph| admissible[*glyph] && fill[*glyph] > 0.0 && fill[*glyph] < 1.0)
                .collect();
            rows.sort_by(|a, b| {
                fill[*a]
                    .total_cmp(&fill[*b])
                    .then_with(|| transitions[*b].cmp(&transitions[*a]))
            });
            for glyph in rows {
                eprintln!(
                    "dither-glyph id={:3} fill={:.3} transitions={:3} dispersion={:.3}",
                    glyph, fill[glyph], transitions[glyph], dispersion[glyph]
                );
            }
        }
        Self {
            fill,
            dispersion,
            admissible,
        }
    }

    #[cfg(test)]
    fn admissible_glyphs(&self) -> Vec<u8> {
        (0..self.admissible.len())
            .filter(|glyph| self.admissible[*glyph])
            .map(|glyph| glyph as u8)
            .collect()
    }

    /// Pick the admissible glyph whose fill best matches `target`, rewarding fine dispersion so
    /// mid-tones resolve to a hatch rather than a hard half-block. Deterministic on ties. The
    /// dither vocabulary is always the graphic set (mode 2's filter), so dithering never emits
    /// letters or digits even in the letter-permitting modes 0 and 1.
    fn pick(&self, target: f64) -> u8 {
        let mut best = SPACE_GLYPH;
        let mut best_key = f64::INFINITY;
        for glyph in 0..self.fill.len() {
            let id = glyph as u8;
            if !self.admissible[glyph] || !glyph_allowed(2, id) {
                continue;
            }
            let key =
                (self.fill[glyph] - target).abs() - DITHER_DISPERSION_WEIGHT * self.dispersion[glyph];
            if key < best_key || (key == best_key && id < best) {
                best_key = key;
                best = id;
            }
        }
        best
    }
}

/// A glyph reads as a uniform tone only when its lit pixels are both spread evenly across the
/// cell and broken into small pieces. Blank and fully solid glyphs qualify trivially. Partial
/// glyphs must (a) carry close to the whole-cell fill in every 4x4 quadrant and (b) contain no
/// connected foreground blob larger than a few pixels. Together these reject letters, arrows,
/// strokes, corners, stripes, and half-blocks while admitting checkers, hatches, and stipples.
/// A curated catalog of "fill-boundary" glyphs: horizontal fills at every row split, vertical
/// fills at every column split, and the four 45-degree triangles. A silhouette cell picks the
/// member that best reproduces its foreground region, so consecutive cells step a slope smoothly.
struct FillBoundaryCatalog {
    glyphs: Vec<(u8, [[bool; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize])>,
}

impl FillBoundaryCatalog {
    fn build(charset: &[BlockGrayImage]) -> Self {
        let width = GLYPH_WIDTH as usize;
        let height = GLYPH_HEIGHT as usize;
        let glyph_mask = |glyph: usize| {
            let mut mask = [[false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
            for (y, row) in mask.iter_mut().enumerate() {
                for (x, pixel) in row.iter_mut().enumerate() {
                    *pixel = charset[glyph][y][x] >= 128;
                }
            }
            mask
        };
        let agreement = |a: &[[bool; 8]; 8], b: &[[bool; 8]; 8]| {
            (0..height)
                .map(|y| (0..width).filter(|x| a[y][*x] == b[y][*x]).count())
                .sum::<usize>()
        };
        // Ideal half-plane fills: bottom/top k rows and left/right k columns.
        let mut ideals: Vec<[[bool; 8]; 8]> = Vec::new();
        for k in 1..height {
            let mut bottom = [[false; 8]; 8];
            let mut top = [[false; 8]; 8];
            for y in 0..height {
                for x in 0..width {
                    bottom[y][x] = y >= height - k;
                    top[y][x] = y < k;
                }
            }
            ideals.push(bottom);
            ideals.push(top);
        }
        for k in 1..width {
            let mut left = [[false; 8]; 8];
            let mut right = [[false; 8]; 8];
            for y in 0..height {
                for x in 0..width {
                    left[y][x] = x < k;
                    right[y][x] = x >= width - k;
                }
            }
            ideals.push(left);
            ideals.push(right);
        }
        let mut selected: Vec<u8> = Vec::new();
        let push = |glyph: u8, selected: &mut Vec<u8>| {
            if !selected.contains(&glyph) {
                selected.push(glyph);
            }
        };
        for ideal in &ideals {
            if let Some(glyph) = (0..charset.len())
                .filter(|glyph| glyph_allowed(2, *glyph as u8))
                .max_by_key(|glyph| agreement(&glyph_mask(*glyph), ideal))
            {
                if agreement(&glyph_mask(glyph), ideal) >= 60 {
                    push(glyph as u8, &mut selected);
                }
            }
        }
        // The four 45-degree triangles, detected by their filled-corner signature.
        for glyph in 0..charset.len() {
            if glyph_allowed(2, glyph as u8) && is_triangle_glyph(&glyph_mask(glyph)) {
                push(glyph as u8, &mut selected);
            }
        }
        let glyphs = selected
            .into_iter()
            .map(|glyph| (glyph, glyph_mask(glyph as usize)))
            .collect();
        Self { glyphs }
    }

    /// Glyph whose lit pixels best agree with `foreground`, and that agreement as a 0..1 fraction.
    fn best_match(
        &self,
        foreground: &[[bool; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize],
    ) -> Option<(u8, f64)> {
        let width = GLYPH_WIDTH as usize;
        let height = GLYPH_HEIGHT as usize;
        self.glyphs
            .iter()
            .map(|(glyph, mask)| {
                let matches: usize = (0..height)
                    .map(|y| (0..width).filter(|x| mask[y][*x] == foreground[y][*x]).count())
                    .sum();
                (*glyph, matches)
            })
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(glyph, matches)| (glyph, matches as f64 / (width * height) as f64))
    }
}

/// A glyph is a 45-degree triangle when its lit pixels fill one quadrant, leave the opposite
/// quadrant empty, and half-fill the two off-diagonal quadrants.
fn is_triangle_glyph(mask: &[[bool; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize]) -> bool {
    let mut quadrant = [0u32; 4];
    for (y, row) in mask.iter().enumerate() {
        for (x, &on) in row.iter().enumerate() {
            if on {
                quadrant[usize::from(y >= 4) * 2 + usize::from(x >= 4)] += 1;
            }
        }
    }
    let fractions = quadrant.map(|count| count as f64 / 16.0);
    let full = (0..4)
        .max_by(|a, b| fractions[*a].total_cmp(&fractions[*b]))
        .unwrap();
    let empty = (0..4)
        .min_by(|a, b| fractions[*a].total_cmp(&fractions[*b]))
        .unwrap();
    const OPPOSITE: [usize; 4] = [3, 2, 1, 0];
    if OPPOSITE[full] != empty || fractions[full] - fractions[empty] < 0.7 {
        return false;
    }
    (0..4).all(|corner| {
        corner == full || corner == empty || (0.25..=0.75).contains(&fractions[corner])
    })
}

fn is_even_stipple(bitmap: &BlockGrayImage) -> bool {
    const QUADRANT: usize = 4;
    let width = GLYPH_WIDTH as usize;
    let height = GLYPH_HEIGHT as usize;
    let mut total = 0usize;
    for row in bitmap.iter().take(height) {
        for pixel in row.iter().take(width) {
            total += (*pixel >= 128) as usize;
        }
    }
    let cell_pixels = width * height;
    if total == 0 || total == cell_pixels {
        return true;
    }
    let global_fill = total as f64 / cell_pixels as f64;
    let quadrant_pixels = QUADRANT * QUADRANT;
    for quad_y in (0..height).step_by(QUADRANT) {
        for quad_x in (0..width).step_by(QUADRANT) {
            let mut lit = 0usize;
            for y in quad_y..quad_y + QUADRANT {
                for x in quad_x..quad_x + QUADRANT {
                    lit += (bitmap[y][x] >= 128) as usize;
                }
            }
            let quadrant_fill = lit as f64 / quadrant_pixels as f64;
            if (quadrant_fill - global_fill).abs() > DITHER_QUADRANT_TOLERANCE {
                return false;
            }
        }
    }
    largest_foreground_component(bitmap) <= DITHER_MAX_COMPONENT
}

/// Size in pixels of the largest 4-connected foreground blob in the cell.
fn largest_foreground_component(bitmap: &BlockGrayImage) -> usize {
    let width = GLYPH_WIDTH as usize;
    let height = GLYPH_HEIGHT as usize;
    let mut visited = vec![vec![false; width]; height];
    let mut largest = 0usize;
    let mut stack = Vec::new();
    for start_y in 0..height {
        for start_x in 0..width {
            if visited[start_y][start_x] || bitmap[start_y][start_x] < 128 {
                continue;
            }
            let mut size = 0usize;
            stack.push((start_x, start_y));
            visited[start_y][start_x] = true;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                let neighbors = [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ];
                for (nx, ny) in neighbors {
                    if nx < width && ny < height && !visited[ny][nx] && bitmap[ny][nx] >= 128 {
                        visited[ny][nx] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            largest = largest.max(size);
        }
    }
    largest
}

fn palette_rgb(index: u8) -> RGB {
    let color = ANSI_COLOR_RGB[index as usize];
    RGB {
        r: color[0],
        g: color[1],
        b: color[2],
    }
}

fn palette_distance(color: RGB, index: u8) -> f64 {
    color_distance_rgb(&color, &palette_rgb(index)) as f64
}

/// Return the `count` palette indices nearest to `color` in CIEDE2000 order, nearest first.
fn nearest_palette_shortlist(color: RGB, count: usize) -> Vec<u8> {
    let mut ranked: Vec<_> = (0..ANSI_COLOR_RGB.len() as u16)
        .map(|index| (palette_distance(color, index as u8), index as u8))
        .collect();
    ranked.sort_by(|first, second| {
        first
            .0
            .total_cmp(&second.0)
            .then_with(|| first.1.cmp(&second.1))
    });
    ranked
        .into_iter()
        .take(count.max(1))
        .map(|(_, index)| index)
        .collect()
}

/// Fraction of the way from `base` toward `partner` that best explains `color`, projecting in
/// linear RGB and clamping to `[0, 1]`.
fn blend_ratio(color: RGB, base: u8, partner: u8) -> f64 {
    let base = ANSI_COLOR_RGB[base as usize];
    let partner = ANSI_COLOR_RGB[partner as usize];
    let mut numerator = 0.0f64;
    let mut denominator = 0.0f64;
    for channel in 0..3 {
        let axis = partner[channel] as f64 - base[channel] as f64;
        let offset = [color.r, color.g, color.b][channel] as f64 - base[channel] as f64;
        numerator += offset * axis;
        denominator += axis * axis;
    }
    if denominator <= f64::EPSILON {
        0.0
    } else {
        (numerator / denominator).clamp(0.0, 1.0)
    }
}

/// Perceived color of a dither glyph that lights `fill` of its pixels with `partner` over `base`.
fn blend_palette_rgb(base: u8, partner: u8, fill: f64) -> RGB {
    let base = ANSI_COLOR_RGB[base as usize];
    let partner = ANSI_COLOR_RGB[partner as usize];
    let mix = |a: u8, b: u8| ((a as f64 * (1.0 - fill)) + (b as f64 * fill)).round().clamp(0.0, 255.0) as u8;
    RGB {
        r: mix(base[0], partner[0]),
        g: mix(base[1], partner[1]),
        b: mix(base[2], partner[2]),
    }
}

struct EdgeTarget {
    mask: BlockGrayImage,
    edges: BlockGrayImage,
}

impl EdgeTarget {
    fn new(input: &BlockGrayImage, fg: u8, bg: u8) -> Self {
        let fg_luma = palette_luma(fg);
        let bg_luma = palette_luma(bg);
        let mut mask = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        if (fg_luma - bg_luma).abs() < f32::EPSILON {
            let min = input.iter().flatten().copied().min().unwrap_or(0) as u16;
            let max = input.iter().flatten().copied().max().unwrap_or(0) as u16;
            let threshold = (min + max) / 2;
            for (source_row, target_row) in input.iter().zip(mask.iter_mut()) {
                for (source, target) in source_row.iter().zip(target_row.iter_mut()) {
                    *target = if *source as u16 > threshold { 255 } else { 0 };
                }
            }
        } else {
            for (source_row, target_row) in input.iter().zip(mask.iter_mut()) {
                for (source, target) in source_row.iter().zip(target_row.iter_mut()) {
                    let luma = *source as f32;
                    *target = if (luma - fg_luma).abs() <= (luma - bg_luma).abs() {
                        255
                    } else {
                        0
                    };
                }
            }
        }
        let edges = sobel_block(&mask);
        Self { mask, edges }
    }

    fn distance(&self, glyph: &BlockGrayImage) -> f64 {
        let glyph_edges = sobel_block(glyph);
        let mut mask_mismatch = 0u32;
        let mut edge_intersection = 0u32;
        let mut edge_union = 0u32;
        for y in 0..GLYPH_HEIGHT as usize {
            for x in 0..GLYPH_WIDTH as usize {
                if self.mask[y][x] != glyph[y][x] {
                    mask_mismatch += 1;
                }
                let target_edge = self.edges[y][x] > 0;
                let glyph_edge = glyph_edges[y][x] > 0;
                edge_intersection += (target_edge && glyph_edge) as u32;
                edge_union += (target_edge || glyph_edge) as u32;
            }
        }
        let mask_loss = mask_mismatch as f64 / (GLYPH_WIDTH * GLYPH_HEIGHT) as f64;
        let edge_loss = if edge_union == 0 {
            0.0
        } else {
            1.0 - edge_intersection as f64 / edge_union as f64
        };
        0.7 * mask_loss + 0.3 * edge_loss
    }
}

/// Revert dither cells to their solid fallback where the perceived-tone gain does not justify the
/// corpus cost of making the cell textured amid its neighbors. Reverting an edge cell lowers the
/// cost for interior cells, so the pass repeats until stable and a large marginal dither field
/// erodes down to its highest-contrast core. `lambda` (env `PETII_DITHER_LAMBDA`) trades tone
/// accuracy against corpus layout fidelity. Returns the number of reverted cells.
fn regularize_dither(
    prior: &CorpusPrior,
    width: u32,
    height: u32,
    alternatives: &[Vec<GlyphCandidate>],
    selected: &mut [usize],
    dither_cells: &mut [bool],
    dither_gains: &[f64],
) -> usize {
    let lambda = std::env::var("PETII_DITHER_LAMBDA")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(DITHER_LAMBDA);
    let width = width as usize;
    let height = height as usize;
    let visible = |selected: &[usize], index: usize| -> u8 {
        let candidate = alternatives[index][selected[index]];
        if candidate.fg == candidate.bg {
            SPACE_GLYPH
        } else {
            candidate.glyph
        }
    };
    let mut reverted = 0usize;
    loop {
        let mut changed = false;
        for index in 0..selected.len() {
            if !dither_cells[index] || selected[index] != 0 {
                continue;
            }
            let dither_token = alternatives[index][0].glyph;
            let x = index % width;
            let y = index / width;
            // Cost = extra negative-log-likelihood of the textured token over the flat token,
            // summed across the cell's present neighbors in the corpus adjacency model.
            let mut cost = 0.0;
            let mut edge = |neighbor: usize, first_is_center: bool, vertical: bool| {
                let neighbor_token = visible(selected, neighbor);
                let (dither_pair, flat_pair) = if first_is_center {
                    (
                        prior.bigram_logp(dither_token, neighbor_token, vertical),
                        prior.bigram_logp(SPACE_GLYPH, neighbor_token, vertical),
                    )
                } else {
                    (
                        prior.bigram_logp(neighbor_token, dither_token, vertical),
                        prior.bigram_logp(neighbor_token, SPACE_GLYPH, vertical),
                    )
                };
                cost += flat_pair - dither_pair;
            };
            if x + 1 < width {
                edge(index + 1, true, false);
            }
            if x > 0 {
                edge(index - 1, false, false);
            }
            if y + 1 < height {
                edge(index + width, true, true);
            }
            if y > 0 {
                edge(index - width, false, true);
            }
            if dither_gains[index] < lambda * cost {
                selected[index] = 1;
                dither_cells[index] = false;
                reverted += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    reverted
}

fn preserve_reference_structure(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    structure_protected: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    proposed: &[usize],
    topology_catalog: &[GlyphTopology],
) -> (Vec<usize>, usize) {
    let mut selected = proposed.to_vec();
    let mut rollbacks = 0usize;
    for index in 0..selected.len() {
        if !structure_protected[index] || proposed[index] == 0 {
            continue;
        }
        let Some(target) = target_topologies[index] else {
            continue;
        };
        let target_is_internal = Side::ALL.iter().all(|side| target.edge_ports(*side) == 0);
        let baseline_breaks =
            incident_break_count(index, 0, graph, alternatives, proposed, topology_catalog);
        let candidate_breaks = incident_break_count(
            index,
            proposed[index],
            graph,
            alternatives,
            proposed,
            topology_catalog,
        );
        let fixes_break = candidate_breaks < baseline_breaks;
        if target_is_internal || !fixes_break {
            selected[index] = 0;
            rollbacks += 1;
        }
    }
    (selected, rollbacks)
}

#[derive(Clone, Copy)]
enum Border {
    Top,
    Right,
    Bottom,
    Left,
}

fn optimize_contour_chains(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    target_topologies: &[Option<GlyphTopology>],
    topology_catalog: &[GlyphTopology],
) -> Vec<usize> {
    let mut selected = vec![0usize; alternatives.len()];
    for chain in graph.open_chains() {
        if chain.cells.len() < 2 {
            continue;
        }
        let mut costs: Vec<Vec<f64>> = Vec::with_capacity(chain.cells.len());
        let mut parents: Vec<Vec<usize>> = Vec::with_capacity(chain.cells.len());
        let first = chain.cells[0];
        let first_candidate_count = alternatives[first].len().min(EDGE_CHAIN_CANDIDATES);
        costs.push(
            alternatives[first]
                .iter()
                .take(first_candidate_count)
                .map(|candidate| {
                    chain_unary_cost(*candidate, target_topologies[first], topology_catalog)
                })
                .collect(),
        );
        parents.push(vec![usize::MAX; first_candidate_count]);

        for position in 1..chain.cells.len() {
            let previous_cell = chain.cells[position - 1];
            let current_cell = chain.cells[position];
            let side = graph
                .side_between(previous_cell, current_cell)
                .expect("contour chain cells must be adjacent");
            let current_candidate_count =
                alternatives[current_cell].len().min(EDGE_CHAIN_CANDIDATES);
            let previous_candidate_count =
                alternatives[previous_cell].len().min(EDGE_CHAIN_CANDIDATES);
            let mut current_costs = vec![f64::INFINITY; current_candidate_count];
            let mut current_parents = vec![0usize; current_candidate_count];
            for (current_index, current) in alternatives[current_cell]
                .iter()
                .take(current_candidate_count)
                .enumerate()
            {
                let unary =
                    chain_unary_cost(*current, target_topologies[current_cell], topology_catalog);
                for (previous_index, previous) in alternatives[previous_cell]
                    .iter()
                    .take(previous_candidate_count)
                    .enumerate()
                {
                    let score = costs[position - 1][previous_index]
                        + chain_pair_cost(*previous, *current, side, topology_catalog)
                        + unary;
                    let best_previous = current_parents[current_index];
                    if score < current_costs[current_index]
                        || (score == current_costs[current_index]
                            && (previous.glyph, previous_index)
                                < (
                                    alternatives[previous_cell][best_previous].glyph,
                                    best_previous,
                                ))
                    {
                        current_costs[current_index] = score;
                        current_parents[current_index] = previous_index;
                    }
                }
            }
            costs.push(current_costs);
            parents.push(current_parents);
        }

        let final_cell = *chain.cells.last().unwrap();
        let final_candidate_count = alternatives[final_cell].len().min(EDGE_CHAIN_CANDIDATES);
        let mut candidate_index = (0..final_candidate_count)
            .min_by(|first, second| {
                costs.last().unwrap()[*first]
                    .total_cmp(&costs.last().unwrap()[*second])
                    .then_with(|| {
                        alternatives[final_cell][*first]
                            .glyph
                            .cmp(&alternatives[final_cell][*second].glyph)
                    })
                    .then_with(|| first.cmp(second))
            })
            .unwrap_or(0);
        for position in (0..chain.cells.len()).rev() {
            selected[chain.cells[position]] = candidate_index;
            if position > 0 {
                candidate_index = parents[position][candidate_index];
            }
        }
    }
    optimize_closed_loops(
        &graph,
        alternatives,
        target_topologies,
        topology_catalog,
        &mut selected,
    );
    selected
}

fn optimize_closed_loops(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    target_topologies: &[Option<GlyphTopology>],
    topology_catalog: &[GlyphTopology],
    selected: &mut [usize],
) {
    for contour_loop in graph.closed_loops() {
        let cells = &contour_loop.cells;
        let first_cell = cells[0];
        let first_candidate_count = alternatives[first_cell].len().min(EDGE_LOOP_CANDIDATES);
        let closing_side = graph
            .side_between(*cells.last().unwrap(), first_cell)
            .expect("closed contour endpoints must be adjacent");
        let mut best_score = f64::INFINITY;
        let mut best_sequence: Option<Vec<usize>> = None;

        for first_choice in 0..first_candidate_count {
            let mut costs: Vec<Vec<f64>> = Vec::with_capacity(cells.len());
            let mut parents: Vec<Vec<usize>> = Vec::with_capacity(cells.len());
            let mut first_costs = vec![f64::INFINITY; first_candidate_count];
            first_costs[first_choice] = chain_unary_cost(
                alternatives[first_cell][first_choice],
                target_topologies[first_cell],
                topology_catalog,
            );
            costs.push(first_costs);
            parents.push(vec![usize::MAX; first_candidate_count]);

            for position in 1..cells.len() {
                let previous_cell = cells[position - 1];
                let current_cell = cells[position];
                let side = graph
                    .side_between(previous_cell, current_cell)
                    .expect("contour loop cells must be adjacent");
                let previous_count = alternatives[previous_cell].len().min(EDGE_LOOP_CANDIDATES);
                let current_count = alternatives[current_cell].len().min(EDGE_LOOP_CANDIDATES);
                let mut current_costs = vec![f64::INFINITY; current_count];
                let mut current_parents = vec![0usize; current_count];
                for (current_index, current) in alternatives[current_cell]
                    .iter()
                    .take(current_count)
                    .enumerate()
                {
                    let unary = chain_unary_cost(
                        *current,
                        target_topologies[current_cell],
                        topology_catalog,
                    );
                    for (previous_index, previous) in alternatives[previous_cell]
                        .iter()
                        .take(previous_count)
                        .enumerate()
                    {
                        let score = costs[position - 1][previous_index]
                            + chain_pair_cost(*previous, *current, side, topology_catalog)
                            + unary;
                        let best_parent = current_parents[current_index];
                        if score < current_costs[current_index]
                            || (score == current_costs[current_index]
                                && (previous.glyph, previous_index)
                                    < (alternatives[previous_cell][best_parent].glyph, best_parent))
                        {
                            current_costs[current_index] = score;
                            current_parents[current_index] = previous_index;
                        }
                    }
                }
                costs.push(current_costs);
                parents.push(current_parents);
            }

            let last_cell = *cells.last().unwrap();
            let last_count = alternatives[last_cell].len().min(EDGE_LOOP_CANDIDATES);
            for last_choice in 0..last_count {
                let score = costs.last().unwrap()[last_choice]
                    + chain_pair_cost(
                        alternatives[last_cell][last_choice],
                        alternatives[first_cell][first_choice],
                        closing_side,
                        topology_catalog,
                    );
                let mut sequence = vec![0usize; cells.len()];
                sequence[cells.len() - 1] = last_choice;
                for position in (1..cells.len()).rev() {
                    sequence[position - 1] = parents[position][sequence[position]];
                }
                let sequence_key: Vec<_> = cells
                    .iter()
                    .zip(sequence.iter())
                    .map(|(cell, candidate)| (alternatives[*cell][*candidate].glyph, *candidate))
                    .collect();
                let best_key = best_sequence.as_ref().map(|best| {
                    cells
                        .iter()
                        .zip(best.iter())
                        .map(|(cell, candidate)| {
                            (alternatives[*cell][*candidate].glyph, *candidate)
                        })
                        .collect::<Vec<_>>()
                });
                if score < best_score
                    || (score == best_score
                        && best_key
                            .as_ref()
                            .map_or(true, |best_key| sequence_key < *best_key))
                {
                    best_score = score;
                    best_sequence = Some(sequence);
                }
            }
        }

        if let Some(sequence) = best_sequence {
            for (cell, candidate) in cells.iter().zip(sequence) {
                selected[*cell] = candidate;
            }
        }
    }
}

fn coordinate_junctions(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    target_topologies: &[Option<GlyphTopology>],
    topology_catalog: &[GlyphTopology],
    selected: &mut [usize],
) {
    for pass in 0..EDGE_JUNCTION_PASSES {
        let junctions = graph.junction_cells();
        for step in 0..junctions.len() {
            let junction = if pass % 2 == 0 {
                junctions[step]
            } else {
                junctions[junctions.len() - 1 - step]
            };
            let mut best = selected[junction];
            let mut best_score = f64::INFINITY;
            for (candidate_index, candidate) in alternatives[junction]
                .iter()
                .take(EDGE_CONTINUITY_CANDIDATES)
                .enumerate()
            {
                let mut score =
                    chain_unary_cost(*candidate, target_topologies[junction], topology_catalog);
                for &neighbor in graph.neighbors(junction) {
                    let side = graph
                        .side_between(junction, neighbor)
                        .expect("junction neighbors must be adjacent");
                    score += chain_pair_cost(
                        *candidate,
                        alternatives[neighbor][selected[neighbor]],
                        side,
                        topology_catalog,
                    ) / graph.neighbors(junction).len() as f64;
                }
                if score < best_score
                    || (score == best_score
                        && (candidate.glyph, candidate_index)
                            < (alternatives[junction][best].glyph, best))
                {
                    best = candidate_index;
                    best_score = score;
                }
            }
            selected[junction] = best;
        }
    }
}

fn chain_unary_cost(
    candidate: GlyphCandidate,
    target: Option<GlyphTopology>,
    topology_catalog: &[GlyphTopology],
) -> f64 {
    let topology = topology_catalog[candidate.glyph as usize];
    candidate.distance
        + EDGE_TARGET_TOPOLOGY_WEIGHT
            * target
                .map(|target| topology.target_distance(target))
                .unwrap_or(0.0)
        + EDGE_TARGET_SIDE_WEIGHT
            * target
                .map(|target| target_side_mismatch(topology, target))
                .unwrap_or(0.0)
}

fn chain_pair_cost(
    first: GlyphCandidate,
    second: GlyphCandidate,
    side: Side,
    topology_catalog: &[GlyphTopology],
) -> f64 {
    EDGE_PORT_CONTINUITY_WEIGHT
        * topology_catalog[first.glyph as usize]
            .shared_port_mismatch_tolerant(side, topology_catalog[second.glyph as usize])
}

#[allow(clippy::too_many_arguments)]
fn constrain_reference_loss(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    proposed: &[usize],
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
) -> Vec<usize> {
    let baseline_loss: f64 = alternatives
        .iter()
        .zip(edge_cells)
        .filter(|(_, edge)| **edge)
        .map(|(candidates, _)| candidates[0].distance)
        .sum();
    let reference_limit = baseline_loss * 1.05 + f64::EPSILON;
    let spur_catalog: Vec<_> = charset.iter().map(bitmap_spur_penalty).collect();
    let mut selected = proposed.to_vec();
    let mut current_loss: f64 = alternatives
        .iter()
        .zip(edge_cells)
        .enumerate()
        .filter(|(_, (_, edge))| **edge)
        .map(|(index, (candidates, _))| candidates[selected[index]].distance)
        .sum();

    while current_loss > reference_limit {
        let mut best_rollback: Option<(f64, f64, usize, usize)> = None;
        for index in 0..alternatives.len() {
            if !edge_cells[index] || selected[index] == 0 {
                continue;
            }
            let selected_cost = local_target_structure_cost(
                index,
                selected[index],
                graph,
                alternatives,
                target_topologies,
                &selected,
                &spur_catalog,
                topology_catalog,
            );
            for candidate_index in 0..alternatives[index].len() {
                let reference_saving = alternatives[index][selected[index]].distance
                    - alternatives[index][candidate_index].distance;
                if reference_saving <= f64::EPSILON {
                    continue;
                }
                let rollback_cost = local_target_structure_cost(
                    index,
                    candidate_index,
                    graph,
                    alternatives,
                    target_topologies,
                    &selected,
                    &spur_catalog,
                    topology_catalog,
                );
                let structure_benefit = rollback_cost - selected_cost;
                let benefit_per_loss = structure_benefit / reference_saving;
                let rollback = (benefit_per_loss, structure_benefit, index, candidate_index);
                if best_rollback.as_ref().map_or(true, |best| {
                    rollback
                        .0
                        .total_cmp(&best.0)
                        .then_with(|| rollback.1.total_cmp(&best.1))
                        .then_with(|| rollback.2.cmp(&best.2))
                        .then_with(|| {
                            alternatives[index][rollback.3]
                                .glyph
                                .cmp(&alternatives[index][best.3].glyph)
                        })
                        .then_with(|| rollback.3.cmp(&best.3))
                        .is_lt()
                }) {
                    best_rollback = Some(rollback);
                }
            }
        }
        let Some((_, _, index, candidate_index)) = best_rollback else {
            break;
        };
        current_loss -= alternatives[index][selected[index]].distance
            - alternatives[index][candidate_index].distance;
        selected[index] = candidate_index;
    }
    selected
}

#[allow(clippy::too_many_arguments)]
fn local_target_structure_cost(
    index: usize,
    candidate_index: usize,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    target_topologies: &[Option<GlyphTopology>],
    selected: &[usize],
    spur_catalog: &[f64],
    topology_catalog: &[GlyphTopology],
) -> f64 {
    let candidate = alternatives[index][candidate_index];
    let topology = topology_catalog[candidate.glyph as usize];
    let target_cost = target_topologies[index].map_or(0.0, |target| {
        EDGE_TARGET_TOPOLOGY_WEIGHT * topology.target_distance(target)
            + EDGE_BUDGET_SIDE_WEIGHT * target_side_mismatch(topology, target)
    });
    let neighbors = graph.neighbors(index);
    let (connection_cost, broken_connections) =
        neighbors
            .iter()
            .fold((0.0, 0usize), |(mismatch, breaks), neighbor| {
                let side = graph
                    .side_between(index, *neighbor)
                    .expect("contour graph neighbors must be adjacent");
                let neighbor_candidate = alternatives[*neighbor][selected[*neighbor]];
                let pair_mismatch = topology.shared_port_mismatch_tolerant(
                    side,
                    topology_catalog[neighbor_candidate.glyph as usize],
                );
                (
                    mismatch + pair_mismatch,
                    breaks + (pair_mismatch > 0.0) as usize,
                )
            });
    let neighbor_denominator = neighbors.len().max(1) as f64;
    target_cost
        + EDGE_TARGET_CONNECTION_WEIGHT * connection_cost / neighbor_denominator
        + EDGE_BUDGET_BREAK_WEIGHT * broken_connections as f64 / neighbor_denominator
        + EDGE_SPUR_WEIGHT * spur_catalog[candidate.glyph as usize]
}

#[allow(clippy::too_many_arguments)]
fn repair_target_connection_pairs(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    initial_selected: &[usize],
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
) -> Vec<usize> {
    let baseline_loss: f64 = alternatives
        .iter()
        .zip(edge_cells)
        .filter(|(_, edge)| **edge)
        .map(|(candidates, _)| candidates[0].distance)
        .sum();
    let reference_limit = baseline_loss * 1.05 + f64::EPSILON;
    let spur_catalog: Vec<_> = charset.iter().map(bitmap_spur_penalty).collect();
    let mut selected = initial_selected.to_vec();
    let mut current_loss: f64 = alternatives
        .iter()
        .zip(edge_cells)
        .enumerate()
        .filter(|(_, (_, edge))| **edge)
        .map(|(index, (candidates, _))| candidates[selected[index]].distance)
        .sum();

    for pass in 0..EDGE_PAIR_REPAIR_PASSES {
        let connections = graph.connections();
        for step in 0..connections.len() {
            let (first, second, side) = if pass % 2 == 0 {
                connections[step]
            } else {
                connections[connections.len() - 1 - step]
            };
            let first_topology =
                topology_catalog[alternatives[first][selected[first]].glyph as usize];
            let second_topology =
                topology_catalog[alternatives[second][selected[second]].glyph as usize];
            if first_topology.shared_port_mismatch_tolerant(side, second_topology) == 0.0 {
                continue;
            }

            let current_cost = pair_target_structure_cost(
                first,
                selected[first],
                second,
                selected[second],
                graph,
                alternatives,
                target_topologies,
                &selected,
                &spur_catalog,
                topology_catalog,
            );
            let current_pair_reference = alternatives[first][selected[first]].distance
                + alternatives[second][selected[second]].distance;
            let current_side_error = [first, second]
                .into_iter()
                .map(|index| {
                    target_topologies[index].map_or(0.0, |target| {
                        let candidate = alternatives[index][selected[index]];
                        target_side_mismatch(topology_catalog[candidate.glyph as usize], target)
                    })
                })
                .sum::<f64>();
            let mut best = (selected[first], selected[second]);
            let mut best_cost = current_cost;
            let mut best_reference = current_pair_reference;

            for first_candidate in 0..alternatives[first].len() {
                for second_candidate in 0..alternatives[second].len() {
                    let first_candidate_topology =
                        topology_catalog[alternatives[first][first_candidate].glyph as usize];
                    let second_candidate_topology =
                        topology_catalog[alternatives[second][second_candidate].glyph as usize];
                    if first_candidate_topology
                        .shared_port_mismatch_tolerant(side, second_candidate_topology)
                        > 0.0
                    {
                        continue;
                    }
                    let candidate_side_error = [
                        (first, first_candidate, first_candidate_topology),
                        (second, second_candidate, second_candidate_topology),
                    ]
                    .into_iter()
                    .map(|(index, _, topology)| {
                        target_topologies[index]
                            .map_or(0.0, |target| target_side_mismatch(topology, target))
                    })
                    .sum::<f64>();
                    if candidate_side_error > current_side_error {
                        continue;
                    }
                    let pair_reference = alternatives[first][first_candidate].distance
                        + alternatives[second][second_candidate].distance;
                    let proposed_loss = current_loss - current_pair_reference + pair_reference;
                    if proposed_loss > reference_limit {
                        continue;
                    }
                    let score = pair_target_structure_cost(
                        first,
                        first_candidate,
                        second,
                        second_candidate,
                        graph,
                        alternatives,
                        target_topologies,
                        &selected,
                        &spur_catalog,
                        topology_catalog,
                    );
                    let key = (
                        alternatives[first][first_candidate].glyph,
                        alternatives[second][second_candidate].glyph,
                        first_candidate,
                        second_candidate,
                    );
                    let best_key = (
                        alternatives[first][best.0].glyph,
                        alternatives[second][best.1].glyph,
                        best.0,
                        best.1,
                    );
                    if score < best_cost
                        || (score == best_cost
                            && (pair_reference < best_reference
                                || (pair_reference == best_reference && key < best_key)))
                    {
                        best = (first_candidate, second_candidate);
                        best_cost = score;
                        best_reference = pair_reference;
                    }
                }
            }
            if best_cost + f64::EPSILON < current_cost {
                current_loss = current_loss - current_pair_reference + best_reference;
                selected[first] = best.0;
                selected[second] = best.1;
            }
        }
    }
    selected
}

#[allow(clippy::too_many_arguments)]
fn pair_target_structure_cost(
    first: usize,
    first_candidate: usize,
    second: usize,
    second_candidate: usize,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    target_topologies: &[Option<GlyphTopology>],
    selected: &[usize],
    spur_catalog: &[f64],
    topology_catalog: &[GlyphTopology],
) -> f64 {
    let choice = |index: usize| {
        if index == first {
            first_candidate
        } else if index == second {
            second_candidate
        } else {
            selected[index]
        }
    };
    let mut cost = 0.0;
    for index in [first, second] {
        let candidate = alternatives[index][choice(index)];
        let topology = topology_catalog[candidate.glyph as usize];
        if let Some(target) = target_topologies[index] {
            cost += EDGE_TARGET_TOPOLOGY_WEIGHT * topology.target_distance(target)
                + EDGE_BUDGET_SIDE_WEIGHT * target_side_mismatch(topology, target);
        }
        cost += EDGE_SPUR_WEIGHT * spur_catalog[candidate.glyph as usize];
    }

    let mut incident_connections = Vec::new();
    for index in [first, second] {
        for &neighbor in graph.neighbors(index) {
            let connection = if index < neighbor {
                (index, neighbor)
            } else {
                (neighbor, index)
            };
            if !incident_connections.contains(&connection) {
                incident_connections.push(connection);
            }
        }
    }
    incident_connections.sort_unstable();
    for (left, right) in incident_connections {
        let side = graph
            .side_between(left, right)
            .expect("contour graph neighbors must be adjacent");
        let left_candidate = alternatives[left][choice(left)];
        let right_candidate = alternatives[right][choice(right)];
        let mismatch = topology_catalog[left_candidate.glyph as usize]
            .shared_port_mismatch_tolerant(side, topology_catalog[right_candidate.glyph as usize]);
        cost += EDGE_TARGET_CONNECTION_WEIGHT * mismatch
            + EDGE_BUDGET_BREAK_WEIGHT * (mismatch > 0.0) as u8 as f64;
    }
    cost
}

#[allow(clippy::too_many_arguments)]
fn cleanup_orphan_excursions(
    width: u32,
    height: u32,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    initial_selected: &[usize],
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
) -> Vec<usize> {
    let baseline_loss: f64 = alternatives
        .iter()
        .zip(edge_cells)
        .filter(|(_, edge)| **edge)
        .map(|(candidates, _)| candidates[0].distance)
        .sum();
    let reference_limit = baseline_loss * 1.05 + f64::EPSILON;
    let spur_catalog: Vec<_> = charset.iter().map(bitmap_spur_penalty).collect();
    let mut selected = initial_selected.to_vec();
    let mut current_loss: f64 = alternatives
        .iter()
        .zip(edge_cells)
        .enumerate()
        .filter(|(_, (_, edge))| **edge)
        .map(|(index, (candidates, _))| candidates[selected[index]].distance)
        .sum();

    for pass in 0..EDGE_ORPHAN_PASSES {
        for step in 0..alternatives.len() {
            let index = if pass % 2 == 0 {
                step
            } else {
                alternatives.len() - 1 - step
            };
            if !edge_cells[index] || alternatives[index].len() < 2 {
                continue;
            }
            let current_candidate = alternatives[index][selected[index]];
            let current_topology = topology_catalog[current_candidate.glyph as usize];
            let current_orphan =
                orphan_excursion_penalty(index, current_topology, graph, width, height);
            if current_orphan == 0.0 {
                continue;
            }
            let current_breaks = incident_break_count(
                index,
                selected[index],
                graph,
                alternatives,
                &selected,
                topology_catalog,
            );
            let current_score = orphan_candidate_score(
                current_candidate,
                current_topology,
                target_topologies[index],
                current_orphan,
                spur_catalog[current_candidate.glyph as usize],
                current_breaks,
            );
            let mut best_choice = selected[index];
            let mut best_score = current_score;
            let mut best_selected = None;
            let mut best_loss = current_loss;
            for (candidate_index, candidate) in alternatives[index].iter().enumerate() {
                let topology = topology_catalog[candidate.glyph as usize];
                if !matches!(topology.role(), GlyphRole::Blank | GlyphRole::Solid) {
                    continue;
                }
                if candidate.distance > current_candidate.distance + EDGE_ORPHAN_MAX_REFERENCE_DELTA
                {
                    continue;
                }
                let orphan = orphan_excursion_penalty(index, topology, graph, width, height);
                let breaks = incident_break_count(
                    index,
                    candidate_index,
                    graph,
                    alternatives,
                    &selected,
                    topology_catalog,
                );
                if orphan >= current_orphan {
                    continue;
                }
                let mut trial_selected = selected.clone();
                trial_selected[index] = candidate_index;
                let mut trial_loss = current_loss - current_candidate.distance + candidate.distance;
                let Some(rollback_penalty) = make_reference_room(
                    index,
                    reference_limit,
                    &mut trial_loss,
                    graph,
                    alternatives,
                    edge_cells,
                    target_topologies,
                    &spur_catalog,
                    topology_catalog,
                    &mut trial_selected,
                ) else {
                    continue;
                };
                let score = orphan_candidate_score(
                    *candidate,
                    topology,
                    target_topologies[index],
                    orphan,
                    spur_catalog[candidate.glyph as usize],
                    breaks,
                ) + EDGE_ORPHAN_ROLLBACK_WEIGHT * rollback_penalty;
                if score < best_score
                    || (score == best_score
                        && (candidate.glyph, candidate_index)
                            < (alternatives[index][best_choice].glyph, best_choice))
                {
                    best_choice = candidate_index;
                    best_score = score;
                    best_selected = Some(trial_selected);
                    best_loss = trial_loss;
                }
            }
            if let Some(best_selected) = best_selected {
                selected = best_selected;
                current_loss = best_loss;
            }
        }
    }
    selected
}

#[allow(clippy::too_many_arguments)]
fn make_reference_room(
    protected_index: usize,
    reference_limit: f64,
    current_loss: &mut f64,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    spur_catalog: &[f64],
    topology_catalog: &[GlyphTopology],
    selected: &mut [usize],
) -> Option<f64> {
    let mut total_penalty = 0.0;
    while *current_loss > reference_limit {
        let mut best: Option<(f64, f64, usize)> = None;
        for donor in 0..selected.len() {
            if donor == protected_index || !edge_cells[donor] || selected[donor] == 0 {
                continue;
            }
            let saving =
                alternatives[donor][selected[donor]].distance - alternatives[donor][0].distance;
            if saving <= f64::EPSILON {
                continue;
            }
            let current_cost = local_target_structure_cost(
                donor,
                selected[donor],
                graph,
                alternatives,
                target_topologies,
                selected,
                spur_catalog,
                topology_catalog,
            );
            let rollback_cost = local_target_structure_cost(
                donor,
                0,
                graph,
                alternatives,
                target_topologies,
                selected,
                spur_catalog,
                topology_catalog,
            );
            let penalty = (rollback_cost - current_cost).max(0.0);
            let candidate = (penalty / saving, penalty, donor);
            if best.as_ref().map_or(true, |best| {
                candidate
                    .0
                    .total_cmp(&best.0)
                    .then_with(|| candidate.1.total_cmp(&best.1))
                    .then_with(|| candidate.2.cmp(&best.2))
                    .is_lt()
            }) {
                best = Some(candidate);
            }
        }
        let Some((_, penalty, donor)) = best else {
            return None;
        };
        let saving =
            alternatives[donor][selected[donor]].distance - alternatives[donor][0].distance;
        selected[donor] = 0;
        *current_loss -= saving;
        total_penalty += penalty;
    }
    Some(total_penalty)
}

fn orphan_candidate_score(
    candidate: GlyphCandidate,
    topology: GlyphTopology,
    target: Option<GlyphTopology>,
    orphan: f64,
    spur: f64,
    breaks: usize,
) -> f64 {
    candidate.distance
        + EDGE_TARGET_TOPOLOGY_WEIGHT
            * target
                .map(|target| topology.target_distance(target))
                .unwrap_or(0.0)
        + EDGE_SPUR_WEIGHT * spur
        + EDGE_ORPHAN_WEIGHT * orphan
        + EDGE_ORPHAN_BREAK_WEIGHT * breaks as f64
}

fn incident_break_count(
    index: usize,
    candidate_index: usize,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    selected: &[usize],
    topology_catalog: &[GlyphTopology],
) -> usize {
    let topology = topology_catalog[alternatives[index][candidate_index].glyph as usize];
    graph
        .neighbors(index)
        .iter()
        .filter(|neighbor| {
            let side = graph
                .side_between(index, **neighbor)
                .expect("contour graph neighbors must be adjacent");
            let neighbor_candidate = alternatives[**neighbor][selected[**neighbor]];
            topology.shared_port_mismatch_tolerant(
                side,
                topology_catalog[neighbor_candidate.glyph as usize],
            ) > 0.0
        })
        .count()
}

fn orphan_excursion_penalty(
    index: usize,
    topology: GlyphTopology,
    graph: &ContourGraph,
    width: u32,
    height: u32,
) -> f64 {
    let active_sides: Vec<_> = Side::ALL
        .into_iter()
        .filter(|side| topology.edge_ports(*side) != 0)
        .collect();
    if active_sides.is_empty() {
        return 0.0;
    }
    let x = index as u32 % width;
    let y = index as u32 / width;
    let unsupported = active_sides
        .iter()
        .filter(|side| {
            let border = match side {
                Side::Top => y == 0,
                Side::Right => x + 1 == width,
                Side::Bottom => y + 1 == height,
                Side::Left => x == 0,
            };
            if border {
                return false;
            }
            !graph
                .neighbors(index)
                .iter()
                .any(|neighbor| graph.side_between(index, *neighbor) == Some(**side))
        })
        .count();
    let unsupported_ratio = unsupported as f64 / active_sides.len() as f64;
    if active_sides.len() == 1 && unsupported == 1 {
        1.0
    } else {
        unsupported_ratio
    }
}

fn refine_edge_continuity(
    width: u32,
    height: u32,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    initial_selected: &[usize],
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
) -> Vec<usize> {
    let mut selected = initial_selected.to_vec();
    for pass in 0..EDGE_CONTINUITY_PASSES {
        let reverse = pass % 2 == 1;
        for step in 0..alternatives.len() {
            let index = if reverse {
                alternatives.len() - 1 - step
            } else {
                step
            };
            if !edge_cells[index] || alternatives[index].len() < 2 {
                continue;
            }
            let x = index as u32 % width;
            let y = index as u32 / width;
            let mut best = selected[index];
            let mut best_score = f64::INFINITY;
            for (candidate_index, candidate) in alternatives[index].iter().enumerate() {
                let mut continuity = 0.0;
                let mut port_continuity = 0.0;
                let mut neighbor_count = 0usize;
                let mut target_connection = 0.0;
                let mut target_neighbor_count = 0usize;
                let candidate_topology = topology_catalog[candidate.glyph as usize];
                for (neighbor_index, own_border, neighbor_border, own_side) in [
                    (
                        y.checked_sub(1).map(|ny| (ny * width + x) as usize),
                        Border::Top,
                        Border::Bottom,
                        Side::Top,
                    ),
                    (
                        (x + 1 < width).then(|| (y * width + x + 1) as usize),
                        Border::Right,
                        Border::Left,
                        Side::Right,
                    ),
                    (
                        (y + 1 < height).then(|| ((y + 1) * width + x) as usize),
                        Border::Bottom,
                        Border::Top,
                        Side::Bottom,
                    ),
                    (
                        x.checked_sub(1).map(|nx| (y * width + nx) as usize),
                        Border::Left,
                        Border::Right,
                        Side::Left,
                    ),
                ] {
                    if let Some(neighbor_index) = neighbor_index {
                        let neighbor = alternatives[neighbor_index][selected[neighbor_index]];
                        continuity += border_mismatch(
                            *candidate,
                            own_border,
                            neighbor,
                            neighbor_border,
                            charset,
                        );
                        let neighbor_topology = topology_catalog[neighbor.glyph as usize];
                        port_continuity += candidate_topology
                            .shared_port_mismatch_tolerant(own_side, neighbor_topology);
                        neighbor_count += 1;
                        if graph
                            .neighbors(index)
                            .binary_search(&neighbor_index)
                            .is_ok()
                        {
                            target_connection += candidate_topology
                                .shared_port_mismatch_tolerant(own_side, neighbor_topology);
                            target_neighbor_count += 1;
                        }
                    }
                }
                let continuity = continuity / neighbor_count.max(1) as f64;
                let port_continuity = port_continuity / neighbor_count.max(1) as f64;
                let target_connection = target_connection / target_neighbor_count.max(1) as f64;
                let target_topology = target_topologies[index]
                    .map(|target| candidate_topology.target_distance(target))
                    .unwrap_or(0.0);
                let target_sides = target_topologies[index]
                    .map(|target| target_side_mismatch(candidate_topology, target))
                    .unwrap_or(0.0);
                let spur = bitmap_spur_penalty(&charset[candidate.glyph as usize]);
                let neighborhood_spur = neighborhood_artifact_penalty(
                    index,
                    *candidate,
                    width,
                    height,
                    alternatives,
                    &selected,
                    charset,
                );
                let score = candidate.distance
                    + EDGE_CONTINUITY_WEIGHT * continuity
                    + EDGE_PORT_CONTINUITY_WEIGHT * port_continuity
                    + EDGE_TARGET_TOPOLOGY_WEIGHT * target_topology
                    + EDGE_TARGET_SIDE_WEIGHT * target_sides
                    + EDGE_TARGET_CONNECTION_WEIGHT * target_connection
                    + EDGE_SPUR_WEIGHT * spur
                    + EDGE_NEIGHBORHOOD_SPUR_WEIGHT * neighborhood_spur;
                if score < best_score
                    || (score == best_score && candidate.glyph < alternatives[index][best].glyph)
                {
                    best = candidate_index;
                    best_score = score;
                }
            }
            selected[index] = best;
        }
    }
    selected
}

#[derive(Debug, Clone, Copy)]
struct EdgeGrammarObjective {
    total: f64,
    reference_loss: f64,
}

fn edge_gate_decision(
    baseline: EdgeGrammarObjective,
    proposed: EdgeGrammarObjective,
    reference_limit: f64,
) -> EdgeGateDecision {
    if proposed.reference_loss > reference_limit {
        EdgeGateDecision::RejectedReferenceLoss
    } else if proposed.total > baseline.total {
        EdgeGateDecision::RejectedObjective
    } else {
        EdgeGateDecision::Accepted
    }
}

#[allow(clippy::too_many_arguments)]
fn measure_edge_grammar(
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    selected: &[usize],
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
    objective: EdgeGrammarObjective,
) -> EdgeGrammarMetrics {
    let mut target_port_loss = 0.0;
    let mut target_cell_count = 0usize;
    let mut target_sides = 0usize;
    let mut covered_sides = 0usize;
    let mut side_union = 0usize;
    let mut side_errors = 0usize;
    let mut false_junction_count = 0usize;
    let mut spur_cell_count = 0usize;
    let mut orphan_excursion_count = 0usize;
    let mut edited_cells = 0usize;

    for index in 0..alternatives.len() {
        edited_cells += (selected[index] != 0) as usize;
        let Some(target) = target_topologies[index] else {
            continue;
        };
        target_cell_count += 1;
        let candidate = alternatives[index][selected[index]];
        let topology = topology_catalog[candidate.glyph as usize];
        target_port_loss += topology.port_distance(target);
        for side in Side::ALL {
            let target_active = target.edge_ports(side) != 0;
            let candidate_active = topology.edge_ports(side) != 0;
            target_sides += target_active as usize;
            side_union += (target_active || candidate_active) as usize;
            covered_sides += (target_active && candidate_active) as usize;
            side_errors += (target_active != candidate_active) as usize;
        }
        false_junction_count +=
            (topology.active_sides() >= 3 && target.active_sides() < 3) as usize;
        spur_cell_count += (bitmap_spur_penalty(&charset[candidate.glyph as usize]) > 0.0) as usize;
        orphan_excursion_count +=
            (orphan_excursion_penalty(index, topology, graph, graph.width(), graph.height()) > 0.0)
                as usize;
    }

    let connections = graph.connections();
    let broken_connections = connections
        .iter()
        .filter(|(first, second, side)| {
            let first_candidate = alternatives[*first][selected[*first]];
            let second_candidate = alternatives[*second][selected[*second]];
            topology_catalog[first_candidate.glyph as usize].shared_port_mismatch_tolerant(
                *side,
                topology_catalog[second_candidate.glyph as usize],
            ) > 0.0
        })
        .count();
    let edited_denominator = edge_cells.iter().filter(|edge| **edge).count().max(1) as f64;
    EdgeGrammarMetrics {
        objective: objective.total,
        reference_loss: objective.reference_loss,
        target_port_loss: target_port_loss / target_cell_count.max(1) as f64,
        shared_port_break_rate: broken_connections as f64 / connections.len().max(1) as f64,
        unexpected_endpoint_rate: side_errors as f64 / side_union.max(1) as f64,
        contour_coverage: covered_sides as f64 / target_sides.max(1) as f64,
        false_junction_count,
        spur_cell_count,
        orphan_excursion_count,
        edited_cells,
        edited_ratio: edited_cells as f64 / edited_denominator,
    }
}

#[allow(clippy::too_many_arguments)]
fn edge_grammar_objective(
    width: u32,
    height: u32,
    graph: &ContourGraph,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    target_topologies: &[Option<GlyphTopology>],
    selected: &[usize],
    charset: &[BlockGrayImage],
    topology_catalog: &[GlyphTopology],
) -> EdgeGrammarObjective {
    let mut reference_loss = 0.0;
    let mut target_loss = 0.0;
    let mut spur_loss = 0.0;
    let mut neighborhood_spur_loss = 0.0;
    let mut orphan_loss = 0.0;
    let mut edge_count = 0usize;
    let mut color_continuity = 0.0;
    let mut port_continuity = 0.0;
    let mut pair_count = 0usize;

    for index in 0..alternatives.len() {
        if !edge_cells[index] {
            continue;
        }
        edge_count += 1;
        let candidate = alternatives[index][selected[index]];
        let topology = topology_catalog[candidate.glyph as usize];
        reference_loss += candidate.distance;
        target_loss += target_topologies[index]
            .map(|target| topology.target_distance(target))
            .unwrap_or(0.0);
        spur_loss += bitmap_spur_penalty(&charset[candidate.glyph as usize]);
        neighborhood_spur_loss += neighborhood_artifact_penalty(
            index,
            candidate,
            width,
            height,
            alternatives,
            selected,
            charset,
        );
        orphan_loss += orphan_excursion_penalty(index, topology, graph, width, height);
    }

    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            for (neighbor_index, own_border, neighbor_border, own_side) in [
                (
                    (x + 1 < width).then(|| index + 1),
                    Border::Right,
                    Border::Left,
                    Side::Right,
                ),
                (
                    (y + 1 < height).then(|| index + width as usize),
                    Border::Bottom,
                    Border::Top,
                    Side::Bottom,
                ),
            ] {
                let Some(neighbor_index) = neighbor_index else {
                    continue;
                };
                if !edge_cells[index] && !edge_cells[neighbor_index] {
                    continue;
                }
                let candidate = alternatives[index][selected[index]];
                let neighbor = alternatives[neighbor_index][selected[neighbor_index]];
                color_continuity +=
                    border_mismatch(candidate, own_border, neighbor, neighbor_border, charset);
                port_continuity += topology_catalog[candidate.glyph as usize]
                    .shared_port_mismatch_tolerant(
                        own_side,
                        topology_catalog[neighbor.glyph as usize],
                    );
                pair_count += 1;
            }
        }
    }

    let edge_denominator = edge_count.max(1) as f64;
    let pair_denominator = pair_count.max(1) as f64;
    reference_loss /= edge_denominator;
    target_loss /= edge_denominator;
    spur_loss /= edge_denominator;
    neighborhood_spur_loss /= edge_denominator;
    orphan_loss /= edge_denominator;
    color_continuity /= pair_denominator;
    port_continuity /= pair_denominator;
    let total = reference_loss
        + EDGE_TARGET_TOPOLOGY_WEIGHT * target_loss
        + EDGE_CONTINUITY_WEIGHT * color_continuity
        + EDGE_PORT_CONTINUITY_WEIGHT * port_continuity
        + EDGE_SPUR_WEIGHT * spur_loss
        + EDGE_NEIGHBORHOOD_SPUR_WEIGHT * neighborhood_spur_loss
        + EDGE_ORPHAN_WEIGHT * orphan_loss;
    EdgeGrammarObjective {
        total,
        reference_loss,
    }
}

fn neighborhood_artifact_penalty(
    center_index: usize,
    center: GlyphCandidate,
    width: u32,
    height: u32,
    alternatives: &[Vec<GlyphCandidate>],
    selected: &[usize],
    charset: &[BlockGrayImage],
) -> f64 {
    const PATCH_CELLS: usize = 3;
    const PATCH_SIZE: usize = PATCH_CELLS * GLYPH_WIDTH as usize;
    const CENTER_START: usize = GLYPH_WIDTH as usize;
    const CENTER_END: usize = CENTER_START * 2;
    let center_x = center_index as u32 % width;
    let center_y = center_index as u32 / width;
    let mut colors = vec![vec![center.bg; PATCH_SIZE]; PATCH_SIZE];
    for cell_y in 0..PATCH_CELLS {
        for cell_x in 0..PATCH_CELLS {
            let grid_x = center_x as i32 + cell_x as i32 - 1;
            let grid_y = center_y as i32 + cell_y as i32 - 1;
            if grid_x < 0 || grid_y < 0 || grid_x >= width as i32 || grid_y >= height as i32 {
                continue;
            }
            let index = (grid_y as u32 * width + grid_x as u32) as usize;
            let candidate = if cell_x == 1 && cell_y == 1 {
                center
            } else {
                alternatives[index][selected[index]]
            };
            let bitmap = &charset[candidate.glyph as usize];
            for y in 0..GLYPH_HEIGHT as usize {
                for x in 0..GLYPH_WIDTH as usize {
                    colors[cell_y * GLYPH_HEIGHT as usize + y][cell_x * GLYPH_WIDTH as usize + x] =
                        rendered_color_index(candidate, bitmap, x, y);
                }
            }
        }
    }

    let mut thin = vec![vec![false; PATCH_SIZE]; PATCH_SIZE];
    for y in 0..PATCH_SIZE {
        for x in 0..PATCH_SIZE {
            let mut same_neighbors = 0usize;
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < PATCH_SIZE as isize
                        && ny < PATCH_SIZE as isize
                        && colors[ny as usize][nx as usize] == colors[y][x]
                    {
                        same_neighbors += 1;
                    }
                }
            }
            thin[y][x] = same_neighbors <= 2;
        }
    }

    let mut visited = vec![vec![false; PATCH_SIZE]; PATCH_SIZE];
    let mut penalty = 0.0f64;
    for start_y in 0..PATCH_SIZE {
        for start_x in 0..PATCH_SIZE {
            if !thin[start_y][start_x] || visited[start_y][start_x] {
                continue;
            }
            let color = colors[start_y][start_x];
            let mut stack = vec![(start_x, start_y)];
            let mut size = 0usize;
            let mut intersects_center = false;
            let mut touches_patch_edge = false;
            visited[start_y][start_x] = true;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                intersects_center |= (CENTER_START..CENTER_END).contains(&x)
                    && (CENTER_START..CENTER_END).contains(&y);
                touches_patch_edge |=
                    x == 0 || y == 0 || x + 1 == PATCH_SIZE || y + 1 == PATCH_SIZE;
                for dy in -1isize..=1 {
                    for dx in -1isize..=1 {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx < 0
                            || ny < 0
                            || nx >= PATCH_SIZE as isize
                            || ny >= PATCH_SIZE as isize
                        {
                            continue;
                        }
                        let nx = nx as usize;
                        let ny = ny as usize;
                        if !visited[ny][nx] && thin[ny][nx] && colors[ny][nx] == color {
                            visited[ny][nx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            if intersects_center && !touches_patch_edge {
                penalty = penalty.max((size.min(8) as f64) / 8.0);
            }
        }
    }
    penalty
}

/// Penalize a tiny foreground stroke or background notch that enters from at
/// most one cell side. Shapes crossing the cell or turning through a corner
/// touch at least two sides and remain available for genuine thin contours.
pub(crate) fn bitmap_spur_penalty(bitmap: &BlockGrayImage) -> f64 {
    let foreground = bitmap
        .iter()
        .flatten()
        .filter(|pixel| **pixel >= 128)
        .count();
    let total = (GLYPH_WIDTH * GLYPH_HEIGHT) as usize;
    let minority_is_foreground = foreground <= total / 2;
    let minority_count = if minority_is_foreground {
        foreground
    } else {
        total - foreground
    };
    let minority_penalty = if minority_count == 0 || minority_count > 16 {
        0.0
    } else {
        let is_minority = |x: usize, y: usize| {
            let foreground_pixel = bitmap[y][x] >= 128;
            foreground_pixel == minority_is_foreground
        };
        let last_x = GLYPH_WIDTH as usize - 1;
        let last_y = GLYPH_HEIGHT as usize - 1;
        let touched_sides = [
            (0..=last_x).any(|x| is_minority(x, 0)),
            (0..=last_y).any(|y| is_minority(last_x, y)),
            (0..=last_x).any(|x| is_minority(x, last_y)),
            (0..=last_y).any(|y| is_minority(0, y)),
        ]
        .into_iter()
        .filter(|touched| *touched)
        .count();
        if touched_sides <= 1 {
            1.0 - (minority_count.saturating_sub(1) as f64 / 32.0)
        } else {
            0.0
        }
    };

    minority_penalty.max(thin_branch_penalty(bitmap))
}

fn thin_branch_penalty(bitmap: &BlockGrayImage) -> f64 {
    let last_x = GLYPH_WIDTH as usize - 1;
    let last_y = GLYPH_HEIGHT as usize - 1;
    let mut thin = vec![vec![false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    for y in 0..=last_y {
        for x in 0..=last_x {
            let value = bitmap[y][x] >= 128;
            let mut same_neighbors = 0usize;
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx <= last_x as isize
                        && ny <= last_y as isize
                        && (bitmap[ny as usize][nx as usize] >= 128) == value
                    {
                        same_neighbors += 1;
                    }
                }
            }
            thin[y][x] = same_neighbors <= 2;
        }
    }

    let mut visited = vec![vec![false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    let mut penalty = 0.0f64;
    for start_y in 0..=last_y {
        for start_x in 0..=last_x {
            if !thin[start_y][start_x] || visited[start_y][start_x] {
                continue;
            }
            let value = bitmap[start_y][start_x] >= 128;
            let mut stack = vec![(start_x, start_y)];
            let mut size = 0usize;
            let mut touched = [false; 4];
            visited[start_y][start_x] = true;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                touched[0] |= y == 0;
                touched[1] |= x == last_x;
                touched[2] |= y == last_y;
                touched[3] |= x == 0;
                for dy in -1isize..=1 {
                    for dx in -1isize..=1 {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx < 0 || ny < 0 || nx > last_x as isize || ny > last_y as isize {
                            continue;
                        }
                        let nx = nx as usize;
                        let ny = ny as usize;
                        if !visited[ny][nx] && thin[ny][nx] && (bitmap[ny][nx] >= 128) == value {
                            visited[ny][nx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            let touched_sides = touched.into_iter().filter(|side| *side).count();
            if touched_sides <= 1 {
                penalty = penalty.max((size.min(8) as f64) / 8.0);
            }
        }
    }
    penalty
}

fn border_mismatch(
    first: GlyphCandidate,
    first_border: Border,
    second: GlyphCandidate,
    second_border: Border,
    charset: &[BlockGrayImage],
) -> f64 {
    let mut mismatch = 0.0;
    for offset in 0..GLYPH_WIDTH as usize {
        let first_rgb = rendered_border_rgb(first, first_border, offset, charset);
        let second_rgb = rendered_border_rgb(second, second_border, offset, charset);
        mismatch += first_rgb
            .iter()
            .zip(second_rgb.iter())
            .map(|(a, b)| (*a as f64 - *b as f64).abs())
            .sum::<f64>()
            / (255.0 * 3.0);
    }
    mismatch / GLYPH_WIDTH as f64
}

fn rendered_border_rgb(
    candidate: GlyphCandidate,
    border: Border,
    offset: usize,
    charset: &[BlockGrayImage],
) -> [u8; 3] {
    let (x, y) = match border {
        Border::Top => (offset, 0),
        Border::Right => (GLYPH_WIDTH as usize - 1, offset),
        Border::Bottom => (offset, GLYPH_HEIGHT as usize - 1),
        Border::Left => (0, offset),
    };
    let color = rendered_color_index(candidate, &charset[candidate.glyph as usize], x, y);
    ANSI_COLOR_RGB[color as usize]
}

fn rendered_color_index(
    candidate: GlyphCandidate,
    bitmap: &BlockGrayImage,
    x: usize,
    y: usize,
) -> u8 {
    if bitmap[y][x] >= 128 {
        candidate.fg
    } else {
        candidate.bg
    }
}

fn palette_luma(index: u8) -> f32 {
    let color = ANSI_COLOR_RGB[index as usize];
    0.299 * color[0] as f32 + 0.587 * color[1] as f32 + 0.114 * color[2] as f32
}

fn sobel_block(block: &BlockGrayImage) -> BlockGrayImage {
    let mut edges = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    for y in 1..GLYPH_HEIGHT as usize - 1 {
        for x in 1..GLYPH_WIDTH as usize - 1 {
            let sample = |dx: isize, dy: isize| {
                block[(y as isize + dy) as usize][(x as isize + dx) as usize] as i32
            };
            let gx = -sample(-1, -1) + sample(1, -1) - 2 * sample(-1, 0) + 2 * sample(1, 0)
                - sample(-1, 1)
                + sample(1, 1);
            let gy = -sample(-1, -1) - 2 * sample(0, -1) - sample(1, -1)
                + sample(-1, 1)
                + 2 * sample(0, 1)
                + sample(1, 1);
            edges[y][x] = (gx.abs() + gy.abs()).min(255) as u8;
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render_grid, PetsciiCell, PetsciiGrid};
    use image::{ImageBuffer, Rgba};

    fn built_dither_ladder() -> DitherLadder {
        let charset = gen_charset_images(
            false,
            GLYPH_WIDTH as usize,
            GLYPH_HEIGHT as usize,
            &C64LOW,
            &C64UP,
        );
        DitherLadder::build(&charset)
    }

    #[test]
    fn dither_vocabulary_admits_the_fine_checker_and_rejects_shapes() {
        let ladder = built_dither_ladder();
        let admissible = ladder.admissible_glyphs();
        // The classic 50% checkerboard is the canonical mid-tone dither.
        assert!(admissible.contains(&102));
        // Space and solid endpoints anchor the extremes.
        assert!(admissible.contains(&SPACE_GLYPH));
        assert!(admissible.contains(&SOLID_GLYPH));
        // A clustered shape (a solid half-block) never reads as a tone.
        let mut half_block = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        for row in half_block.iter_mut().take(GLYPH_HEIGHT as usize / 2) {
            for pixel in row.iter_mut() {
                *pixel = 255;
            }
        }
        assert!(!is_even_stipple(&half_block));
    }

    fn built_charset() -> Vec<BlockGrayImage> {
        gen_charset_images(
            false,
            GLYPH_WIDTH as usize,
            GLYPH_HEIGHT as usize,
            &C64LOW,
            &C64UP,
        )
    }

    #[test]
    fn fill_boundary_catalog_has_triangles_and_axis_fills() {
        let catalog = FillBoundaryCatalog::build(&built_charset());
        let members: Vec<u8> = catalog.glyphs.iter().map(|(glyph, _)| *glyph).collect();
        // The four 45-degree triangles.
        for triangle in [95, 105, 223, 233] {
            assert!(members.contains(&triangle), "missing triangle {triangle}");
        }
        // A horizontal bottom-half fill (glyph 98) and a vertical fill are axis-aligned members.
        assert!(members.contains(&98), "missing bottom-half fill");
        assert!(members.len() >= 12, "catalog too small: {}", members.len());
    }

    #[test]
    fn is_triangle_glyph_accepts_triangles_and_rejects_blocks() {
        let charset = built_charset();
        let mask_of = |glyph: usize| {
            let mut mask = [[false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
            for (y, row) in mask.iter_mut().enumerate() {
                for (x, pixel) in row.iter_mut().enumerate() {
                    *pixel = charset[glyph][y][x] >= 128;
                }
            }
            mask
        };
        assert!(is_triangle_glyph(&mask_of(223)));
        assert!(is_triangle_glyph(&mask_of(95)));
        // A solid bottom-half block is a clean axis fill, not a diagonal triangle.
        assert!(!is_triangle_glyph(&mask_of(98)));
        // Solid and blank are not triangles.
        assert!(!is_triangle_glyph(&mask_of(SOLID_GLYPH as usize)));
        assert!(!is_triangle_glyph(&mask_of(SPACE_GLYPH as usize)));
    }

    #[test]
    fn best_match_selects_a_horizontal_fill_for_a_shallow_boundary() {
        let catalog = FillBoundaryCatalog::build(&built_charset());
        // Foreground occupies the bottom three rows: a shallow, near-horizontal boundary.
        let mut foreground = [[false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        for row in foreground.iter_mut().skip(5) {
            for pixel in row.iter_mut() {
                *pixel = true;
            }
        }
        let (glyph, agreement) = catalog.best_match(&foreground).unwrap();
        assert!(agreement > 0.95, "weak match: {agreement}");
        // Glyph 121 is the bottom-three-rows fill.
        assert_eq!(glyph, 121);
    }

    #[test]
    fn slope_conversion_draws_a_fill_boundary_glyph_on_a_clean_diagonal() {
        let charset = built_charset();
        let catalog_members: Vec<u8> = FillBoundaryCatalog::build(&charset)
            .glyphs
            .iter()
            .map(|(glyph, _)| *glyph)
            .collect();
        // Paint an 8x8 cell as the lower-left triangle glyph: dark where lit, light elsewhere.
        let mut image = ImageBuffer::from_pixel(8, 8, Rgba([255, 255, 255, 255]));
        for y in 0..8u32 {
            for x in 0..8u32 {
                if charset[223][y as usize][x as usize] >= 128 {
                    image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            }
        }
        let image = DynamicImage::ImageRgba8(image);
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 2,
            top_k: 1,
            contrast: 0.0,
        };
        let with_slopes = convert_image_styled(&image, &config, false, true).unwrap();
        let glyph = with_slopes.grid.cells[0].glyph;
        assert!(
            catalog_members.contains(&glyph),
            "slope path did not pick a fill-boundary glyph, got {glyph}"
        );
        // Deterministic.
        let again = convert_image_styled(&image, &config, false, true).unwrap();
        assert_eq!(with_slopes.grid, again.grid);
    }

    #[test]
    fn dither_regularizer_reverts_low_gain_cells_next_to_flat() {
        // A corpus where flat-next-to-flat dominates, so texturing a cell beside a flat neighbor
        // carries a real layout cost.
        let mut report = crate::corpus::empty_corpus_report();
        report.visible_glyph_counts[32] = 10_000;
        report.horizontal_bigram_counts.insert("32:32".into(), 10_000);
        report.vertical_bigram_counts.insert("32:32".into(), 10_000);
        let prior = CorpusPrior::from_report(&report);

        let dither = GlyphCandidate {
            glyph: 102,
            distance: 0.0,
            fg: 1,
            bg: 0,
            texture: 1,
        };
        // Cell 0 is a dither cell (fallback solid at index 1); cell 1 is already flat.
        let alternatives = vec![
            vec![dither, solid_candidate(SOLID_GLYPH, 0, 0)],
            vec![solid_candidate(SOLID_GLYPH, 0, 0)],
        ];

        let mut selected = vec![0, 0];
        let mut dither_cells = vec![true, false];
        let reverted = regularize_dither(
            &prior,
            2,
            1,
            &alternatives,
            &mut selected,
            &mut dither_cells,
            &[0.01, 0.0],
        );
        assert_eq!(reverted, 1);
        assert_eq!(selected, vec![1, 0]);
        assert!(!dither_cells[0]);

        // A large perceived-tone gain keeps the dither despite the layout cost.
        let mut selected = vec![0, 0];
        let mut dither_cells = vec![true, false];
        let kept = regularize_dither(
            &prior,
            2,
            1,
            &alternatives,
            &mut selected,
            &mut dither_cells,
            &[1000.0, 0.0],
        );
        assert_eq!(kept, 0);
        assert_eq!(selected, vec![0, 0]);
    }

    #[test]
    fn dither_ladder_prefers_the_finest_glyph_at_fifty_percent() {
        let ladder = built_dither_ladder();
        let glyph = ladder.pick(0.5);
        assert!((ladder.fill[glyph as usize] - 0.5).abs() < f64::EPSILON);
        // Among 50%-fill glyphs the finest hatch has the highest dispersion.
        assert!(ladder.dispersion[glyph as usize] >= 0.99);
    }

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

    #[test]
    fn mode_two_candidates_never_include_letters_or_digits() {
        let image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8, 8, Rgba([10, 10, 10, 255])));
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 2,
            top_k: 16,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        assert!(result
            .alternatives
            .iter()
            .flatten()
            .all(|candidate| glyph_allowed(2, candidate.glyph)));
    }

    #[test]
    fn mode_two_rejects_noisy_punctuation_in_both_polarities() {
        for glyph in [33, 37, 38, 161, 165, 166] {
            assert!(!glyph_allowed(2, glyph));
        }
        assert!(glyph_allowed(0, 33));
        assert!(glyph_allowed(1, 161));
    }

    #[test]
    fn flat_background_uses_space_and_detected_background_color() {
        let image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 8, Rgba([64, 91, 137, 255])));
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        assert!(result
            .grid
            .cells
            .iter()
            .all(|cell| cell.glyph == SPACE_GLYPH && cell.bg != 0));
    }

    #[test]
    fn flat_non_background_region_uses_solid_glyph() {
        let mut image = ImageBuffer::from_pixel(24, 8, Rgba([64, 91, 137, 255]));
        for y in 0..8 {
            for x in 16..24 {
                image.put_pixel(x, y, Rgba([235, 235, 225, 255]));
            }
        }
        let config = ConversionConfig {
            width: 3,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let result = convert_image(&DynamicImage::ImageRgba8(image), &config).unwrap();
        assert_eq!(result.grid.cells[0].glyph, SPACE_GLYPH);
        assert_eq!(result.grid.cells[1].glyph, SPACE_GLYPH);
        assert_eq!(result.grid.cells[2].glyph, SOLID_GLYPH);
    }

    #[test]
    fn mode_two_uses_detected_scene_background_when_present() {
        let sky = Rgba([18, 101, 178, 255]);
        let reference = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8, 8, sky));
        let gray = reference.clone().into_luma8();
        let background_rgb = ((sky[0] as u32) << 24)
            | ((sky[1] as u32) << 16)
            | ((sky[2] as u32) << 8)
            | sky[3] as u32;
        let expected = find_best_color_u32(background_rgb) as u8;

        let (bg, _) = select_cell_colors(&reference, &gray, 0, 0, 2, background_rgb);

        assert_eq!(bg, expected);
        assert_ne!(bg, 0);
    }

    #[test]
    fn mode_two_uses_local_dark_background_when_sky_is_absent() {
        let mut image = ImageBuffer::from_pixel(8, 8, Rgba([24, 24, 24, 255]));
        for y in 0..8 {
            for x in 4..8 {
                image.put_pixel(x, y, Rgba([58, 58, 58, 255]));
            }
        }
        let reference = DynamicImage::ImageRgba8(image);
        let gray = reference.clone().into_luma8();
        let sky = Rgba([18, 101, 178, 255]);
        let background_rgb = ((sky[0] as u32) << 24)
            | ((sky[1] as u32) << 16)
            | ((sky[2] as u32) << 8)
            | sky[3] as u32;
        let sky_index = find_best_color_u32(background_rgb) as u8;

        let (bg, fg) = select_cell_colors(&reference, &gray, 0, 0, 2, background_rgb);

        assert_ne!(bg, sky_index);
        assert_ne!(fg, sky_index);
        assert_ne!(bg, fg);
    }

    #[test]
    fn edge_target_prefers_matching_fill_side() {
        let mut input = vec![vec![0u8; 8]; 8];
        for row in &mut input {
            row[4..].fill(255);
        }
        let target = EdgeTarget::new(&input, 15, 0);
        let matching = input.clone();
        let inverted: BlockGrayImage = input
            .iter()
            .map(|row| row.iter().map(|pixel| 255 - pixel).collect())
            .collect();
        assert_eq!(target.distance(&matching), 0.0);
        assert!(target.distance(&matching) < target.distance(&inverted));
    }

    #[test]
    fn structure_protection_restores_a_supported_internal_edge() {
        let mut internal = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        for row in internal.iter_mut().take(5).skip(3) {
            for pixel in row.iter_mut().take(5).skip(3) {
                *pixel = 255;
            }
        }
        let blank = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        let topology_catalog = vec![
            GlyphTopology::from_bitmap(&internal),
            GlyphTopology::from_bitmap(&blank),
        ];
        let targets = vec![Some(GlyphTopology::from_bitmap(&internal))];
        let graph = ContourGraph::from_targets(1, 1, &targets);
        let alternatives = vec![vec![
            GlyphCandidate {
                glyph: 0,
                distance: 0.0,
                fg: 15,
                bg: 0,
                texture: 1,
            },
            GlyphCandidate {
                glyph: 1,
                distance: 0.01,
                fg: 15,
                bg: 0,
                texture: 1,
            },
        ]];
        let (selected, rollbacks) = preserve_reference_structure(
            &graph,
            &alternatives,
            &[true],
            &targets,
            &[1],
            &topology_catalog,
        );

        assert_eq!(selected, vec![0]);
        assert_eq!(rollbacks, 1);
    }

    #[test]
    fn sobel_detects_edge_crossing_character_cells() {
        let mut image = GrayImage::new(16, 8);
        for y in 0..8 {
            for x in 8..16 {
                image.put_pixel(x, y, Luma([255]));
            }
        }
        let edges = sobel_image(&image);
        let left = get_grayscale_block_at(&edges, 0, 0, 8, 8);
        let right = get_grayscale_block_at(&edges, 1, 0, 8, 8);
        assert!(is_edge_cell(&left));
        assert!(is_edge_cell(&right));
    }

    #[test]
    fn edge_cleanup_removes_tiny_components_but_keeps_connected_contours() {
        let mut edges = GrayImage::new(16, 8);
        edges.put_pixel(1, 1, Luma([255]));
        edges.put_pixel(2, 1, Luma([64]));
        for x in 5..13 {
            edges.put_pixel(x, 4, Luma([if x == 8 { 255 } else { 64 }]));
        }
        let cleaned = clean_edge_image(&edges);
        assert_eq!(cleaned.get_pixel(1, 1).0[0], 0);
        assert_eq!(cleaned.get_pixel(2, 1).0[0], 0);
        assert_eq!(cleaned.get_pixel(5, 4).0[0], 64);
        assert_eq!(cleaned.get_pixel(8, 4).0[0], 255);
    }

    #[test]
    fn continuity_selection_rejects_a_dangling_cell_border() {
        let charset = gen_charset_images(false, 8, 8, &C64LOW, &C64UP);
        let continuous = GlyphCandidate {
            glyph: SPACE_GLYPH,
            distance: 0.02,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let dangling = GlyphCandidate {
            glyph: SOLID_GLYPH,
            distance: 0.0,
            fg: 0,
            bg: 6,
            texture: 1,
        };
        let blue_space = solid_candidate(SPACE_GLYPH, 6, 6);
        let alternatives = vec![
            vec![blue_space],
            vec![dangling, continuous],
            vec![blue_space],
        ];
        let topology_catalog = build_topology_catalog(&charset);
        let graph = ContourGraph::from_targets(3, 1, &[None, None, None]);
        let selected = refine_edge_continuity(
            3,
            1,
            &graph,
            &alternatives,
            &[false, true, false],
            &[None, None, None],
            &[0, 0, 0],
            &charset,
            &topology_catalog,
        );
        assert_eq!(selected[1], 1);
    }

    #[test]
    fn topology_selection_aligns_ports_even_when_rendered_colors_are_equal() {
        let horizontal_half = |start_y: usize| {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            for row in bitmap.iter_mut().skip(start_y) {
                row.fill(255);
            }
            bitmap
        };
        let charset = vec![horizontal_half(4), horizontal_half(6)];
        let topology_catalog = build_topology_catalog(&charset);
        let aligned = GlyphCandidate {
            glyph: 0,
            distance: 0.02,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let shifted = GlyphCandidate {
            glyph: 1,
            distance: 0.0,
            ..aligned
        };
        let alternatives = vec![vec![aligned], vec![shifted, aligned], vec![aligned]];
        let graph = ContourGraph::from_targets(3, 1, &[None, None, None]);

        let selected = refine_edge_continuity(
            3,
            1,
            &graph,
            &alternatives,
            &[false, true, false],
            &[None, None, None],
            &[0, 0, 0],
            &charset,
            &topology_catalog,
        );

        assert_eq!(selected[1], 1);
    }

    #[test]
    fn contour_chain_optimizer_selects_one_consistent_sequence() {
        let horizontal_half = |start_y: usize| {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            for row in bitmap.iter_mut().skip(start_y) {
                row.fill(255);
            }
            bitmap
        };
        let charset = vec![horizontal_half(4), horizontal_half(6)];
        let topology_catalog = build_topology_catalog(&charset);
        let aligned = GlyphCandidate {
            glyph: 0,
            distance: 0.02,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let shifted = GlyphCandidate {
            glyph: 1,
            distance: 0.0,
            ..aligned
        };
        let alternatives = vec![vec![aligned], vec![shifted, aligned], vec![aligned]];
        let target = GlyphTopology::from_bitmap(&charset[0]);
        let targets = vec![Some(target); 3];
        let graph = ContourGraph::from_targets(3, 1, &targets);

        let first = optimize_contour_chains(&graph, &alternatives, &targets, &topology_catalog);
        let second = optimize_contour_chains(&graph, &alternatives, &targets, &topology_catalog);

        assert_eq!(first, vec![0, 1, 0]);
        assert_eq!(first, second);
    }

    #[test]
    fn closed_loop_optimizer_prices_the_closing_seam() {
        let quadrant = |top: bool, left: bool| {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            let y_range = if top { 0..4 } else { 4..8 };
            let x_range = if left { 0..4 } else { 4..8 };
            for y in y_range {
                for x in x_range.clone() {
                    bitmap[y][x] = 255;
                }
            }
            bitmap
        };
        let charset = vec![
            quadrant(false, false),
            quadrant(false, true),
            quadrant(true, false),
            quadrant(true, true),
            vec![vec![0u8; 8]; 8],
        ];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let alternatives = vec![
            vec![candidate(0, 0.0)],
            vec![candidate(1, 0.0)],
            vec![candidate(4, 0.0), candidate(2, 0.02)],
            vec![candidate(3, 0.0)],
        ];
        let targets: Vec<_> = [0usize, 1, 2, 3]
            .into_iter()
            .map(|glyph| Some(topology_catalog[glyph]))
            .collect();
        let graph = ContourGraph::from_targets(2, 2, &targets);

        let selected = optimize_contour_chains(&graph, &alternatives, &targets, &topology_catalog);

        assert_eq!(selected, vec![0, 0, 1, 0]);
    }

    #[test]
    fn junction_coordination_selects_all_incident_ports() {
        let vertical = || {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            for row in &mut bitmap {
                row[3..5].fill(255);
            }
            bitmap
        };
        let horizontal = || {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            for row in bitmap.iter_mut().skip(3).take(2) {
                row.fill(255);
            }
            bitmap
        };
        let mut t_junction = vertical();
        for row in t_junction.iter_mut().skip(3).take(2) {
            row[4..].fill(255);
        }
        let charset = vec![vertical(), horizontal(), t_junction, vec![vec![0u8; 8]; 8]];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let fallback = vec![candidate(3, 0.0)];
        let mut alternatives = vec![fallback.clone(); 9];
        alternatives[1] = vec![candidate(0, 0.0)];
        alternatives[4] = vec![candidate(3, 0.0), candidate(2, 0.02)];
        alternatives[5] = vec![candidate(1, 0.0)];
        alternatives[7] = vec![candidate(0, 0.0)];
        let mut targets = vec![None; 9];
        targets[1] = Some(topology_catalog[0]);
        targets[4] = Some(topology_catalog[2]);
        targets[5] = Some(topology_catalog[1]);
        targets[7] = Some(topology_catalog[0]);
        let graph = ContourGraph::from_targets(3, 3, &targets);
        let mut first = vec![0usize; 9];
        let mut second = first.clone();

        coordinate_junctions(
            &graph,
            &alternatives,
            &targets,
            &topology_catalog,
            &mut first,
        );
        coordinate_junctions(
            &graph,
            &alternatives,
            &targets,
            &topology_catalog,
            &mut second,
        );

        assert_eq!(first[4], 1);
        assert_eq!(first, second);
    }

    #[test]
    fn edge_metrics_distinguish_connected_and_broken_ports() {
        let horizontal_fill = |start_y: usize| {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            for row in bitmap.iter_mut().skip(start_y) {
                row.fill(255);
            }
            bitmap
        };
        let charset = vec![horizontal_fill(4), horizontal_fill(6)];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8| GlyphCandidate {
            glyph,
            distance: 0.0,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let alternatives = vec![vec![candidate(0)], vec![candidate(0), candidate(1)]];
        let targets = vec![Some(topology_catalog[0]); 2];
        let graph = ContourGraph::from_targets(2, 1, &targets);
        let objective = EdgeGrammarObjective {
            total: 0.0,
            reference_loss: 0.0,
        };
        let connected = measure_edge_grammar(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &[0, 0],
            &charset,
            &topology_catalog,
            objective,
        );
        let broken = measure_edge_grammar(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &[0, 1],
            &charset,
            &topology_catalog,
            objective,
        );

        assert_eq!(connected.shared_port_break_rate, 0.0);
        assert!(broken.shared_port_break_rate > connected.shared_port_break_rate);
        assert!(broken.target_port_loss > connected.target_port_loss);
    }

    #[test]
    fn reference_budget_uses_intermediate_candidates_deterministically() {
        let mut blank = vec![vec![0u8; 8]; 8];
        let mut vertical_fill = vec![vec![0u8; 8]; 8];
        for row in &mut vertical_fill {
            row[4..].fill(255);
        }
        let charset = vec![std::mem::take(&mut blank), vertical_fill];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let alternatives = vec![
            vec![candidate(0, 0.1), candidate(1, 0.105), candidate(1, 0.2)],
            vec![candidate(0, 0.1), candidate(1, 0.105), candidate(1, 0.2)],
        ];
        let targets = vec![Some(topology_catalog[1]); 2];
        let graph = ContourGraph::from_targets(1, 2, &targets);
        let proposed = [2usize, 2];
        let first = constrain_reference_loss(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &proposed,
            &charset,
            &topology_catalog,
        );
        let second = constrain_reference_loss(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &proposed,
            &charset,
            &topology_catalog,
        );
        let reference_loss: f64 = first
            .iter()
            .enumerate()
            .map(|(index, selected)| alternatives[index][*selected].distance)
            .sum();

        assert_eq!(first, vec![1, 1]);
        assert_eq!(first, second);
        assert!(reference_loss <= 0.21 + f64::EPSILON);
    }

    #[test]
    fn pair_repair_connects_both_ends_within_reference_budget() {
        let horizontal_fill = |start_y: usize| {
            let mut bitmap = vec![vec![0u8; 8]; 8];
            for row in bitmap.iter_mut().skip(start_y) {
                row.fill(255);
            }
            bitmap
        };
        let charset = vec![horizontal_fill(4), horizontal_fill(6)];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let alternatives = vec![
            vec![candidate(1, 0.1), candidate(0, 0.11)],
            vec![candidate(0, 0.1), candidate(1, 0.11)],
        ];
        let targets = vec![Some(topology_catalog[0]); 2];
        let graph = ContourGraph::from_targets(2, 1, &targets);
        let first = repair_target_connection_pairs(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &[0, 0],
            &charset,
            &topology_catalog,
        );
        let second = repair_target_connection_pairs(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &[0, 0],
            &charset,
            &topology_catalog,
        );
        let selected_first = topology_catalog[alternatives[0][first[0]].glyph as usize];
        let selected_second = topology_catalog[alternatives[1][first[1]].glyph as usize];

        assert_eq!(first, second);
        assert_eq!(
            selected_first.shared_port_mismatch_tolerant(Side::Right, selected_second),
            0.0
        );
    }

    #[test]
    fn orphan_cleanup_removes_an_unsupported_single_side_stroke() {
        let blank = vec![vec![0u8; 8]; 8];
        let mut endpoint = blank.clone();
        for row in endpoint.iter_mut().take(5) {
            row[3..5].fill(255);
        }
        let charset = vec![blank, endpoint];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 0,
            texture: 1,
        };
        let fallback = vec![candidate(0, 0.0)];
        let mut alternatives = vec![fallback; 9];
        alternatives[4] = vec![candidate(1, 0.1), candidate(0, 0.102)];
        let mut edge_cells = vec![false; 9];
        edge_cells[4] = true;
        let mut targets = vec![None; 9];
        targets[4] = Some(topology_catalog[1]);
        let graph = ContourGraph::from_targets(3, 3, &targets);

        let first = cleanup_orphan_excursions(
            3,
            3,
            &graph,
            &alternatives,
            &edge_cells,
            &targets,
            &[0; 9],
            &charset,
            &topology_catalog,
        );
        let second = cleanup_orphan_excursions(
            3,
            3,
            &graph,
            &alternatives,
            &edge_cells,
            &targets,
            &[0; 9],
            &charset,
            &topology_catalog,
        );

        assert_eq!(first[4], 1);
        assert_eq!(first, second);
    }

    #[test]
    fn orphan_cleanup_swaps_low_value_edits_to_stay_within_reference_budget() {
        let blank = vec![vec![0u8; 8]; 8];
        let mut endpoint = blank.clone();
        for row in endpoint.iter_mut().take(5) {
            row[3..5].fill(255);
        }
        let charset = vec![blank, endpoint];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 0,
            texture: 1,
        };
        let mut alternatives = vec![vec![candidate(0, 0.1)]; 12];
        alternatives[5] = vec![candidate(1, 0.1), candidate(0, 0.13)];
        alternatives[10] = vec![candidate(0, 0.1), candidate(1, 0.13)];
        alternatives[11] = vec![candidate(0, 0.1), candidate(1, 0.12)];
        let edge_cells = vec![true; 12];
        let mut targets = vec![None; 12];
        targets[5] = Some(topology_catalog[1]);
        let graph = ContourGraph::from_targets(4, 3, &targets);

        let selected = cleanup_orphan_excursions(
            4,
            3,
            &graph,
            &alternatives,
            &edge_cells,
            &targets,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            &charset,
            &topology_catalog,
        );
        let reference_loss: f64 = alternatives
            .iter()
            .zip(&edge_cells)
            .enumerate()
            .filter(|(_, (_, edge))| **edge)
            .map(|(index, (candidates, _))| candidates[selected[index]].distance)
            .sum();

        assert_eq!(selected, vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(reference_loss <= 1.26 + f64::EPSILON);
    }

    #[test]
    fn orphan_cleanup_preserves_a_real_edge_when_blank_is_too_distant() {
        let blank = vec![vec![0u8; 8]; 8];
        let mut endpoint = blank.clone();
        for row in endpoint.iter_mut().take(5) {
            row[3..5].fill(255);
        }
        let charset = vec![blank, endpoint];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8, distance: f64| GlyphCandidate {
            glyph,
            distance,
            fg: 6,
            bg: 0,
            texture: 1,
        };
        let alternatives = vec![vec![candidate(1, 0.1), candidate(0, 0.15)]];
        let targets = vec![Some(topology_catalog[1])];
        let graph = ContourGraph::from_targets(1, 1, &targets);

        let selected = cleanup_orphan_excursions(
            1,
            1,
            &graph,
            &alternatives,
            &[true],
            &targets,
            &[0],
            &charset,
            &topology_catalog,
        );

        assert_eq!(selected, vec![0]);
    }

    #[test]
    fn reference_repaint_recomputes_color_for_the_final_glyph_regions() {
        let charset = vec![vec![vec![0u8; 8]; 8]];
        let target_rgb = ANSI_COLOR_RGB[254];
        let reference =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 8, image::Rgb(target_rgb)));
        let mut alternatives = vec![vec![GlyphCandidate {
            glyph: 0,
            distance: 0.0,
            fg: 1,
            bg: 25,
            texture: 1,
        }]];

        repaint_selected_colors(&reference, 1, 1, &[0], &mut alternatives, &charset, &[false]);

        assert_eq!(ANSI_COLOR_RGB[alternatives[0][0].bg as usize], target_rgb);
    }

    #[test]
    fn reference_repaint_preserves_a_color_jump_present_in_the_reference() {
        let charset = vec![vec![vec![0u8; 8]; 8]];
        let left_rgb = ANSI_COLOR_RGB[25];
        let right_rgb = ANSI_COLOR_RGB[254];
        let mut reference = image::RgbImage::new(16, 8);
        for y in 0..8 {
            for x in 0..16 {
                reference.put_pixel(x, y, image::Rgb(if x < 8 { left_rgb } else { right_rgb }));
            }
        }
        let reference = DynamicImage::ImageRgb8(reference);
        let candidate = GlyphCandidate {
            glyph: 0,
            distance: 0.0,
            fg: 1,
            bg: 1,
            texture: 1,
        };
        let mut alternatives = vec![vec![candidate], vec![candidate]];

        repaint_selected_colors(&reference, 2, 1, &[0, 0], &mut alternatives, &charset, &[false, false]);

        assert_eq!(ANSI_COLOR_RGB[alternatives[0][0].bg as usize], left_rgb);
        assert_eq!(ANSI_COLOR_RGB[alternatives[1][0].bg as usize], right_rgb);
    }

    #[test]
    fn reference_repaint_uses_ciede2000_instead_of_rgb_nearest() {
        let charset = vec![vec![vec![0u8; 8]; 8]];
        let sky_rgb = [65, 101, 150];
        let reference =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 8, image::Rgb(sky_rgb)));
        let mut alternatives = vec![vec![GlyphCandidate {
            glyph: 0,
            distance: 0.0,
            fg: 1,
            bg: 60,
            texture: 1,
        }]];

        repaint_selected_colors(&reference, 1, 1, &[0], &mut alternatives, &charset, &[false]);

        let selected = ANSI_COLOR_RGB[alternatives[0][0].bg as usize];
        let reference = RGB {
            r: sky_rgb[0],
            g: sky_rgb[1],
            b: sky_rgb[2],
        };
        let selected = RGB {
            r: selected[0],
            g: selected[1],
            b: selected[2],
        };
        let gray_purple = RGB {
            r: ANSI_COLOR_RGB[60][0],
            g: ANSI_COLOR_RGB[60][1],
            b: ANSI_COLOR_RGB[60][2],
        };
        assert!(
            color_distance_rgb(&reference, &selected)
                < color_distance_rgb(&reference, &gray_purple)
        );
        assert_ne!(alternatives[0][0].bg, 60);
    }

    #[test]
    fn top_one_conversion_bypasses_cross_cell_candidate_expansion() {
        let mut image = image::RgbImage::new(16, 8);
        for y in 0..8 {
            for x in 0..16 {
                image.put_pixel(
                    x,
                    y,
                    image::Rgb(if x % 8 < 4 {
                        [20, 40, 80]
                    } else {
                        [220, 230, 240]
                    }),
                );
            }
        }
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 2,
            top_k: 16,
            contrast: 0.0,
        };

        let result = convert_image_top1(&DynamicImage::ImageRgb8(image), &config).unwrap();

        assert!(result
            .alternatives
            .iter()
            .all(|candidates| candidates.len() == 1));
        assert_eq!(result.edge_grammar.final_metrics.edited_cells, 0);
    }

    #[test]
    fn edge_metrics_detect_dangling_and_overconnected_glyphs() {
        let mut straight = vec![vec![0u8; 8]; 8];
        for row in straight.iter_mut().skip(4) {
            row.fill(255);
        }
        let mut endpoint = vec![vec![0u8; 8]; 8];
        for row in endpoint.iter_mut().skip(3).take(2) {
            row[..6].fill(255);
        }
        let mut junction = vec![vec![0u8; 8]; 8];
        for row in &mut junction {
            row[3..5].fill(255);
        }
        for row in junction.iter_mut().skip(3).take(2) {
            row[..4].fill(255);
        }
        let charset = vec![straight, endpoint, junction];
        let topology_catalog = build_topology_catalog(&charset);
        let candidate = |glyph: u8| GlyphCandidate {
            glyph,
            distance: 0.0,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let alternatives = vec![
            vec![candidate(0), candidate(1), candidate(2)],
            vec![candidate(0)],
        ];
        let targets = vec![Some(topology_catalog[0]); 2];
        let graph = ContourGraph::from_targets(2, 1, &targets);
        let objective = EdgeGrammarObjective {
            total: 0.0,
            reference_loss: 0.0,
        };
        let dangling = measure_edge_grammar(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &[1, 0],
            &charset,
            &topology_catalog,
            objective,
        );
        let overconnected = measure_edge_grammar(
            &graph,
            &alternatives,
            &[true, true],
            &targets,
            &[2, 0],
            &charset,
            &topology_catalog,
            objective,
        );

        assert!(dangling.unexpected_endpoint_rate > 0.0);
        assert_eq!(overconnected.false_junction_count, 1);
        assert!(overconnected.unexpected_endpoint_rate > 0.0);
    }

    #[test]
    fn edge_gate_reports_acceptance_and_both_fallback_reasons() {
        let baseline = EdgeGrammarObjective {
            total: 1.0,
            reference_loss: 0.5,
        };
        assert_eq!(
            edge_gate_decision(
                baseline,
                EdgeGrammarObjective {
                    total: 0.9,
                    reference_loss: 0.51,
                },
                0.525,
            ),
            EdgeGateDecision::Accepted
        );
        assert_eq!(
            edge_gate_decision(
                baseline,
                EdgeGrammarObjective {
                    total: 1.1,
                    reference_loss: 0.5,
                },
                0.525,
            ),
            EdgeGateDecision::RejectedObjective
        );
        assert_eq!(
            edge_gate_decision(
                baseline,
                EdgeGrammarObjective {
                    total: 0.8,
                    reference_loss: 0.53,
                },
                0.525,
            ),
            EdgeGateDecision::RejectedReferenceLoss
        );
    }

    #[test]
    fn mode_two_edge_report_serializes_deterministically() {
        let mut image = ImageBuffer::from_pixel(24, 8, Rgba([30, 30, 30, 255]));
        for y in 4..8 {
            for x in 0..24 {
                image.put_pixel(x, y, Rgba([230, 230, 230, 255]));
            }
        }
        let config = ConversionConfig {
            width: 3,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let first = convert_image(&DynamicImage::ImageRgba8(image.clone()), &config).unwrap();
        let second = convert_image(&DynamicImage::ImageRgba8(image), &config).unwrap();
        let first_json = serde_json::to_vec_pretty(&first.edge_grammar).unwrap();
        let second_json = serde_json::to_vec_pretty(&second.edge_grammar).unwrap();

        assert_ne!(first.edge_grammar.decision, EdgeGateDecision::Disabled);
        assert!(first.edge_grammar.edge_cells > 0);
        assert_eq!(first_json, second_json);
        assert_eq!(first.edge_debug.is_some(), second.edge_debug.is_some());
    }

    #[test]
    fn spur_penalty_distinguishes_dangling_and_crossing_lines() {
        let mut dangling = vec![vec![0u8; 8]; 8];
        for row in dangling.iter_mut().take(5) {
            row[3] = 255;
        }
        let mut crossing = vec![vec![0u8; 8]; 8];
        for row in &mut crossing {
            row[3] = 255;
        }
        assert!(bitmap_spur_penalty(&dangling) > 0.0);
        assert_eq!(bitmap_spur_penalty(&crossing), 0.0);
    }

    #[test]
    fn mode_two_edge_cell_uses_graphic_partial_fill() {
        let mut image = ImageBuffer::from_pixel(32, 8, Rgba([64, 91, 137, 255]));
        for y in 0..8 {
            for x in 20..32 {
                image.put_pixel(x, y, Rgba([235, 235, 225, 255]));
            }
        }
        let config = ConversionConfig {
            width: 4,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let result = convert_image(&DynamicImage::ImageRgba8(image), &config).unwrap();
        let edge_cell = result.grid.cells[2];
        assert_ne!(edge_cell.glyph, SPACE_GLYPH);
        assert_ne!(edge_cell.glyph, SOLID_GLYPH);
        assert!(glyph_allowed(2, edge_cell.glyph));
        assert_ne!(edge_cell.fg, edge_cell.bg);
    }
}
