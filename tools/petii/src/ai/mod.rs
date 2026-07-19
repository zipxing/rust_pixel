//! Schema-validated AI boundaries. Provider-produced content cannot mutate a
//! PETSCII grid until these types pass validation.

pub mod artifacts;
pub mod loop_runner;
pub mod provider;
pub mod schema;

pub use artifacts::{CandidateArtifact, RunManifest};
pub use loop_runner::{run_with_reference, AiLoopBudget, AiLoopCandidate, AiLoopResult};
pub use provider::{OpenAiCompatibleProvider, ReferenceGenerator};
pub use schema::ArtPlan;
