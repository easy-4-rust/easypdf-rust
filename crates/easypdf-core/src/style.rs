//! Style types — colors, fonts, and borders.

use std::borrow::Cow;

// --- Color ---

/// Represents a color in various color spaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfColor {
    /// RGB color with components in 0.0–1.0 range.
    Rgb(f64, f64, f64),
    /// Grayscale color with component in 0.0–1.0 range.
    Gray(f64),
    /// CMYK color with components in 0.0–1.0 range.
    Cmyk(f64, f64, f64, f64),
}

impl Default for PdfColor {
    fn default() -> Self {
        Self::Rgb(0.0, 0.0, 0.0) // black
    }
}

impl PdfColor {
    /// Create an RGB color from 0–255 integer components.
    #[must_use]
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(
            f64::from(r) / 255.0,
            f64::from(g) / 255.0,
            f64::from(b) / 255.0,
        )
    }

    /// Black.
    #[must_use]
    pub const fn black() -> Self {
        Self::Rgb(0.0, 0.0, 0.0)
    }

    /// White.
    #[must_use]
    pub const fn white() -> Self {
        Self::Rgb(1.0, 1.0, 1.0)
    }

    /// Red.
    #[must_use]
    pub const fn red() -> Self {
        Self::Rgb(1.0, 0.0, 0.0)
    }

    /// Green.
    #[must_use]
    pub const fn green() -> Self {
        Self::Rgb(0.0, 1.0, 0.0)
    }

    /// Blue.
    #[must_use]
    pub const fn blue() -> Self {
        Self::Rgb(0.0, 0.0, 1.0)
    }

    /// Light gray (0.8).
    #[must_use]
    pub const fn light_gray() -> Self {
        Self::Gray(0.8)
    }

    /// Medium gray (0.5).
    #[must_use]
    pub const fn gray() -> Self {
        Self::Gray(0.5)
    }
}

// --- Font ---

/// Font family specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontFamily {
    /// One of the 14 built-in PDF fonts.
    BuiltIn(BuiltInFont),
    /// A custom font loaded from a TTF/OTF file path.
    Custom(Cow<'static, str>),
}

/// The 14 standard Type 1 fonts guaranteed in every PDF reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInFont {
    /// Times-Roman (serif).
    TimesRoman,
    /// Times-Bold.
    TimesBold,
    /// Times-Italic.
    TimesItalic,
    /// Times-BoldItalic.
    TimesBoldItalic,
    /// Helvetica (sans-serif).
    Helvetica,
    /// Helvetica-Bold.
    HelveticaBold,
    /// Helvetica-Oblique.
    HelveticaOblique,
    /// Helvetica-BoldOblique.
    HelveticaBoldOblique,
    /// Courier (monospace).
    Courier,
    /// Courier-Bold.
    CourierBold,
    /// Courier-Oblique.
    CourierOblique,
    /// Courier-BoldOblique.
    CourierBoldOblique,
    /// Symbol.
    Symbol,
    /// `ZapfDingbats`.
    ZapfDingbats,
}

/// Font style modifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct FontStyle {
    /// Bold weight.
    pub bold: bool,
    /// Italic/oblique.
    pub italic: bool,
}

/// A complete font specification.
#[derive(Debug, Clone)]
pub struct PdfFont {
    /// Font family name or path.
    pub family: FontFamily,
    /// Font size in PDF points.
    pub size: f64,
    /// Bold and/or italic.
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
    /// Helvetica at the given size.
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

    /// Times-Roman at the given size.
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

    /// Courier at the given size.
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

    /// Set the font size.
    #[must_use]
    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Enable bold.
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.style.bold = true;
        self
    }

    /// Enable italic.
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.style.italic = true;
        self
    }
}

// --- Border ---

/// A table cell border definition.
#[derive(Debug, Clone, Copy)]
pub struct TableBorder {
    /// Border width in PDF points (0 = no border).
    pub width: f64,
    /// Border color.
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

/// Pre-defined table styles.
#[derive(Debug, Clone)]
pub struct TableStyle {
    /// Header background color.
    pub header_bg: Option<PdfColor>,
    /// Header font.
    pub header_font: PdfFont,
    /// Body font.
    pub body_font: PdfFont,
    /// Cell border.
    pub border: TableBorder,
    /// Whether to use alternating row colors.
    pub striped: bool,
    /// Alternating row background color (used when `striped` is true).
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
    /// Create a simple table style with no background colors and thin borders.
    #[must_use]
    pub fn simple() -> Self {
        Self {
            header_bg: None,
            striped: false,
            ..Default::default()
        }
    }

    /// Create a table style with alternating (striped) row colors.
    #[must_use]
    pub fn striped() -> Self {
        Self {
            header_bg: Some(PdfColor::Gray(0.7)),
            striped: true,
            ..Default::default()
        }
    }
}
