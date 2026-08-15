//! Backend selection for PDF rendering.

use std::path::Path;

#[cfg(not(feature = "pdfium"))]
use super::error::RenderError;
use super::error::Result;
use super::traits::PdfRenderer;

/// Available rendering backends.
///
/// Use [`RenderBackend::build_renderer`] to construct a concrete
/// [`PdfRenderer`] from a PDF file path. Use
/// [`is_available`](Self::is_available) to check whether a backend's
/// runtime dependencies (e.g., dynamic libraries) are present.
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::RenderBackend;
///
/// // Check if pdfium is available before using it:
/// if RenderBackend::Pdfium.is_available() {
///     let renderer = RenderBackend::Pdfium.build_renderer("doc.pdf".as_ref())?;
/// } else {
///     // Fall back to text renderer:
///     let renderer = RenderBackend::Text.build_renderer("doc.pdf".as_ref())?;
/// }
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderBackend {
    /// Google `PDFium` backend (highest quality).
    ///
    /// Requires the `pdfium` feature and the `libpdfium` dynamic library
    /// to be available at runtime.
    Pdfium,

    /// Pure-Rust text fallback backend.
    ///
    /// Extracts text via `easypdf-reader` and renders it as a simple
    /// white-background, black-text image. Quality is low but no
    /// external dependencies are needed.
    Text,
}

impl RenderBackend {
    /// Construct a [`PdfRenderer`] for the given PDF file.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::BackendUnavailable`] if the backend's runtime
    /// dependencies are missing, or [`RenderError::Io`] /
    /// [`RenderError::Parse`] if the PDF cannot be opened.
    pub fn build_renderer(&self, pdf_path: &Path) -> Result<Box<dyn PdfRenderer>> {
        match self {
            Self::Text => {
                let renderer = super::backends::text_backend::TextRenderer::open(pdf_path)?;
                Ok(Box::new(renderer))
            }
            #[cfg(feature = "pdfium")]
            Self::Pdfium => {
                let renderer = super::backends::pdfium_backend::PdfiumRenderer::open(pdf_path)?;
                Ok(Box::new(renderer))
            }
            #[cfg(not(feature = "pdfium"))]
            Self::Pdfium => Err(RenderError::BackendUnavailable {
                name: "pdfium",
                reason: "the 'pdfium' feature is not enabled".to_owned(),
            }),
        }
    }

    /// Check whether this backend is available in the current environment.
    ///
    /// For the `Text` backend this always returns `true`. For `Pdfium` it
    /// checks whether the `pdfium` feature is enabled and the dynamic
    /// library can be loaded.
    #[must_use]
    pub fn is_available(&self) -> bool {
        match self {
            Self::Text => true,
            #[cfg(feature = "pdfium")]
            Self::Pdfium => super::backends::pdfium_backend::PdfiumRenderer::probe().is_ok(),
            #[cfg(not(feature = "pdfium"))]
            Self::Pdfium => false,
        }
    }

    /// Return the default backend for the current environment.
    ///
    /// Prefers `Pdfium` if available, otherwise falls back to `Text`.
    #[must_use]
    pub fn default_backend() -> Self {
        if Self::Pdfium.is_available() {
            Self::Pdfium
        } else {
            Self::Text
        }
    }
}

impl std::fmt::Display for RenderBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdfium => f.write_str("pdfium"),
            Self::Text => f.write_str("text"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_backend_is_available() {
        assert!(RenderBackend::Text.is_available());
    }

    #[test]
    fn pdfium_backend_not_available_without_feature() {
        assert!(!RenderBackend::Pdfium.is_available());
    }

    #[test]
    fn default_backend_is_text() {
        // Without pdfium feature, default should be Text
        let backend = RenderBackend::default_backend();
        assert_eq!(backend, RenderBackend::Text);
    }

    #[test]
    fn display_text() {
        assert_eq!(RenderBackend::Text.to_string(), "text");
    }

    #[test]
    fn display_pdfium() {
        assert_eq!(RenderBackend::Pdfium.to_string(), "pdfium");
    }

    #[test]
    fn pdfium_build_renderer_returns_error() {
        let result = RenderBackend::Pdfium.build_renderer("/nonexistent.pdf".as_ref());
        assert!(result.is_err());
    }

    #[test]
    fn text_build_renderer_with_invalid_path() {
        let result = RenderBackend::Text.build_renderer("/nonexistent.pdf".as_ref());
        assert!(result.is_err());
    }

    #[test]
    fn clone_copy() {
        let a = RenderBackend::Text;
        let b = a;
        let c = a;
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn debug_format() {
        assert!(format!("{:?}", RenderBackend::Text).contains("Text"));
        assert!(format!("{:?}", RenderBackend::Pdfium).contains("Pdfium"));
    }
}
