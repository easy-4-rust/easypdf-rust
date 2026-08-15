//! PDFium-based rendering backend.
//!
//! Uses the [`pdfium_render`] crate to provide high-quality PDF page
//! rasterization. Requires the `libpdfium` dynamic library to be available
//! at runtime.
//!
//! This module is only compiled when the `pdfium` feature is enabled.

use std::path::Path;

use pdfium_render::prelude::{PdfRenderConfig, Pdfium, PdfiumError};

use crate::render::config::RenderConfig;
use crate::render::error::{RenderError, Result};
use crate::render::traits::{PdfRenderer, RenderedImage};

/// Bind to the pdfium dynamic library, trying the directory of the PDF first,
/// then falling back to system library paths.
fn bind_pdfium(
    pdf_dir: &Path,
) -> std::result::Result<Box<dyn pdfium_render::prelude::PdfiumLibraryBindings>, PdfiumError> {
    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(pdf_dir))
        .or_else(|_| Pdfium::bind_to_system_library())
}

/// High-quality PDF renderer backed by Google `PDFium`.
///
/// Requires the `pdfium` Cargo feature and the `libpdfium` shared library
/// to be present at runtime.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// use easypdf_markdown::render::backends::pdfium_backend::PdfiumRenderer;
/// use easypdf_markdown::render::{PdfRenderer, RenderConfig};
///
/// let renderer = PdfiumRenderer::open(Path::new("document.pdf"))?;
/// let image = renderer.render_page(0, &RenderConfig::default())?;
/// image.save("page_0.png".as_ref())?;
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
pub struct PdfiumRenderer {
    document_path: std::path::PathBuf,
    page_count: usize,
}

impl PdfiumRenderer {
    /// Probe whether the pdfium dynamic library can be loaded.
    ///
    /// # Errors
    ///
    /// Returns a [`PdfiumError`] if the library cannot be found or loaded.
    pub fn probe() -> std::result::Result<(), PdfiumError> {
        bind_pdfium(Path::new("."))?;
        Ok(())
    }

    /// Open a PDF file for rendering with the pdfium backend.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::BackendUnavailable`] if the pdfium library
    /// cannot be loaded, or [`RenderError::Parse`] if the PDF cannot be opened.
    pub fn open(path: &Path) -> Result<Self> {
        let pdfium_bind = bind_pdfium(path.parent().unwrap_or(Path::new("."))).map_err(|e| {
            RenderError::BackendUnavailable {
                name: "pdfium",
                reason: e.to_string(),
            }
        })?;

        let pdfium = Pdfium::new(pdfium_bind);

        let document = pdfium
            .load_pdf_from_file(path.to_str().unwrap_or(""), None)
            .map_err(|e| RenderError::Parse(e.to_string()))?;

        let page_count = document.pages().len();

        Ok(Self {
            document_path: path.to_path_buf(),
            page_count: usize::from(page_count),
        })
    }

    /// Convert a [`RenderConfig`] DPI to a target pixel width for an A4 page.
    fn target_width(config: &RenderConfig) -> i32 {
        // A4 width at 72 DPI is 595 points.
        let scale = f64::from(config.dpi) / 72.0;
        #[allow(clippy::cast_possible_truncation)] // A4 宽度有限，round 后不会截断
        let w = (595.0 * scale).round() as i32;
        if let Some(max_w) = config.max_width {
            w.min(i32::try_from(max_w).unwrap_or(i32::MAX))
        } else {
            w
        }
    }
}

impl PdfRenderer for PdfiumRenderer {
    fn render_page(&self, page_index: usize, config: &RenderConfig) -> Result<RenderedImage> {
        if page_index >= self.page_count {
            return Err(RenderError::InvalidPage {
                index: page_index,
                total: self.page_count,
            });
        }

        // Bind per call: `Pdfium` holds `Box<dyn PdfiumLibraryBindings>` which is
        // neither `Send` nor `Sync`, so it cannot be stored in this (Send + Sync)
        // renderer. The OS caches the already-loaded dynamic library, so repeated
        // binding is cheap after the first call.
        let pdfium_bind = bind_pdfium(self.document_path.parent().unwrap_or(Path::new(".")))
            .map_err(|e| RenderError::BackendUnavailable {
                name: "pdfium",
                reason: e.to_string(),
            })?;
        let pdfium = Pdfium::new(pdfium_bind);

        let document = pdfium
            .load_pdf_from_file(self.document_path.to_str().unwrap_or(""), None)
            .map_err(|e| RenderError::Parse(e.to_string()))?;

        let page = document
            .pages()
            .get(u16::try_from(page_index).unwrap_or(u16::MAX))
            .map_err(|e| RenderError::Parse(e.to_string()))?;

        let target_width = Self::target_width(config);
        let max_height = config
            .max_height
            .map_or(i32::MAX, |h| i32::try_from(h).unwrap_or(i32::MAX));

        let render_config = PdfRenderConfig::new()
            .set_target_width(target_width)
            .set_maximum_height(max_height);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| RenderError::Pdfium(e.to_string()))?;

        let width = bitmap.width().cast_unsigned();
        let height = bitmap.height().cast_unsigned();
        let raw = bitmap.as_raw_bytes();
        // pdfium produces BGRA; convert to RGBA.
        let mut rgba = Vec::with_capacity(raw.len());
        for chunk in raw.chunks_exact(4) {
            rgba.push(chunk[2]); // R
            rgba.push(chunk[1]); // G
            rgba.push(chunk[0]); // B
            rgba.push(chunk[3]); // A
        }

        Ok(RenderedImage::new(
            width,
            height,
            config.format,
            rgba,
            page_index,
        ))
    }

    fn name(&self) -> &'static str {
        "pdfium"
    }

    fn max_dpi(&self) -> u32 {
        2400
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_reports_unavailable_when_no_library() {
        // This test verifies that probe() does not panic even when
        // pdfium is not installed. It may succeed or fail depending
        // on the environment; we just verify it returns a Result.
        let _result = PdfiumRenderer::probe();
    }

    #[test]
    fn test_open_nonexistent_returns_error() {
        // Attempting to open a nonexistent file should fail gracefully.
        let result = PdfiumRenderer::open(Path::new("/nonexistent/path/file.pdf"));
        assert!(result.is_err());
    }
}
