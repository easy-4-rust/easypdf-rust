//! PDF page rendering to raster images for `easypdf-rust`.
//!
//! This module provides a trait-based abstraction ([`PdfRenderer`]) for rendering
//! PDF pages to raster images (PNG, JPEG). Two backends are available:
//!
//! - **`TextRenderer`** (default, pure Rust) -- extracts text via
//!   [`easypdf_reader::PdfReader`] and renders it as a simple bitmap image.
//!   Quality is low but sufficient for OCR pipelines. No external dependencies.
//!
//! - **`PdfiumRenderer`** (feature `pdfium`) -- uses Google `PDFium` for
//!   high-quality rendering. Requires the `libpdfium` dynamic library at
//!   runtime.
//!
//! # Quick start
//!
//! ```no_run
//! use easypdf_markdown::render::{render_page_to_png, render_all_pages_to_dir};
//!
//! // Render page 0 at 150 DPI:
//! render_page_to_png("input.pdf".as_ref(), 0, "page_0.png".as_ref(), 150)?;
//!
//! // Render all pages to a directory:
//! let paths = render_all_pages_to_dir("input.pdf".as_ref(), "output/".as_ref(), 150)?;
//! # Ok::<(), easypdf_markdown::render::RenderError>(())
//! ```
//!
//! # Choosing a backend
//!
//! Use [`RenderBackend::default_backend`] to auto-select the best available
//! backend, or pick explicitly:
//!
//! ```no_run
//! use easypdf_markdown::render::RenderBackend;
//!
//! let renderer = RenderBackend::default_backend()
//!     .build_renderer("document.pdf".as_ref())?;
//! # Ok::<(), easypdf_markdown::render::RenderError>(())
//! ```

pub mod backend;
pub mod backends;
pub mod config;
pub mod error;
pub mod traits;

// --- Re-exports for convenience ---
pub use backend::RenderBackend;
pub use config::{Background, ImageFormat, RenderConfig};
pub use error::{RenderError, Result};
pub use traits::{PdfRenderer, RenderedImage};

use std::path::{Path, PathBuf};

/// Render a single PDF page to a PNG file.
///
/// Uses the default backend (text fallback) at the specified DPI.
///
/// # Errors
///
/// Returns [`RenderError`] if the PDF cannot be opened, the page index
/// is invalid, or the output file cannot be written.
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::render_page_to_png;
///
/// render_page_to_png("input.pdf".as_ref(), 0, "page_0.png".as_ref(), 150)?;
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
pub fn render_page_to_png(
    pdf_path: &Path,
    page_index: usize,
    output: &Path,
    dpi: u32,
) -> Result<()> {
    let config = RenderConfig {
        dpi,
        format: ImageFormat::Png,
        ..RenderConfig::default()
    };
    let renderer = RenderBackend::default_backend().build_renderer(pdf_path)?;
    renderer.render_page_to_path(page_index, &config, output)
}

/// Render all pages of a PDF to PNG files in a directory.
///
/// Output files are named `page_000.png`, `page_001.png`, etc.
/// The output directory is created if it does not exist.
///
/// # Errors
///
/// Returns [`RenderError`] if the PDF cannot be opened, a page fails
/// to render, or the output directory/files cannot be written.
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::render_all_pages_to_dir;
///
/// let paths = render_all_pages_to_dir("input.pdf".as_ref(), "output/".as_ref(), 150)?;
/// for p in &paths {
///     println!("rendered: {}", p.display());
/// }
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
pub fn render_all_pages_to_dir(
    pdf_path: &Path,
    output_dir: &Path,
    dpi: u32,
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(output_dir)?;

    let config = RenderConfig {
        dpi,
        format: ImageFormat::Png,
        ..RenderConfig::default()
    };
    let renderer = RenderBackend::default_backend().build_renderer(pdf_path)?;

    // Determine page count by probing indices until InvalidPage.
    let mut page_count = 0usize;
    loop {
        match renderer.render_page(page_count, &config) {
            Ok(_) => page_count += 1,
            Err(RenderError::InvalidPage { .. }) => break,
            Err(e) => return Err(e),
        }
    }

    // Re-render and save (the probe above consumed the images).
    let mut paths = Vec::with_capacity(page_count);
    for i in 0..page_count {
        let filename = format!("page_{i:03}.png");
        let path = output_dir.join(&filename);
        renderer.render_page_to_path(i, &config, &path)?;
        paths.push(path);
    }
    Ok(paths)
}

