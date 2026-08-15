//! PDF 读取与文本提取（lopdf 后端）。
//!
//! 提供 [`PdfReader`] 用于解析 PDF 文档并提取文本、元数据和页面信息。

mod extract;
mod pdf_reader;

#[cfg(test)]
#[allow(clippy::items_after_statements, clippy::similar_names)]
mod tests;

pub use pdf_reader::PdfReader;
use pdf_reader::usize_to_u64_saturating;
