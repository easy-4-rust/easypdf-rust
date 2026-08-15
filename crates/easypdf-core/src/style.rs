//! 样式类型——颜色、字体和边框。

use std::borrow::Cow;

// --- 颜色 ---

/// 表示不同颜色空间中的颜色。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfColor {
    /// RGB 颜色，分量范围 0.0–1.0。
    Rgb(f64, f64, f64),
    /// 灰度颜色，分量范围 0.0–1.0。
    Gray(f64),
    /// CMYK 颜色，分量范围 0.0–1.0。
    Cmyk(f64, f64, f64, f64),
}

impl Default for PdfColor {
    fn default() -> Self {
        Self::Rgb(0.0, 0.0, 0.0) // black
    }
}

impl PdfColor {
    /// 从 0–255 整数分量创建 RGB 颜色。
    #[must_use]
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(
            f64::from(r) / 255.0,
            f64::from(g) / 255.0,
            f64::from(b) / 255.0,
        )
    }

    /// 黑色。
    #[must_use]
    pub const fn black() -> Self {
        Self::Rgb(0.0, 0.0, 0.0)
    }

    /// 白色。
    #[must_use]
    pub const fn white() -> Self {
        Self::Rgb(1.0, 1.0, 1.0)
    }

    /// 红色。
    #[must_use]
    pub const fn red() -> Self {
        Self::Rgb(1.0, 0.0, 0.0)
    }

    /// 绿色。
    #[must_use]
    pub const fn green() -> Self {
        Self::Rgb(0.0, 1.0, 0.0)
    }

    /// 蓝色。
    #[must_use]
    pub const fn blue() -> Self {
        Self::Rgb(0.0, 0.0, 1.0)
    }

    /// 浅灰色（0.8）。
    #[must_use]
    pub const fn light_gray() -> Self {
        Self::Gray(0.8)
    }

    /// 中灰色（0.5）。
    #[must_use]
    pub const fn gray() -> Self {
        Self::Gray(0.5)
    }
}

// --- 字体 ---

/// 字体族规格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontFamily {
    /// 14 种内置 PDF 字体之一。
    BuiltIn(BuiltInFont),
    /// 从 TTF/OTF 文件路径加载的自定义字体。
    Custom(Cow<'static, str>),
}

/// 保证在每个 PDF 阅读器中可用的 14 种标准 Type 1 字体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInFont {
    /// Times-Roman（衬线体）。
    TimesRoman,
    /// Times-Bold。
    TimesBold,
    /// Times-Italic。
    TimesItalic,
    /// Times-BoldItalic。
    TimesBoldItalic,
    /// Helvetica（无衬线体）。
    Helvetica,
    /// Helvetica-Bold。
    HelveticaBold,
    /// Helvetica-Oblique。
    HelveticaOblique,
    /// Helvetica-BoldOblique。
    HelveticaBoldOblique,
    /// Courier（等宽体）。
    Courier,
    /// Courier-Bold。
    CourierBold,
    /// Courier-Oblique。
    CourierOblique,
    /// Courier-BoldOblique。
    CourierBoldOblique,
    /// Symbol。
    Symbol,
    /// `ZapfDingbats`。
    ZapfDingbats,
}

/// 字体样式修饰符。
#[derive(Debug, Clone, Copy, Default)]
pub struct FontStyle {
    /// 粗体。
    pub bold: bool,
    /// 斜体/倾斜。
    pub italic: bool,
}

/// 完整的字体规格。
#[derive(Debug, Clone)]
pub struct PdfFont {
    /// 字体族名称或路径。
    pub family: FontFamily,
    /// 字体大小（PDF 点）。
    pub size: f64,
    /// 粗体和/或斜体。
    pub style: FontStyle,
}

impl Default for PdfFont {
    fn default() -> Self {
        Self {
            family: FontFamily::BuiltIn(BuiltInFont::Helvetica),
            size: 12.0,
            style: FontStyle::default(),
        }
    }
}

impl PdfFont {
    /// 使用给定大小的 Helvetica 字体。
    #[must_use]
    pub fn helvetica(size: f64) -> Self {
        Self {
            family: FontFamily::BuiltIn(BuiltInFont::Helvetica),
            size,
            style: FontStyle {
                bold: false,
                italic: false,
            },
        }
    }

    /// 使用给定大小的 Times-Roman 字体。
    #[must_use]
    pub fn times_roman(size: f64) -> Self {
        Self {
            family: FontFamily::BuiltIn(BuiltInFont::TimesRoman),
            size,
            style: FontStyle {
                bold: false,
                italic: false,
            },
        }
    }

    /// 使用给定大小的 Courier 字体。
    #[must_use]
    pub fn courier(size: f64) -> Self {
        Self {
            family: FontFamily::BuiltIn(BuiltInFont::Courier),
            size,
            style: FontStyle {
                bold: false,
                italic: false,
            },
        }
    }

    /// 设置字体大小。
    #[must_use]
    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// 启用粗体。
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.style.bold = true;
        self
    }

    /// 启用斜体。
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        self
    }
}

// --- 边框 ---

/// 表格单元格边框定义。
#[derive(Debug, Clone, Copy)]
pub struct TableBorder {
    /// 边框宽度（PDF 点，0 = 无边框）。
    pub width: f64,
    /// 边框颜色。
    pub color: PdfColor,
}

