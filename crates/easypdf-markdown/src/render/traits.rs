//! Core rendering traits and output types.

use std::ops::Range;
use std::path::Path;

use super::config::{ImageFormat, RenderConfig};
use super::error::Result;

/// A rendered page image with raw pixel data.
///
/// Contains RGBA pixel bytes along with dimensions and metadata.
/// Use [`save`](Self::save) to write to disk, or
/// [`to_dynamic_image`](Self::to_dynamic_image) to convert to an
/// `image::DynamicImage` for further processing.
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
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The format used for encoding.
    pub format: ImageFormat,
    /// Raw RGBA pixel bytes (4 bytes per pixel, row-major, top-to-bottom).
    pub pixels: Vec<u8>,
    /// 0-based page index of the rendered page.
    pub page_index: usize,
}

impl RenderedImage {
    /// Create a new `RenderedImage` from raw RGBA pixels.
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

    /// Save the rendered image to a file.
    ///
    /// The output format is determined by the file extension (`.png`, `.jpg`,
    /// `.jpeg`) or falls back to the format stored in this image.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Io`](super::RenderError::Io) if the file cannot
    /// be written, or
    /// [`RenderError::ImageEncode`](super::RenderError::ImageEncode) if
    /// encoding fails.
    pub fn save(&self, path: &Path) -> Result<()> {
        let dynamic = self.to_dynamic_image();
        dynamic
            .save(path)
            .map_err(|e| super::RenderError::ImageEncode(e.to_string()))?;
        Ok(())
    }

    /// Convert to an `image::DynamicImage`.
    ///
    /// Returns an `Rgba8` image backed by the stored pixel buffer.
    ///
    /// # Panics
    ///
    /// Panics if the pixel buffer length does not equal `width * height * 4`.
    #[must_use]
    pub fn to_dynamic_image(&self) -> image::DynamicImage {
        let img = image::RgbaImage::from_raw(self.width, self.height, self.pixels.clone())
            .expect("pixel buffer size must match width * height * 4");
        image::DynamicImage::ImageRgba8(img)
    }

    /// Encode the image as PNG bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::ImageEncode`](super::RenderError::ImageEncode)
    /// if encoding fails.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>> {
        let dynamic = self.to_dynamic_image();
        let mut buf = Vec::new();
        dynamic
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| super::RenderError::ImageEncode(e.to_string()))?;
        Ok(buf)
    }
}

/// PDF page renderer abstraction.
///
/// Implementors provide page-to-raster conversion using a specific backend
/// (e.g., pdfium, text fallback). The trait is object-safe and requires
/// `Send + Sync` for use across threads.
///
/// # Implementing a custom backend
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
    /// Render a single page to an RGBA pixel buffer.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`](super::RenderError) if the page index is invalid, the DPI exceeds
    /// the backend maximum, or the rendering fails.
    fn render_page(&self, page_index: usize, config: &RenderConfig) -> Result<RenderedImage>;

    /// Render a single page and save directly to a file path.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`](super::RenderError) if rendering or file writing fails.
    fn render_page_to_path(
        &self,
        page_index: usize,
        config: &RenderConfig,
        output: &Path,
    ) -> Result<()> {
        self.render_page(page_index, config)?.save(output)
    }

    /// Render a range of pages.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`](super::RenderError) if any page in the range fails to render.
    fn render_pages(
        &self,
        page_range: Range<usize>,
        config: &RenderConfig,
    ) -> Result<Vec<RenderedImage>> {
        page_range.map(|i| self.render_page(i, config)).collect()
    }

    /// Human-readable name of this rendering backend.
    fn name(&self) -> &'static str;

    /// Maximum DPI supported by this backend. Default: 600.
    fn max_dpi(&self) -> u32 {
        600
    }

    /// Whether this backend supports vector (SVG) output. Default: `false`.
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
        // PNG magic bytes
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

    // Test a mock PdfRenderer implementation
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
