//! PDF 渲染后端选择。

use std::path::Path;

#[cfg(not(feature = "pdfium"))]
use super::error::RenderError;
use super::error::Result;
use super::traits::PdfRenderer;

/// 可用的渲染后端。
///
/// 使用 [`RenderBackend::build_renderer`] 从 PDF 文件路径构建具体的
/// [`PdfRenderer`]。使用 [`is_available`](Self::is_available) 检查
/// 后端的运行时依赖（如动态库）是否存在。
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::RenderBackend;
///
/// // 使用前检查 pdfium 是否可用：
/// if RenderBackend::Pdfium.is_available() {
///     let renderer = RenderBackend::Pdfium.build_renderer("doc.pdf".as_ref())?;
/// } else {
///     // 回退到文本渲染器：
///     let renderer = RenderBackend::Text.build_renderer("doc.pdf".as_ref())?;
/// }
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderBackend {
    /// Google `PDFium` 后端（最高质量）。
    ///
    /// 需要 `pdfium` feature 且运行时需要 `libpdfium` 动态库。
    Pdfium,

    /// 纯 Rust 文本回退后端。
    ///
    /// 通过 `easypdf-reader` 提取文本并渲染为简单的白底黑字图像。
    /// 质量较低但无需外部依赖。
    Text,
}

impl RenderBackend {
    /// 为给定 PDF 文件构建 [`PdfRenderer`]。
    ///
    /// # Errors
    ///
    /// 当后端运行时依赖缺失时返回 [`RenderError::BackendUnavailable`]，
    /// 当 PDF 无法打开时返回 [`RenderError::Io`] 或 [`RenderError::Parse`]。
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

    /// 检查此后端在当前环境中是否可用。
    ///
    /// `Text` 后端始终返回 `true`。`Pdfium` 后端检查 `pdfium` feature
    /// 是否启用且动态库是否可加载。
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

    /// 返回当前环境的默认后端。
    ///
    /// 优先使用 `Pdfium`（如可用），否则回退到 `Text`。
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
        // 未启用 pdfium feature 时，默认应为 Text。
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
