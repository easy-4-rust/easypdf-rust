use crate::render::{RenderConfig, RenderError};

use easypdf_core::PdfError;

/// 委托给借用的 `PdfRenderer` 的适配器。
pub(super) struct StoredRendererAdapter<'a>(pub &'a dyn crate::render::PdfRenderer);

impl crate::render::PdfRenderer for StoredRendererAdapter<'_> {
    fn render_page(
        &self,
        page_index: usize,
        config: &RenderConfig,
    ) -> crate::render::Result<crate::render::RenderedImage> {
        self.0.render_page(page_index, config)
    }

    fn render_page_to_path(
        &self,
        page_index: usize,
        config: &RenderConfig,
        output: &std::path::Path,
    ) -> crate::render::Result<()> {
        self.0.render_page_to_path(page_index, config, output)
    }

    fn render_pages(
        &self,
        page_range: std::ops::Range<usize>,
        config: &RenderConfig,
    ) -> crate::render::Result<Vec<crate::render::RenderedImage>> {
        self.0.render_pages(page_range, config)
    }

    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn max_dpi(&self) -> u32 {
        self.0.max_dpi()
    }

    fn supports_vector(&self) -> bool {
        self.0.supports_vector()
    }
}

/// 将 `RenderError` 转换为 `PdfError`。
pub(super) fn render_error_to_pdf(e: &RenderError) -> PdfError {
    PdfError::Other(format!("render error: {e}"))
}

/// 构建用于测试的 `RenderedImage`（无需真实 PDF）。
///
/// 创建给定尺寸的白色 RGBA 图像。
#[cfg(test)]
pub(crate) fn make_test_rendered_image(
    width: u32,
    height: u32,
    page_index: usize,
) -> crate::render::RenderedImage {
    let pixels = vec![255u8; (width * height * 4) as usize];
    crate::render::RenderedImage::new(
        width,
        height,
        crate::render::ImageFormat::Png,
        pixels,
        page_index,
    )
}
