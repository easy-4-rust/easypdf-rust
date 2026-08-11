//! Rendering backend implementations.

pub mod text_backend;

#[cfg(feature = "pdfium")]
pub mod pdfium_backend;
