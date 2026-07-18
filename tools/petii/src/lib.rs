//! Reusable PETSCII conversion and deterministic quality-improvement pipeline.

pub mod ai;
pub mod benchmark;
pub mod c64;
pub mod contour;
pub mod converter;
pub mod corpus;
pub mod edge_debug;
pub mod glyph_topology;
pub mod optimizer;
pub mod preview;
pub mod types;

pub use benchmark::{
    run_benchmark, run_dither_eval, BenchmarkCase, BenchmarkCaseReport, BenchmarkOptions,
    BenchmarkReport, BenchmarkSuite, BenchmarkSummary, BenchmarkWinner, DitherEvalRow,
};
pub use converter::{
    convert_image, convert_image_dithered, convert_image_dithered_prior, convert_image_top1,
    generate_config_variants, ConversionResult, EdgeDebugData, EdgeGateDecision, EdgeGrammarMetrics,
    EdgeGrammarReport,
};
pub use corpus::{analyze_pix_corpus, CorpusPrior, CorpusReport, NaturalnessScore};
pub use edge_debug::render_edge_debug;
pub use optimizer::{
    optimize_grid, perceptual_tone_distance, perceptual_tone_score, score_grid,
    OptimizationWeights, ScoreBreakdown,
};
pub use preview::render_grid;
pub use types::{ConversionConfig, GlyphCandidate, PetsciiCell, PetsciiGrid};