/// Render a single PDF page to an in-memory [`RenderedImage`].
///
/// Uses the default backend at the specified DPI.
///
/// # Errors
///
/// Returns [`RenderError`] if the PDF cannot be opened or the page
/// index is invalid.
pub fn render_page(
    pdf_path: &Path,
    page_index: usize,
    config: &RenderConfig,
) -> Result<RenderedImage> {
    let renderer = RenderBackend::default_backend().build_renderer(pdf_path)?;
    renderer.render_page(page_index, config)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names, clippy::float_cmp)]
    use super::*;

    /// Helper: build a minimal valid PDF in memory.
    fn make_test_pdf_bytes() -> Vec<u8> {
        let mut doc = lopdf::Document::new();
        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf 72 700 Td (Hello) Tj ET".to_vec(),
        )));
        let mut font_dict = lopdf::Dictionary::new();
        font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
        let font_id = doc.add_object(lopdf::Object::Dictionary(font_dict));
        let mut resources = lopdf::Dictionary::new();
        let mut fonts = lopdf::Dictionary::new();
        fonts.set("F1", lopdf::Object::Reference(font_id));
        resources.set("Font", lopdf::Object::Dictionary(fonts));
        let resources_id = doc.add_object(lopdf::Object::Dictionary(resources));
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        page_dict.set("Resources", lopdf::Object::Reference(resources_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));
        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages_dict.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages_dict.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_render_page_to_png_convenience() {
        let bytes = make_test_pdf_bytes();
        let dir = std::env::temp_dir().join("easypdf_render_lib_test");
        let _ = std::fs::remove_dir_all(&dir);
        // Write the PDF to a temp file (convenience API takes a path).
        let pdf_path = dir.join("test.pdf");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&pdf_path, &bytes).unwrap();

        let output = dir.join("page_0.png");
        render_page_to_png(&pdf_path, 0, &output, 72).unwrap();
        assert!(output.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_render_all_pages_to_dir() {
        let bytes = make_test_pdf_bytes();
        let dir = std::env::temp_dir().join("easypdf_render_all_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pdf_path = dir.join("test.pdf");
        std::fs::write(&pdf_path, &bytes).unwrap();

        let out_dir = dir.join("output");
        let paths = render_all_pages_to_dir(&pdf_path, &out_dir, 72).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_render_page_in_memory() {
        let bytes = make_test_pdf_bytes();
        let dir = std::env::temp_dir().join("easypdf_render_mem_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pdf_path = dir.join("test.pdf");
        std::fs::write(&pdf_path, &bytes).unwrap();

        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = render_page(&pdf_path, 0, &config).unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_default_backend_is_text() {
        let backend = RenderBackend::default_backend();
        // Without pdfium feature, default should be Text.
        assert_eq!(backend, RenderBackend::Text);
    }

    #[test]
    fn test_backend_display() {
        assert_eq!(RenderBackend::Text.to_string(), "text");
        assert_eq!(RenderBackend::Pdfium.to_string(), "pdfium");
    }

    #[test]
    fn test_text_backend_is_available() {
        assert!(RenderBackend::Text.is_available());
    }

    #[test]
    fn test_pdfium_backend_not_available_without_feature() {
        // Without the pdfium feature, Pdfium should not be available.
        #[cfg(not(feature = "pdfium"))]
        assert!(!RenderBackend::Pdfium.is_available());
    }
}
