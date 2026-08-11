//! Heuristic table detection for PDF-to-Markdown conversion.
//!
//! This module provides [`TableDetectorProcessor`], an implementation of
//! [`PdfMarkdownProcessor`](crate::PdfMarkdownProcessor) that
//! scans paragraph blocks for table patterns and replaces them with
//! [`PdfBlock::Table`](easypdf_core::PdfBlock::Table) blocks.
//!
//! # Supported patterns
//!
//! | Pattern | Example | Separator |
//! |---------|---------|-----------|
//! | Pipe | `\| Name \| Age \|` | `\|` character |
//! | Tab | `Name\tAge` | Tab character |
//! | Whitespace | `Name    Age    City` | 2+ consecutive spaces |
//!
//! # Quick start
//!
//! ```
//! use easypdf_markdown::{ProcessorPipeline, PdfMarkdownProcessor};
//! use easypdf_markdown::table::TableDetectorProcessor;
//!
//! let mut pipeline = ProcessorPipeline::new();
//! pipeline.register(Box::new(TableDetectorProcessor::new()));
//! ```
//!
//! # Configuration
//!
//! Use [`TableDetectionConfig`](crate::table::TableDetectionConfig) to tune:
//! - Minimum row/column counts
//! - Column separator strategy (Pipe, Tab, Whitespace, or Auto)
//! - Whether to allow irregular column counts

pub mod config;
mod detector;
mod heuristic;
mod parser;
#[cfg(test)]
mod tests;

pub use config::{ColumnSeparator, TableDetectionConfig};
pub use detector::TableDetectorProcessor;
