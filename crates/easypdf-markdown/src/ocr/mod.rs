//! `easypdf-markdown` 的 OCR 处理器：扫描件 PDF 文本提取。
//!
//! 本模块提供 [`OcrProcessor`]，它实现了
//! [`PdfMarkdownProcessor`](crate::PdfMarkdownProcessor)，通过 OCR 从
//! 图片密集型或扫描件 PDF 页面中提取文本。它被设计为 Markdown 处理器管道中
//! 的最后回退手段。
//!
//! # 架构
//!
//! ```text
//! PdfInput -> PdfRenderer -> 页面图像 -> OcrEngine -> 文本 -> PdfBlock::Paragraph
//! ```
//!
//! [`OcrEngine`] trait 抽象了不同的 OCR 后端：
//!
//! - [`MockOcrEngine`](crate::ocr::engines::MockOcrEngine) -- 返回固定文本（默认，用于测试）
//! - `ocrs` feature -- 通过 [`ocrs`](https://crates.io/crates/ocrs) crate 的纯 Rust OCR
//! - `llm` feature -- 通过 [`rig-core`](https://crates.io/crates/rig-core) 的 LLM Vision API
//!
//! # 快速开始
//!
//! ```
//! use easypdf_markdown::ocr::{
//!     OcrProcessor, OcrConfig, OcrTrigger,
//!     engines::MockOcrEngine,
//! };
//!
//! let processor = OcrProcessor::with_mock_engine();
//! // 或使用自定义配置：
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
