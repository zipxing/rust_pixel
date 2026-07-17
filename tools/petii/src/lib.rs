//! Reusable PETSCII conversion and deterministic quality-improvement pipeline.

pub mod ai;
pub mod c64;
pub mod contour;
pub mod converter;
pub mod corpus;
pub mod edge_debug;
pub mod glyph_topology;
pub mod optimizer;
pub mod preview;
pub mod types;

pub use converter::{
    convert_image, generate_config_variants, ConversionResult, EdgeDebugData, EdgeGateDecision,
    EdgeGrammarMetrics, EdgeGrammarReport,
};
pub use corpus::{analyze_pix_corpus, CorpusReport};
pub use edge_debug::render_edge_debug;
pub use optimizer::{optimize_grid, score_grid, OptimizationWeights, ScoreBreakdown};
pub use preview::render_grid;
pub use types::{ConversionConfig, GlyphCandidate, PetsciiCell, PetsciiGrid};
