//! Content model types for PDF elements — text, tables, images, and shapes.

use crate::enums::{TextAlignment, VerticalAlignment};
use crate::style::{PdfColor, PdfFont};

// --- Text ---

/// A block of positioned text with formatting.
#[derive(Debug, Clone)]
pub struct PdfText {
    /// The text string to render.
    pub content: String,
    /// Horizontal alignment within the text block.
    pub alignment: TextAlignment,
    /// Font specification for this text.
    pub font: PdfFont,
    /// Text color.
    pub color: PdfColor,
}

impl PdfText {
    /// Create a new text element with the given content.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            alignment: TextAlignment::default(),
            font: PdfFont::default(),
            color: PdfColor::default(),
        }
    }

    /// Set the font for this text.
    #[must_use]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.font = font;
        self
    }

    /// Set the alignment for this text.
    #[must_use]
    pub const fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the color for this text.
    #[must_use]
    pub const fn color(mut self, color: PdfColor) -> Self {
        self.color = color;
        self
    }
}

// --- Table ---

/// Configuration for a table to be rendered in a PDF.
#[derive(Debug, Clone)]
pub struct PdfTable {
    /// Table headers.
    pub headers: Vec<String>,
    /// Row data (each row is a vec of string values).
    pub rows: Vec<Vec<String>>,
    /// Column widths in PDF points. If empty, columns are evenly distributed.
    pub column_widths: Vec<f64>,
    /// Overall table width in PDF points.
    pub width: f64,
}

impl PdfTable {
    /// Create a new table with the given headers.
    #[must_use]
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
            column_widths: Vec::new(),
            width: 0.0,
        }
    }

    /// Add a data row to the table.
    #[must_use]
    pub fn row(mut self, row: Vec<String>) -> Self {
        self.rows.push(row);
        self
    }

    /// Add multiple data rows to the table.
    #[must_use]
    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Set the table width.
    #[must_use]
    pub const fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
}

// --- Table Cell ---

/// A single cell within a table.
#[derive(Debug, Clone, Default)]
pub struct PdfTableCell {
    /// Cell text content.
    pub content: String,
    /// Horizontal alignment within the cell.
    pub h_alignment: TextAlignment,
    /// Vertical alignment within the cell.
    pub v_alignment: VerticalAlignment,
    /// Font specification.
    pub font: PdfFont,
    /// Text color.
    pub color: PdfColor,
}

// --- Image ---

/// An image to be embedded in a PDF.
#[derive(Debug, Clone)]
pub struct PdfImage {
    /// Raw image bytes (PNG, JPEG, etc. — format auto-detected).
    pub data: Vec<u8>,
    /// Desired width in PDF points (0 = use natural size at 72 DPI).
    pub width: f64,
    /// Desired height in PDF points (0 = use natural size at 72 DPI).
    pub height: f64,
}

impl PdfImage {
    /// Create an image from raw bytes.
    #[must_use]
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data,
            width: 0.0,
            height: 0.0,
        }
    }

    /// Create an image from a file path.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Io` if the file cannot be read.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> crate::error::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self::from_bytes(data))
    }
}

// --- Shape ---

/// A line segment.
#[derive(Debug, Clone, Copy)]
pub struct PdfLine {
    /// Start x coordinate.
    pub x1: f64,
    /// Start y coordinate.
    pub y1: f64,
    /// End x coordinate.
    pub x2: f64,
    /// End y coordinate.
    pub y2: f64,
    /// Line width in PDF points.
    pub width: f64,
    /// Line color.
    pub color: PdfColor,
}

/// A rectangle.
#[derive(Debug, Clone, Copy)]
pub struct PdfRect {
    /// Lower-left x.
    pub x: f64,
    /// Lower-left y.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
    /// Border width (0 = no border).
    pub border_width: f64,
    /// Border color.
    pub border_color: PdfColor,
    /// Fill color (transparent if `None`).
    pub fill_color: Option<PdfColor>,
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn pdf_text_new() {
        let t = PdfText::new("hello");
        assert_eq!(t.content, "hello");
    }

    #[test]
    fn pdf_text_font() {
        let t = PdfText::new("x").font(PdfFont::default());
        let _ = format!("{:?}", t.font);
    }

    #[test]
    fn pdf_text_alignment() {
        let t = PdfText::new("x").alignment(TextAlignment::Center);
        assert_eq!(t.alignment, TextAlignment::Center);
    }

    #[test]
    fn pdf_text_color() {
        let t = PdfText::new("x").color(PdfColor::default());
        assert_eq!(t.color, PdfColor::default());
    }

    #[test]
    fn pdf_text_debug_clone() {
        let t = PdfText::new("test");
        let cloned = t.clone();
        assert_eq!(t.content, cloned.content);
        let _ = format!("{:?}", t);
    }

    #[test]
    fn pdf_table_new() {
        let t = PdfTable::new(vec!["A".into(), "B".into()]);
        assert_eq!(t.headers.len(), 2);
        assert!(t.rows.is_empty());
    }

    #[test]
    fn pdf_table_row() {
        let t = PdfTable::new(vec!["A".into()])
            .row(vec!["1".into()]);
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn pdf_table_rows() {
        let t = PdfTable::new(vec!["A".into()])
            .rows(vec![vec!["1".into()], vec!["2".into()]]);
        assert_eq!(t.rows.len(), 2);
    }

    #[test]
    fn pdf_table_width() {
        let t = PdfTable::new(vec!["A".into()]).width(200.0);
        assert_eq!(t.width, 200.0);
    }

    #[test]
    fn pdf_table_debug_clone() {
        let t = PdfTable::new(vec!["A".into()]);
        let cloned = t.clone();
        assert_eq!(t.headers, cloned.headers);
        let _ = format!("{:?}", t);
    }

    #[test]
    fn pdf_image_from_bytes() {
        let img = PdfImage::from_bytes(vec![1, 2, 3]);
        assert_eq!(img.data, vec![1, 2, 3]);
        assert_eq!(img.width, 0.0);
        assert_eq!(img.height, 0.0);
    }

    #[test]
    fn pdf_image_debug_clone() {
        let img = PdfImage::from_bytes(vec![1]);
        let cloned = img.clone();
        assert_eq!(img.data, cloned.data);
        let _ = format!("{:?}", img);
    }

    #[test]
    fn pdf_table_cell_default() {
        let cell = PdfTableCell::default();
        assert!(cell.content.is_empty());
    }

    #[test]
    fn pdf_table_cell_debug_clone() {
        let cell = PdfTableCell { content: "x".into(), ..Default::default() };
        let cloned = cell.clone();
        assert_eq!(cell.content, cloned.content);
        let _ = format!("{:?}", cell);
    }

    #[test]
    fn pdf_line_debug_copy() {
        let line = PdfLine { x1: 0.0, y1: 0.0, x2: 100.0, y2: 100.0, width: 1.0, color: PdfColor::default() };
        let copied = line;
        assert_eq!(line.x2, copied.x2);
        let _ = format!("{:?}", line);
    }

    #[test]
    fn pdf_rect_debug_copy() {
        let rect = PdfRect { x: 0.0, y: 0.0, w: 100.0, h: 50.0, border_width: 1.0, border_color: PdfColor::default(), fill_color: None };
        let copied = rect;
        assert_eq!(rect.w, copied.w);
        let _ = format!("{:?}", rect);
    }
}
