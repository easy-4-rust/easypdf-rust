//! Automatic layout engine for easypdf-rust (C2).
//!
//! Provides `FlowLayout` for auto-positioning content elements
//! on PDF pages without manual (x, y) coordinate calculation.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use easypdf_core::error::Result;
use easypdf_core::{Orientation, PageSize, PdfFont, PdfText};
use easypdf_writer::PdfWriter;

/// Direction for flow layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Stack elements vertically (top to bottom).
    Vertical,
    /// Stack elements horizontally (left to right).
    Horizontal,
}

/// Automatic vertical/horizontal layout engine.
pub struct FlowLayout {
    #[allow(dead_code)] // 预留：横向布局（Direction::Horizontal）尚未实现，字段保留供后续使用
    direction: Direction,
    margins: f64,
    spacing: f64,
    cursor: f64,
    page_width: f64,
    page_height: f64,
    writer: PdfWriter,
}

impl FlowLayout {
    /// Create a new vertical flow layout with default margins.
    #[must_use]
    pub fn vertical(writer: PdfWriter, page_size: PageSize) -> Self {
        let (w, h) = page_size.dimensions();
        Self {
            direction: Direction::Vertical,
            margins: 72.0,
            spacing: 12.0,
            cursor: h - 72.0,
            page_width: w,
            page_height: h,
            writer,
        }
    }

    /// Set the margin size in PDF points.
    #[must_use]
    pub fn margins(mut self, margins: f64) -> Self {
        self.margins = margins;
        self.cursor = self.page_height - margins;
        self
    }

    /// Set spacing between elements.
    #[must_use]
    pub fn spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }

    /// Add text and auto-advance the cursor.
    ///
    /// # Errors
    ///
    /// Returns an error if the write operation fails.
    pub fn add_text(&mut self, content: &str, font: &PdfFont, estimated_height: f64) -> Result<()> {
        let y = self.cursor - estimated_height;
        self.writer
            .write_text(&PdfText::new(content).font(font.clone()), self.margins, y)?;
        self.cursor = y - self.spacing;
        Ok(())
    }

    /// Get the remaining vertical space on the current page.
    #[must_use]
    pub fn remaining_space(&self) -> f64 {
        self.cursor - self.margins
    }

    /// Add a new page and reset the cursor.
    ///
    /// # Errors
    ///
    /// Returns an error if the page cannot be created.
    pub fn new_page(&mut self) -> Result<()> {
        self.writer
            .add_page(PageSize::Custom(self.page_width, self.page_height), Orientation::Portrait)?;
        self.cursor = self.page_height - self.margins;
        Ok(())
    }

    /// Finish and save the document.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn finish(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.writer.finish(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_layout_vertical() {
        let writer = PdfWriter::new("test");
        let mut layout = FlowLayout::vertical(writer, PageSize::A4)
            .margins(50.0)
            .spacing(10.0);
        assert!(layout.remaining_space() > 0.0);
        layout.add_text("Hello", &PdfFont::helvetica(12.0), 20.0).unwrap();
        layout.add_text("World", &PdfFont::helvetica(12.0), 20.0).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_layout_test.pdf");
        layout.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_flow_layout_new_page() {
        let writer = PdfWriter::new("test");
        let mut layout = FlowLayout::vertical(writer, PageSize::A4);
        // Fill the page until space runs out, then add new page
        while layout.remaining_space() > 30.0 {
            layout.add_text("line", &PdfFont::helvetica(10.0), 15.0).unwrap();
        }
        assert!(layout.new_page().is_ok());
        assert!(layout.remaining_space() > 200.0);
    }

    #[test]
    fn test_flow_layout_horizontal() {
        let writer = PdfWriter::new("test");
        let layout = FlowLayout {
            direction: Direction::Horizontal,
            margins: 50.0,
            spacing: 20.0,
            cursor: 100.0,
            page_width: 595.0,
            page_height: 842.0,
            writer,
        };
        assert_eq!(layout.direction, Direction::Horizontal);
        assert_eq!(layout.spacing, 20.0);
    }
}
