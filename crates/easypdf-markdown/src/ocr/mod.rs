//! OCR processor for `easypdf-markdown`: scanned PDF text extraction.
//!
//! This module provides an [`OcrProcessor`] that implements
//! [`PdfMarkdownProcessor`](crate::PdfMarkdownProcessor) to extract
//! text from image-heavy or scanned PDF pages via OCR. It is designed as the
//! last-resort fallback in the markdown processor pipeline.
//!
//! # Architecture
//!
//! ```text
//! PdfInput -> PdfRenderer -> page image -> OcrEngine -> text -> PdfBlock::Paragraph
//! ```
//!
//! The [`OcrEngine`] trait abstracts over different OCR backends:
//!
//! - [`MockOcrEngine`](crate::ocr::engines::MockOcrEngine) -- returns fixed text (default, for testing)
//! - `ocrs` feature -- pure Rust OCR via the [`ocrs`](https://crates.io/crates/ocrs) crate
//! - `llm` feature -- LLM Vision API via [`rig-core`](https://crates.io/crates/rig-core)
//!
//! # Quick start
//!
//! ```
//! use easypdf_markdown::ocr::{
//!     OcrProcessor, OcrConfig, OcrTrigger,
//!     engines::MockOcrEngine,
//! };
//!
//! let processor = OcrProcessor::with_mock_engine();
//! // or with custom config:
//! let processor = OcrProcessor::with_mock_engine()
//!     .with_config(OcrConfig {
//!         trigger: OcrTrigger::Always,
//!         ..OcrConfig::default()
//!     });
//! ```

pub mod config;
pub mod engine;
pub mod engines;
pub mod processor;

// Re-exports for convenience.
pub use config::{OcrConfig, OcrTrigger};
pub use engine::{OcrEngine, OcrImage, OcrResult, WordBox};
pub use processor::OcrProcessor;
