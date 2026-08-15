//! 实现 `PdfMarkdownProcessor` 的 OCR 处理器。

mod core;
mod renderer;
#[cfg(test)]
mod tests;

pub use core::OcrProcessor;
