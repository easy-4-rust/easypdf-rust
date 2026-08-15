//! 用于 PDF 转 Markdown 的启发式表格检测。
//!
//! 本模块提供 [`TableDetectorProcessor`]，它是
//! [`PdfMarkdownProcessor`](crate::PdfMarkdownProcessor) 的一个实现，
//! 扫描段落块中的表格模式并将其替换为
//! [`PdfBlock::Table`](easypdf_core::PdfBlock::Table) 块。
//!
//! # 支持的模式
//!
//! | 模式 | 示例 | 分隔符 |
//! |------|------|--------|
//! | 管道 | `\| Name \| Age \|` | `\|` 字符 |
//! | 制表符 | `Name\tAge` | 制表符 |
//! | 空格 | `Name    Age    City` | 2 个以上连续空格 |
//!
//! # 快速开始
//!
//! ```
//! use easypdf_markdown::{ProcessorPipeline, PdfMarkdownProcessor};
//! use easypdf_markdown::table::TableDetectorProcessor;
//!
//! let mut pipeline = ProcessorPipeline::new();
//! pipeline.register(Box::new(TableDetectorProcessor::new()));
//! ```
//!
//! # 配置
//!
//! 使用 [`TableDetectionConfig`](crate::table::TableDetectionConfig) 调整：
//! - 最小行/列数
//! - 列分隔策略（Pipe、Tab、Whitespace 或 Auto）
//! - 是否允许不规则列数

pub mod config;
mod detector;
mod heuristic;
mod parser;
#[cfg(test)]
mod tests;

pub use config::{ColumnSeparator, TableDetectionConfig};
pub use detector::TableDetectorProcessor;
