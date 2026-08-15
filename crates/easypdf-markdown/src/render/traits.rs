//! 核心渲染 trait 与输出类型。

use std::ops::Range;
use std::path::Path;

use super::config::{ImageFormat, RenderConfig};
use super::error::Result;

/// 已渲染的页面图像，包含原始像素数据。
///
/// 包含 RGBA 像素字节及尺寸和元数据。使用 [`save`](Self::save) 写入磁盘，
/// 或使用 [`to_dynamic_image`](Self::to_dynamic_image) 转换为
/// `image::DynamicImage` 以进行后续处理。
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::{RenderConfig, RenderBackend};
///
/// let renderer = RenderBackend::Text.build_renderer("document.pdf".as_ref())?;
/// let rendered = renderer.render_page(0, &RenderConfig::default())?;
/// rendered.save("page_0.png".as_ref())?;
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
#[derive(Debug, Clone)]
pub struct RenderedImage {
    /// 宽度（像素）。
    pub width: u32,
    /// 高度（像素）。
    pub height: u32,
    /// 编码使用的格式。
    pub format: ImageFormat,
    /// 原始 RGBA 像素字节（每像素 4 字节，行优先，从上到下）。
    pub pixels: Vec<u8>,
    /// 渲染页面的从 0 开始的页码索引。
    pub page_index: usize,
}

impl RenderedImage {
    /// 从原始 RGBA 像素创建新的 `RenderedImage`。
    #[must_use]
    pub fn new(
        width: u32,
        height: u32,
        format: ImageFormat,
        pixels: Vec<u8>,
        page_index: usize,
    ) -> Self {
        Self {
            width,
            height,
            format,
            pixels,
            page_index,
        }
    }

    /// 将渲染图像保存到文件。
    ///
    /// 输出格式由文件扩展名（`.png`、`.jpg`、`.jpeg`）决定，
    /// 或回退到此图像中存储的格式。
    ///
    /// # Errors
    ///
    /// 当文件无法写入时返回
    /// [`RenderError::Io`](super::RenderError::Io)，
    /// 当编码失败时返回
    /// [`RenderError::ImageEncode`](super::RenderError::ImageEncode)。
    pub fn save(&self, path: &Path) -> Result<()> {
        let dynamic = self.to_dynamic_image();
        dynamic
            .save(path)
            .map_err(|e| super::RenderError::ImageEncode(e.to_string()))?;
        Ok(())
    }

    /// 转换为 `image::DynamicImage`。
    ///
    /// 返回基于存储像素缓冲区的 `Rgba8` 图像。
    ///
    /// # Panics
    ///
    /// 当像素缓冲区长度不等于 `width * height * 4` 时 panic。
    #[must_use]
    pub fn to_dynamic_image(&self) -> image::DynamicImage {
        let img = image::RgbaImage::from_raw(self.width, self.height, self.pixels.clone())
            .expect("pixel buffer size must match width * height * 4");
        image::DynamicImage::ImageRgba8(img)
    }

    /// 将图像编码为 PNG 字节。
    ///
    /// # Errors
    ///
    /// 当编码失败时返回
    /// [`RenderError::ImageEncode`](super::RenderError::ImageEncode)。
    pub fn to_png_bytes(&self) -> Result<Vec<u8>> {
        let dynamic = self.to_dynamic_image();
        let mut buf = Vec::new();
        dynamic
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| super::RenderError::ImageEncode(e.to_string()))?;
        Ok(buf)
    }
}

/// PDF 页面渲染器抽象。
///
/// 实现者使用特定后端（如 pdfium、文本回退）提供页面到光栅图像的转换。
/// 该 trait 是对象安全的，且要求 `Send + Sync` 以便跨线程使用。
///
/// # 实现自定义后端
///
/// ```
/// use easypdf_markdown::render::{PdfRenderer, RenderConfig, RenderedImage, ImageFormat, Background};
/// use easypdf_markdown::render::error::Result;
/// use std::ops::Range;
/// use std::path::Path;
///
/// struct MyRenderer;
///
/// impl PdfRenderer for MyRenderer {
///     fn render_page(&self, page_index: usize, config: &RenderConfig) -> Result<RenderedImage> {
///         let (w, h) = page_dimensions(config.dpi);
///         let pixels = vec![255u8; (w * h * 4) as usize];
///         Ok(RenderedImage::new(w, h, config.format, pixels, page_index))
///     }
///
///     fn render_page_to_path(&self, page_index: usize, config: &RenderConfig, output: &Path) -> Result<()> {
///         self.render_page(page_index, config)?.save(output)
///     }
///
///     fn render_pages(&self, range: Range<usize>, config: &RenderConfig) -> Result<Vec<RenderedImage>> {
///         range.map(|i| self.render_page(i, config)).collect()
///     }
///
///     fn name(&self) -> &'static str { "my-custom" }
/// }
///
/// fn page_dimensions(dpi: u32) -> (u32, u32) {
///     let scale = f64::from(dpi) / 72.0;
///     ((595.0 * scale) as u32, (842.0 * scale) as u32)
/// }
/// ```
pub trait PdfRenderer: Send + Sync {
    /// 将单页渲染为 RGBA 像素缓冲区。
    ///
    /// # Errors
    ///
    /// 当页码无效、DPI 超过后端最大值或渲染失败时返回
    /// [`RenderError`](super::RenderError)。
    fn render_page(&self, page_index: usize, config: &RenderConfig) -> Result<RenderedImage>;

