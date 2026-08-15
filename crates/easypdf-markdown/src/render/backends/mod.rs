//! 渲染后端实现。

pub mod text_backend;

#[cfg(feature = "pdfium")]
pub mod pdfium_backend;
