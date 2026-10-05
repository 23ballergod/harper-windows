//! Offline AI grammar suggestions for Harper.
//!
//! Harper's rule engine catches most mistakes instantly. This crate adds a second, deeper pass that
//! runs a small open-weight language model entirely on the user's machine (no API key, no network
//! after the one-time model download) and turns its corrections into ordinary Harper lints.

mod checker;
mod diff;
mod engine;
mod models;
mod qwen2;
mod sentences;

pub use checker::{AiChecker, CheckerStatus};
pub use diff::{AI_RULE_NAME, Edit, compute_edits, edit_to_lint};
pub use engine::{Corrector, QwenCorrector};
pub use models::{AiModel, ModelFile};
pub use sentences::{Sentence, split_sentences};
