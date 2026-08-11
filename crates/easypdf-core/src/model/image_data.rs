//! PDF 图片元数据与格式。

/// 从 PDF 中识别的图片信息。
///
/// `bytes` 为 `None` 时表示仅存在引用（未提取原始数据）。
///
/// # Examples
///
/// ```
/// use easypdf_core::{ImageData, ImageFormat};
///
/// let data = ImageData::new(ImageFormat::Png)
///     .with_caption("Logo")
///     .with_dimensions(200.0, 100.0);
/// assert_eq!(data.format(), ImageFormat::Png);
/// assert_eq!(data.width_pt(), Some(200.0));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ImageData {
    /// 图片格式。
    format: ImageFormat,
    /// 图片原始字节；`None` 表示未提取。
    bytes: Option<Vec<u8>>,
    /// 图片标题。
    caption: Option<String>,
    /// 替代文本。
    alt_text: Option<String>,
    /// 宽度（PDF points）。
    width_pt: Option<f64>,
    /// 高度（PDF points）。
    height_pt: Option<f64>,
}

impl ImageData {
    /// 创建指定格式的图片数据。
    #[must_use]
    pub const fn new(format: ImageFormat) -> Self {
        Self {
            format,
            bytes: None,
            caption: None,
            alt_text: None,
            width_pt: None,
            height_pt: None,
        }
    }

    /// 设置原始字节。
    #[must_use]
    pub fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.bytes = Some(bytes);
        self
    }

    /// 设置标题。
    #[must_use]
    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// 设置替代文本。
    #[must_use]
    pub fn with_alt_text(mut self, alt: impl Into<String>) -> Self {
        self.alt_text = Some(alt.into());
        self
    }

    /// 设置图片尺寸（PDF points）。
    #[must_use]
    pub const fn with_dimensions(mut self, width_pt: f64, height_pt: f64) -> Self {
        self.width_pt = Some(width_pt);
        self.height_pt = Some(height_pt);
        self
    }

    /// 返回图片格式。
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    /// 返回原始字节。
    #[must_use]
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }

    /// 返回标题。
    #[must_use]
    pub fn caption(&self) -> Option<&str> {
        self.caption.as_deref()
    }

    /// 返回替代文本。
    #[must_use]
    pub fn alt_text(&self) -> Option<&str> {
        self.alt_text.as_deref()
    }

    /// 返回宽度（PDF points）。
    #[must_use]
    pub const fn width_pt(&self) -> Option<f64> {
        self.width_pt
    }

    /// 返回高度（PDF points）。
    #[must_use]
    pub const fn height_pt(&self) -> Option<f64> {
        self.height_pt
    }
}

/// 图片格式枚举。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// PNG 格式。
    Png,
    /// JPEG 格式。
    Jpeg,
    /// GIF 格式。
    Gif,
    /// BMP 格式。
    Bmp,
    /// TIFF 格式。
    Tiff,
    /// SVG 格式。
    Svg,
    /// 未知格式。
    Unknown,
}

impl ImageFormat {
    /// 返回格式的典型文件扩展名。
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Svg => "svg",
            Self::Unknown => "bin",
        }
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_with_format_only() {
        let data = ImageData::new(ImageFormat::Png);
        assert_eq!(data.format(), ImageFormat::Png);
        assert!(data.bytes().is_none());
        assert!(data.caption().is_none());
        assert!(data.alt_text().is_none());
        assert!(data.width_pt().is_none());
        assert!(data.height_pt().is_none());
    }

    #[test]
    fn with_bytes_sets_bytes() {
        let data = ImageData::new(ImageFormat::Jpeg).with_bytes(vec![0xFF, 0xD8, 0xFF]);
        assert_eq!(data.bytes(), Some([0xFF, 0xD8, 0xFF].as_slice()));
    }

    #[test]
    fn with_caption_sets_caption() {
        let data = ImageData::new(ImageFormat::Png).with_caption("Logo");
        assert_eq!(data.caption(), Some("Logo"));
    }

    #[test]
    fn with_alt_text_sets_alt_text() {
        let data = ImageData::new(ImageFormat::Gif).with_alt_text("A cat");
        assert_eq!(data.alt_text(), Some("A cat"));
    }

    #[test]
    fn with_dimensions_sets_both() {
        let data = ImageData::new(ImageFormat::Bmp).with_dimensions(200.0, 100.0);
        assert_eq!(data.width_pt(), Some(200.0));
        assert_eq!(data.height_pt(), Some(100.0));
    }

    #[test]
    fn builder_chaining_sets_all_fields() {
        let data = ImageData::new(ImageFormat::Svg)
            .with_bytes(vec![1, 2, 3])
            .with_caption("SVG Icon")
            .with_alt_text("icon")
            .with_dimensions(64.0, 64.0);
        assert_eq!(data.format(), ImageFormat::Svg);
        assert_eq!(data.bytes(), Some([1, 2, 3].as_slice()));
        assert_eq!(data.caption(), Some("SVG Icon"));
        assert_eq!(data.alt_text(), Some("icon"));
        assert_eq!(data.width_pt(), Some(64.0));
        assert_eq!(data.height_pt(), Some(64.0));
    }

    #[test]
    fn clone_preserves_all_fields() {
        let data = ImageData::new(ImageFormat::Tiff)
            .with_bytes(vec![10, 20])
            .with_caption("Photo")
            .with_dimensions(800.0, 600.0);
        let cloned = data.clone();
        assert_eq!(data, cloned);
    }

    #[test]
    fn partial_eq_works() {
        let a = ImageData::new(ImageFormat::Png).with_caption("A");
        let b = ImageData::new(ImageFormat::Png).with_caption("A");
        let c = ImageData::new(ImageFormat::Png).with_caption("B");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn extension_png() {
        assert_eq!(ImageFormat::Png.extension(), "png");
    }

    #[test]
    fn extension_jpeg() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn extension_gif() {
        assert_eq!(ImageFormat::Gif.extension(), "gif");
    }

    #[test]
    fn extension_bmp() {
        assert_eq!(ImageFormat::Bmp.extension(), "bmp");
    }

    #[test]
    fn extension_tiff() {
        assert_eq!(ImageFormat::Tiff.extension(), "tiff");
    }

    #[test]
    fn extension_svg() {
        assert_eq!(ImageFormat::Svg.extension(), "svg");
    }

    #[test]
    fn extension_unknown() {
        assert_eq!(ImageFormat::Unknown.extension(), "bin");
    }

    #[test]
    fn image_format_eq() {
        assert_eq!(ImageFormat::Png, ImageFormat::Png);
        assert_ne!(ImageFormat::Png, ImageFormat::Jpeg);
    }
}
