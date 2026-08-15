//! OCR processor implementing `PdfMarkdownProcessor`.

mod core;
mod renderer;
#[cfg(test)]
mod tests;

pub use core::OcrProcessor;
