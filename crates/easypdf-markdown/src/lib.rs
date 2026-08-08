//! 将 PDF 语义内容确定性转换为 Markdown。

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

mod image_policy;
mod markdown_export_report;
mod markdown_export_result;
mod markdown_profile;
mod markdown_renderer;
mod markdown_warning;
mod ocr_policy;
mod pdf_markdown_export_builder;
mod table_policy;

pub use image_policy::ImagePolicy;
pub use markdown_export_report::MarkdownExportReport;
pub use markdown_export_result::MarkdownExportResult;
pub use markdown_profile::MarkdownProfile;
pub use markdown_renderer::MarkdownRenderer;
pub use markdown_warning::MarkdownWarning;
pub use ocr_policy::OcrPolicy;
pub use pdf_markdown_export_builder::PdfMarkdownExportBuilder;
pub use table_policy::TablePolicy;
