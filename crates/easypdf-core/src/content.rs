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