impl Default for TableBorder {
    fn default() -> Self {
        Self {
            width: 0.5,
            color: PdfColor::black(),
        }
    }
}

/// 预定义表格样式。
#[derive(Debug, Clone)]
pub struct TableStyle {
    /// 表头背景颜色。
    pub header_bg: Option<PdfColor>,
    /// 表头字体。
    pub header_font: PdfFont,
    /// 正文字体。
    pub body_font: PdfFont,
    /// 单元格边框。
    pub border: TableBorder,
    /// 是否使用交替行颜色。
    pub striped: bool,
    /// 交替行背景颜色（`striped` 为 `true` 时使用）。
    pub stripe_color: PdfColor,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            header_bg: Some(PdfColor::light_gray()),
            header_font: PdfFont::helvetica(11.0).bold(),
            body_font: PdfFont::helvetica(10.0),
            border: TableBorder::default(),
            striped: false,
            stripe_color: PdfColor::Gray(0.95),
        }
    }
}

impl TableStyle {
    /// 创建无背景色、细边框的简洁表格样式。
    #[must_use]
    pub fn simple() -> Self {
        Self {
            header_bg: None,
            striped: false,
            ..Default::default()
        }
    }

    /// 创建带交替（条纹）行颜色的表格样式。
    #[must_use]
    pub fn striped() -> Self {
        Self {
            header_bg: Some(PdfColor::Gray(0.7)),
            striped: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn color_default_is_black() {
        let c = PdfColor::default();
        assert_eq!(c, PdfColor::Rgb(0.0, 0.0, 0.0));
    }

    #[test]
    fn color_rgb_u8() {
        let c = PdfColor::rgb_u8(255, 0, 0);
        assert_eq!(c, PdfColor::Rgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn color_named() {
        assert_eq!(PdfColor::black(), PdfColor::Rgb(0.0, 0.0, 0.0));
        assert_eq!(PdfColor::white(), PdfColor::Rgb(1.0, 1.0, 1.0));
        assert_eq!(PdfColor::red(), PdfColor::Rgb(1.0, 0.0, 0.0));
        assert_eq!(PdfColor::green(), PdfColor::Rgb(0.0, 1.0, 0.0));
        assert_eq!(PdfColor::blue(), PdfColor::Rgb(0.0, 0.0, 1.0));
    }

    #[test]
    fn color_gray() {
        assert_eq!(PdfColor::light_gray(), PdfColor::Gray(0.8));
        assert_eq!(PdfColor::gray(), PdfColor::Gray(0.5));
    }

    #[test]
    fn color_cmyk() {
        let c = PdfColor::Cmyk(0.0, 1.0, 1.0, 0.0);
        let _ = format!("{:?}", c);
    }

    #[test]
    fn color_clone_copy() {
        let c = PdfColor::red();
        let copied = c;
        assert_eq!(c, copied);
    }

    #[test]
    fn font_default() {
        let f = PdfFont::default();
        assert_eq!(f.size, 12.0);
    }

    #[test]
    fn font_helvetica() {
        let f = PdfFont::helvetica(14.0);
        assert_eq!(f.size, 14.0);
    }

    #[test]
    fn font_times_roman() {
        let f = PdfFont::times_roman(10.0);
        assert_eq!(f.size, 10.0);
    }

    #[test]
    fn font_courier() {
        let f = PdfFont::courier(8.0);
        assert_eq!(f.size, 8.0);
    }

    #[test]
    fn font_with_size() {
        let f = PdfFont::default().with_size(20.0);
        assert_eq!(f.size, 20.0);
    }

    #[test]
    fn font_bold() {
        let f = PdfFont::default().bold();
        assert!(f.style.bold);
    }

    #[test]
    fn font_italic() {
        let f = PdfFont::default().italic();
        assert!(f.style.italic);
    }

    #[test]
    fn font_debug() {
        let f = PdfFont::default();
        let _ = format!("{:?}", f);
    }

    #[test]
    fn font_family_eq() {
        assert_eq!(
            FontFamily::BuiltIn(BuiltInFont::Helvetica),
            FontFamily::BuiltIn(BuiltInFont::Helvetica)
        );
        assert_ne!(
            FontFamily::BuiltIn(BuiltInFont::Helvetica),
            FontFamily::BuiltIn(BuiltInFont::Courier)
        );
    }

    #[test]
    fn built_in_font_variants() {
        assert_ne!(BuiltInFont::TimesRoman, BuiltInFont::TimesBold);
    }

    #[test]
    fn font_style_default() {
        let fs = FontStyle::default();
        assert!(!fs.bold);
        assert!(!fs.italic);
    }

    #[test]
    fn table_border_default() {
        let b = TableBorder::default();
        assert_eq!(b.width, 0.5);
    }

    #[test]
    fn table_style_default() {
        let s = TableStyle::default();
        assert!(s.header_bg.is_some());
        assert!(!s.striped);
    }

    #[test]
    fn table_style_simple() {
        let s = TableStyle::simple();
        assert!(s.header_bg.is_none());
        assert!(!s.striped);
    }

    #[test]
    fn table_style_striped() {
        let s = TableStyle::striped();
        assert!(s.header_bg.is_some());
        assert!(s.striped);
    }

    #[test]
    fn table_style_debug_clone() {
        let s = TableStyle::default();
        let cloned = s.clone();
        assert_eq!(s.striped, cloned.striped);
        let _ = format!("{:?}", s);
    }
}
