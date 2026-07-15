//! Reusable PETSCII conversion and deterministic quality-improvement pipeline.

pub mod ai;
pub mod c64;
pub mod converter;
pub mod optimizer;
pub mod preview;
pub mod types;

pub use converter::{convert_image, generate_config_variants, ConversionResult};
pub use optimizer::{optimize_grid, score_grid, OptimizationWeights, ScoreBreakdown};
pub use preview::render_grid;
pub use types::{ConversionConfig, GlyphCandidate, PetsciiCell, PetsciiGrid};
