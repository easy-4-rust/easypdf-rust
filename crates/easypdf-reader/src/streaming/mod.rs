//! 流式 PDF 字节流扫描器。
//!
//! 扫描原始 PDF 字节中的 `stream...endstream` 边界，解压内容流，
//! 并在不构建完整 `lopdf::Document` 对象树的情况下提取文本算子。
//! 专为超大 PDF（>100 MB）或资源受限环境设计，避免完整的
//! xref/对象树解析开销。

mod byte_finder;
mod cmap;
pub(super) mod scanner;
mod stream_scan_result;
mod text_extract;

#[cfg(test)]
mod tests;

// 在 streaming 模块级别重导出公共类型，以便父模块（`lib.rs`）
// 可以使用 `streaming::StreamScanner` 和 `streaming::StreamScanResult`。
pub(super) use scanner::StreamScanner;
pub(super) use stream_scan_result::StreamScanResult;
