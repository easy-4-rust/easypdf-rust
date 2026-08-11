//! OCR engine implementations.
//!
//! Provides a mock engine by default, with optional feature-gated backends:
//!
//! - [`MockOcrEngine`] -- returns fixed text (always available, for testing)
//! - `OcrsEngine` -- pure Rust OCR via `ocrs` crate (feature `ocrs`)
//! - `LlmOcrEngine` -- LLM Vision API via `rig-core` (feature `llm`)
//! - `DeepSeekOcrEngine` -- DeepSeek-OCR-2 via `rig-core` (feature `llm`)

mod mock;

pub use mock::MockOcrEngine;

#[cfg(feature = "ocrs")]
mod ocrs_backend;

#[cfg(feature = "ocrs")]
pub use ocrs_backend::OcrsEngine;

#[cfg(feature = "llm")]
mod llm_backend;

#[cfg(feature = "llm")]
pub use llm_backend::{DeepSeekOcrEngine, LlmOcrEngine};
