//! 渲染配置类型。

/// PDF 页面渲染配置。
///
/// 控制 DPI、输出格式、背景色和可选的尺寸约束。
/// 使用 [`RenderConfig::default`] 获取合理默认值
///（150 DPI、PNG、白色背景、无尺寸限制）。
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
    /// 渲染分辨率（每英寸点数）。默认值：150。
    pub dpi: u32,
    /// 输出图像格式。默认值：[`ImageFormat::Png`]。
    pub format: ImageFormat,
    /// 页面背景色。默认值：[`Background::White`]。
    pub background: Background,
    /// 最大输出宽度（像素）。`None` 表示无限制。
    pub max_width: Option<u32>,
    /// 最大输出高度（像素）。`None` 表示无限制。
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

/// 渲染页面的输出图像格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// 便携式网络图形（无损）。
    Png,
    /// JPEG（有损，文件更小）。
    Jpeg,
}

/// 渲染页面的背景色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Background {
    /// 纯白背景。
    White,
    /// 透明背景（需要 PNG 输出）。
    Transparent,
}