    /// 渲染单页并直接保存到文件路径。
    ///
    /// # Errors
    ///
    /// 当渲染或文件写入失败时返回 [`RenderError`](super::RenderError)。
    fn render_page_to_path(
        &self,
        page_index: usize,
        config: &RenderConfig,
        output: &Path,
    ) -> Result<()> {
        self.render_page(page_index, config)?.save(output)
    }

    /// 渲染一个范围的页面。
    ///
    /// # Errors
    ///
    /// 当范围内任一页面渲染失败时返回 [`RenderError`](super::RenderError)。
    fn render_pages(
        &self,
        page_range: Range<usize>,
        config: &RenderConfig,
    ) -> Result<Vec<RenderedImage>> {
        page_range.map(|i| self.render_page(i, config)).collect()
    }

    /// 此渲染后端的可读名称。
    fn name(&self) -> &'static str;

    /// 此后端支持的最大 DPI。默认值：600。
    fn max_dpi(&self) -> u32 {
        600
    }

    /// 此后端是否支持矢量（SVG）输出。默认值：`false`。
    fn supports_vector(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::super::config::{ImageFormat, RenderConfig};
    use super::*;

    #[test]
    fn rendered_image_new() {
        let pixels = vec![255u8; 4 * 2 * 2]; // 2x2 RGBA
        let img = RenderedImage::new(2, 2, ImageFormat::Png, pixels.clone(), 0);
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 2);
        assert_eq!(img.pixels, pixels);
        assert_eq!(img.page_index, 0);
    }

    #[test]
    fn rendered_image_clone() {
        let pixels = vec![0u8; 4];
        let img = RenderedImage::new(1, 1, ImageFormat::Png, pixels, 5);
        let cloned = img.clone();
        assert_eq!(img.width, cloned.width);
        assert_eq!(img.height, cloned.height);
        assert_eq!(img.page_index, cloned.page_index);
    }

    #[test]
    fn rendered_image_debug() {
        let pixels = vec![0u8; 4];
        let img = RenderedImage::new(1, 1, ImageFormat::Png, pixels, 0);
        let dbg = format!("{:?}", img);
        assert!(dbg.contains("RenderedImage"));
    }

    #[test]
    fn rendered_image_to_dynamic_image() {
        let pixels = vec![128u8; 4 * 2 * 2];
        let img = RenderedImage::new(2, 2, ImageFormat::Png, pixels, 0);
        let dynamic = img.to_dynamic_image();
        assert_eq!(dynamic.width(), 2);
        assert_eq!(dynamic.height(), 2);
    }

    #[test]
    fn rendered_image_to_png_bytes() {
        let pixels = vec![255u8; 4 * 2 * 2];
        let img = RenderedImage::new(2, 2, ImageFormat::Png, pixels, 0);
        let png = img.to_png_bytes().unwrap();
        assert!(!png.is_empty());
        // PNG 魔术字节
        assert_eq!(&png[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn rendered_image_save_and_load() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_render_test.png");
        let pixels = vec![200u8; 4 * 3 * 3];
        let img = RenderedImage::new(3, 3, ImageFormat::Png, pixels, 0);
        img.save(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    // 测试 mock PdfRenderer 实现
    struct MockRenderer;

    impl PdfRenderer for MockRenderer {
        fn render_page(&self, page_index: usize, config: &RenderConfig) -> Result<RenderedImage> {
            let pixels = vec![255u8; 4 * 10 * 10];
            Ok(RenderedImage::new(
                10,
                10,
                config.format,
                pixels,
                page_index,
            ))
        }

        fn name(&self) -> &'static str {
            "mock"
        }
    }

    #[test]
    fn mock_renderer_render_page() {
        let renderer = MockRenderer;
        let config = RenderConfig::default();
        let img = renderer.render_page(0, &config).unwrap();
        assert_eq!(img.width, 10);
        assert_eq!(img.page_index, 0);
    }

    #[test]
    fn mock_renderer_name() {
        let renderer = MockRenderer;
        assert_eq!(renderer.name(), "mock");
    }

    #[test]
    fn mock_renderer_max_dpi_default() {
        let renderer = MockRenderer;
        assert_eq!(renderer.max_dpi(), 600);
    }

    #[test]
    fn mock_renderer_supports_vector_default() {
        let renderer = MockRenderer;
        assert!(!renderer.supports_vector());
    }

    #[test]
    fn mock_renderer_render_pages() {
        let renderer = MockRenderer;
        let config = RenderConfig::default();
        let images = renderer.render_pages(0..3, &config).unwrap();
        assert_eq!(images.len(), 3);
        assert_eq!(images[0].page_index, 0);
        assert_eq!(images[2].page_index, 2);
    }

    #[test]
    fn mock_renderer_render_page_to_path() {
        let renderer = MockRenderer;
        let config = RenderConfig::default();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_render_page_to_path_test.png");
        renderer.render_page_to_path(0, &config, &path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }
}
