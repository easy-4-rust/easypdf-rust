//! Rendering configuration types.

/// Configuration for PDF page rendering.
///
/// Controls DPI, output format, background color, and optional dimension
/// constraints. Use [`RenderConfig::default`] for sensible defaults
/// (150 DPI, PNG, white background, no size limits).
///
/// # Examples
///
/// ```
/// use easypdf_markdown::render::{RenderConfig, ImageFormat, Background};
///
/// let config = RenderConfig {
///     dpi: 300,
///     format: ImageFormat::Png,
///     background: Background::White,
///     max_width: Some(2048),
///     max_height: None,
/// };
/// assert_eq!(config.dpi, 300);
/// ```
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Rendering resolution in dots per inch. Default: 150.
    pub dpi: u32,
    /// Output image format. Default: [`ImageFormat::Png`].
    pub format: ImageFormat,
    /// Page background color. Default: [`Background::White`].
    pub background: Background,
    /// Maximum output width in pixels. `None` means no limit.
    pub max_width: Option<u32>,
    /// Maximum output height in pixels. `None` means no limit.
    pub max_height: Option<u32>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            dpi: 150,
            format: ImageFormat::Png,
            background: Background::White,
            max_width: None,
            max_height: None,
        }
    }
}

/// Output image format for rendered pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// Portable Network Graphics (lossless).
    Png,
    /// JPEG (lossy, smaller files).
    Jpeg,
}

/// Background color for rendered pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Background {
    /// Solid white background.
    White,
    /// Transparent background (requires PNG output).
    Transparent,
}
