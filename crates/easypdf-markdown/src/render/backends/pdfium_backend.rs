//! 基于 `PDFium` 的渲染后端。
//!
//! 使用 [`pdfium_render`] crate 提供高质量的 PDF 页面光栅化。
//! 运行时需要 `libpdfium` 动态库。
//!
//! 仅在启用 `pdfium` feature 时编译此模块。

use std::path::Path;

use pdfium_render::prelude::{PdfRenderConfig, Pdfium, PdfiumError};

use crate::render::config::RenderConfig;
use crate::render::error::{RenderError, Result};
use crate::render::traits::{PdfRenderer, RenderedImage};

/// 绑定 pdfium 动态库，先尝试 PDF 所在目录，再回退到系统库路径。
fn bind_pdfium(
    pdf_dir: &Path,
) -> std::result::Result<Box<dyn pdfium_render::prelude::PdfiumLibraryBindings>, PdfiumError> {
    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(pdf_dir))
        .or_else(|_| Pdfium::bind_to_system_library())
}

/// 基于 Google `PDFium` 的高质量 PDF 渲染器。
///
/// 需要 `pdfium` Cargo feature 且运行时需要 `libpdfium` 共享库。
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
    /// 探测 pdfium 动态库是否可加载。
    ///
    /// # Errors
    ///
    /// 当库无法找到或加载时返回 [`PdfiumError`]。
    pub fn probe() -> std::result::Result<(), PdfiumError> {
        bind_pdfium(Path::new("."))?;
        Ok(())
    }

    /// 使用 pdfium 后端打开 PDF 文件进行渲染。
    ///
    /// # Errors
    ///
    /// 当 pdfium 库无法加载时返回 [`RenderError::BackendUnavailable`]，
    /// 当 PDF 无法打开时返回 [`RenderError::Parse`]。
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

    /// 将 [`RenderConfig`] 的 DPI 转换为 A4 页面的目标像素宽度。
    fn target_width(config: &RenderConfig) -> i32 {
        // A4 在 72 DPI 下宽度为 595 点。
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

        // 每次调用时绑定：`Pdfium` 持有 `Box<dyn PdfiumLibraryBindings>`，
        // 既不是 `Send` 也不是 `Sync`，因此无法存储在此 (Send + Sync) 渲染器中。
        // 操作系统会缓存已加载的动态库，因此重复绑定在首次调用后代价很低。
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
        // pdfium 产生 BGRA；转换为 RGBA。
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
        // 此测试验证即使 pdfium 未安装，probe() 也不会 panic。
        // 根据环境可能成功或失败；仅验证返回 Result。
        let _result = PdfiumRenderer::probe();
    }

    #[test]
    fn test_open_nonexistent_returns_error() {
        // 尝试打开不存在的文件应优雅失败。
        let result = PdfiumRenderer::open(Path::new("/nonexistent/path/file.pdf"));
        assert!(result.is_err());
    }
}
