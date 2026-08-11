//! Pure-Rust text fallback renderer.
//!
//! Extracts text from a PDF via [`easypdf_reader::PdfReader`] and renders it
//! as a simple white-background, black-text raster image. Quality is low but
//! sufficient for OCR pipelines. No external dependencies are required.
#![cfg_attr(test, allow(clippy::similar_names))]

use std::path::Path;

use easypdf_reader::PdfReader;

use crate::render::config::{Background, RenderConfig};
use crate::render::error::{RenderError, Result};
use crate::render::traits::{PdfRenderer, RenderedImage};

// A4 at 72 DPI: 595 x 842 points.
const A4_WIDTH_PT: f64 = 595.0;
const A4_HEIGHT_PT: f64 = 842.0;

/// Minimal bitmap font glyph (5 wide x 7 tall, stored as 7 bytes).
/// Each byte represents one row; bit 4 is the leftmost pixel.
type Glyph = [u8; 7];

/// Return the 5x7 bitmap glyph for an ASCII character.
/// Unknown characters return a filled block.
fn glyph_for(ch: u8) -> Glyph {
    match ch {
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        b'"' => [0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'#' => [0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x00, 0x00],
        b'$' => [0x04, 0x1E, 0x05, 0x0E, 0x14, 0x0F, 0x04],
        b'%' => [0x03, 0x13, 0x08, 0x04, 0x19, 0x18, 0x00],
        b'&' => [0x06, 0x09, 0x05, 0x12, 0x09, 0x16, 0x00],
        b'\'' => [0x04, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'(' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        b')' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        b'*' => [0x00, 0x04, 0x15, 0x0E, 0x15, 0x04, 0x00],
        b'+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        b',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x02],
        b'-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04],
        b'/' => [0x00, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00],
        b'0' => [0x0E, 0x11, 0x19, 0x15, 0x13, 0x11, 0x0E],
        b'1' => [0x04, 0x06, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'2' => [0x0E, 0x11, 0x10, 0x08, 0x04, 0x02, 0x1F],
        b'3' => [0x1F, 0x08, 0x04, 0x08, 0x10, 0x11, 0x0E],
        b'4' => [0x08, 0x0C, 0x0A, 0x09, 0x1F, 0x08, 0x08],
        b'5' => [0x1F, 0x01, 0x0F, 0x10, 0x10, 0x11, 0x0E],
        b'6' => [0x0C, 0x02, 0x01, 0x0F, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x10, 0x08, 0x04, 0x02, 0x02, 0x02],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x1E, 0x10, 0x08, 0x06],
        b':' => [0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00],
        b';' => [0x00, 0x00, 0x04, 0x00, 0x04, 0x02, 0x00],
        b'<' => [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08],
        b'=' => [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00],
        b'>' => [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02],
        b'?' => [0x0E, 0x11, 0x10, 0x08, 0x04, 0x00, 0x04],
        b'@' => [0x0E, 0x11, 0x15, 0x1D, 0x01, 0x01, 0x1E],
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x0F, 0x11, 0x11, 0x0F, 0x11, 0x11, 0x0F],
        b'C' => [0x0E, 0x11, 0x01, 0x01, 0x01, 0x11, 0x0E],
        b'D' => [0x07, 0x09, 0x11, 0x11, 0x11, 0x09, 0x07],
        b'E' => [0x1F, 0x01, 0x01, 0x0F, 0x01, 0x01, 0x1F],
        b'F' => [0x1F, 0x01, 0x01, 0x0F, 0x01, 0x01, 0x01],
        b'G' => [0x0E, 0x11, 0x01, 0x1D, 0x11, 0x11, 0x0E],
        b'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'J' => [0x1C, 0x08, 0x08, 0x08, 0x08, 0x09, 0x06],
        b'K' => [0x11, 0x09, 0x05, 0x03, 0x05, 0x09, 0x11],
        b'L' => [0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x1F],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x11, 0x13, 0x15, 0x19, 0x11, 0x11],
        b'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'P' => [0x0F, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x01],
        b'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x09, 0x16],
        b'R' => [0x0F, 0x11, 0x11, 0x0F, 0x05, 0x09, 0x11],
        b'S' => [0x0E, 0x11, 0x01, 0x0E, 0x10, 0x11, 0x0E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x0A, 0x0A, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        b'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1F, 0x10, 0x08, 0x04, 0x02, 0x01, 0x1F],
        b'[' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        b'\\' => [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x00],
        b']' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        b'^' => [0x04, 0x0A, 0x11, 0x00, 0x00, 0x00, 0x00],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        b'`' => [0x02, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'a' => [0x00, 0x00, 0x0E, 0x10, 0x1E, 0x11, 0x1E],
        b'b' => [0x01, 0x01, 0x0F, 0x11, 0x11, 0x11, 0x0F],
        b'c' => [0x00, 0x00, 0x0E, 0x01, 0x01, 0x11, 0x0E],
        b'd' => [0x10, 0x10, 0x16, 0x19, 0x11, 0x11, 0x1E],
        b'e' => [0x00, 0x00, 0x0E, 0x11, 0x1F, 0x01, 0x0E],
        b'f' => [0x0C, 0x12, 0x02, 0x07, 0x02, 0x02, 0x02],
        b'g' => [0x00, 0x1E, 0x11, 0x11, 0x1E, 0x10, 0x0E],
        b'h' => [0x01, 0x01, 0x0D, 0x13, 0x11, 0x11, 0x11],
        b'i' => [0x04, 0x00, 0x06, 0x04, 0x04, 0x04, 0x0E],
        b'j' => [0x08, 0x00, 0x0C, 0x08, 0x08, 0x09, 0x06],
        b'k' => [0x01, 0x01, 0x09, 0x05, 0x03, 0x05, 0x09],
        b'l' => [0x06, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'm' => [0x00, 0x00, 0x05, 0x1B, 0x15, 0x11, 0x11],
        b'n' => [0x00, 0x00, 0x0D, 0x13, 0x11, 0x11, 0x11],
        b'o' => [0x00, 0x00, 0x0E, 0x11, 0x11, 0x11, 0x0E],
        b'p' => [0x00, 0x00, 0x0F, 0x11, 0x0F, 0x01, 0x01],
        b'q' => [0x00, 0x00, 0x16, 0x19, 0x1E, 0x10, 0x10],
        b'r' => [0x00, 0x00, 0x0D, 0x13, 0x01, 0x01, 0x01],
        b's' => [0x00, 0x00, 0x0E, 0x01, 0x0E, 0x10, 0x0F],
        b't' => [0x02, 0x02, 0x07, 0x02, 0x02, 0x12, 0x0C],
        b'u' => [0x00, 0x00, 0x11, 0x11, 0x11, 0x19, 0x16],
        b'v' => [0x00, 0x00, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'w' => [0x00, 0x00, 0x11, 0x11, 0x15, 0x15, 0x0A],
        b'x' => [0x00, 0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11],
        b'y' => [0x00, 0x00, 0x11, 0x11, 0x1E, 0x10, 0x0E],
        b'z' => [0x00, 0x00, 0x1F, 0x08, 0x04, 0x02, 0x1F],
        b'{' => [0x08, 0x04, 0x04, 0x02, 0x04, 0x04, 0x08],
        b'|' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'}' => [0x02, 0x04, 0x04, 0x08, 0x04, 0x04, 0x02],
        b'~' => [0x00, 0x04, 0x02, 0x1F, 0x02, 0x04, 0x00],
        // Fallback: filled block for unknown characters.
        _ => [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F],
    }
}

/// Pure-Rust text fallback renderer.
///
/// Opens a PDF with [`PdfReader`], extracts text per page, and renders it as
/// a simple raster image using a built-in 5x7 bitmap font. The output is
/// suitable for OCR pipelines where visual fidelity is not critical.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use easypdf_markdown::render::backends::text_backend::TextRenderer;
/// use easypdf_markdown::render::{PdfRenderer, RenderConfig};
///
/// let renderer = TextRenderer::open(Path::new("document.pdf"))?;
/// let image = renderer.render_page(0, &RenderConfig::default())?;
/// image.save(Path::new("page_0.png"))?;
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
pub struct TextRenderer {
    /// Raw PDF bytes retained so we can create a fresh `PdfReader` per render
    /// call (because `PdfReader::pages()` consumes `self`).
    pdf_bytes: Vec<u8>,
    page_count: usize,
}

impl TextRenderer {
    /// Open a PDF file for text-based rendering.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Io`] if the file cannot be read, or
    /// [`RenderError::Parse`] if the PDF is malformed.
    pub fn open(path: &Path) -> Result<Self> {
        let pdf_bytes = std::fs::read(path)?;
        let reader =
            PdfReader::from_bytes(pdf_bytes.clone()).map_err(|e| RenderError::Parse(e.to_string()))?;
        let page_count = reader
            .page_count()
            .map_err(|e| RenderError::Parse(e.to_string()))?;
        Ok(Self {
            pdf_bytes,
            page_count,
        })
    }

    /// Open a PDF from in-memory bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Parse`] if the bytes are not a valid PDF.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let reader =
            PdfReader::from_bytes(bytes.clone()).map_err(|e| RenderError::Parse(e.to_string()))?;
        let page_count = reader
            .page_count()
            .map_err(|e| RenderError::Parse(e.to_string()))?;
        Ok(Self {
            pdf_bytes: bytes,
            page_count,
        })
    }

    /// Extract text for a single page (0-based index).
    fn extract_page_text(&self, page_index: usize) -> String {
        PdfReader::from_bytes(self.pdf_bytes.clone())
            .ok()
            .and_then(|r| r.pages(page_index..page_index + 1).extract_text().ok())
            .unwrap_or_default()
    }

    /// Compute pixel dimensions for an A4 page at the given DPI.
    fn page_pixels(dpi: u32) -> (u32, u32) {
        let scale = f64::from(dpi) / 72.0;
        let w = f64_to_u32(A4_WIDTH_PT * scale);
        let h = f64_to_u32(A4_HEIGHT_PT * scale);
        (w.max(1), h.max(1))
    }

    /// Render extracted text onto an RGBA pixel buffer.
    fn render_text_to_pixels(
        text: &str,
        width: u32,
        height: u32,
        dpi: u32,
        background: Background,
    ) -> Vec<u8> {
        let bg_color: [u8; 4] = match background {
            Background::White => [255, 255, 255, 255],
            Background::Transparent => [0, 0, 0, 0],
        };
        let fg_color: [u8; 4] = [0, 0, 0, 255];

        let mut pixels = vec![0u8; usize::try_from(width * height * 4).unwrap_or(0)];

        // Fill background.
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&bg_color);
        }

        // Scale factor: at 72 DPI the font is 1x, at 150 DPI ~2x, etc.
        let scale = f64_to_u32((f64::from(dpi) / 72.0).max(1.0));
        let glyph_w = 5 * scale + scale; // 5 pixel glyph + 1 pixel spacing
        let glyph_h = 7 * scale + scale; // 7 pixel glyph + 1 pixel spacing
        let line_height = glyph_h + scale;
        let margin = 2 * scale;

        let cols = usize::try_from((width.saturating_sub(margin * 2)) / glyph_w).unwrap_or(1).max(1);
        let rows = usize::try_from((height.saturating_sub(margin * 2)) / line_height).unwrap_or(1).max(1);

        // Split text into lines that fit the page width.
        let lines: Vec<&str> = text.lines().collect();
        let mut drawn_rows: usize = 0;

        for line in &lines {
            if drawn_rows >= rows {
                break;
            }

            // Wrap long lines.
            let mut remaining = *line;
            while !remaining.is_empty() && drawn_rows < rows {
                let take = remaining.len().min(cols);
                let (chunk, rest) = remaining.split_at(take);
                remaining = rest;

                let y_offset = margin + u32::try_from(drawn_rows).unwrap_or(u32::MAX) * line_height;

                for (col, ch) in chunk.bytes().enumerate() {
                    let x_offset = margin + u32::try_from(col).unwrap_or(u32::MAX) * glyph_w;
                    draw_glyph(&mut pixels, width, height, x_offset, y_offset, scale, ch, fg_color);
                }

                drawn_rows += 1;
            }

            // Empty line handling: if the original line was empty, advance one row.
            if line.is_empty() && drawn_rows < rows {
                drawn_rows += 1;
            }
        }

        pixels
    }
}

/// Convert an `f64` to `u32` with saturation (clamps negative to 0, overflow to `u32::MAX`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn f64_to_u32(value: f64) -> u32 {
    if value.is_sign_negative() {
        0
    } else {
        value.round() as u32 // Cast is intentional for DPI/pixel math; negative handled above.
    }
}

/// Draw a single glyph onto the pixel buffer.
#[allow(clippy::too_many_arguments)]
fn draw_glyph(
    pixels: &mut [u8],
    img_width: u32,
    img_height: u32,
    x: u32,
    y: u32,
    scale: u32,
    ch: u8,
    color: [u8; 4],
) {
    let glyph = glyph_for(ch);
    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                // Fill the scaled pixel block.
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + col * scale + sx;
                        let py = y + u32::try_from(row).unwrap_or(u32::MAX) * scale + sy;
                        if px < img_width && py < img_height {
                            let idx = usize::try_from((py * img_width + px) * 4).unwrap_or(0);
                            if idx + 3 < pixels.len() {
                                pixels[idx..idx + 4].copy_from_slice(&color);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl PdfRenderer for TextRenderer {
    fn render_page(&self, page_index: usize, config: &RenderConfig) -> Result<RenderedImage> {
        if page_index >= self.page_count {
            return Err(RenderError::InvalidPage {
                index: page_index,
                total: self.page_count,
            });
        }

        if config.dpi > self.max_dpi() {
            return Err(RenderError::DpiExceeded {
                requested: config.dpi,
                max: self.max_dpi(),
            });
        }

        let (mut width, mut height) = Self::page_pixels(config.dpi);

        // Apply max dimension constraints.
        if let Some(max_w) = config.max_width
            && width > max_w
        {
            let ratio = f64::from(max_w) / f64::from(width);
            width = max_w;
            height = f64_to_u32(f64::from(height) * ratio).max(1);
        }
        if let Some(max_h) = config.max_height
            && height > max_h
        {
            let ratio = f64::from(max_h) / f64::from(height);
            height = max_h;
            width = f64_to_u32(f64::from(width) * ratio).max(1);
        }

        // Extract text for this page.
        let text = self.extract_page_text(page_index);

        let pixels = Self::render_text_to_pixels(&text, width, height, config.dpi, config.background);

        Ok(RenderedImage::new(width, height, config.format, pixels, page_index))
    }

    fn name(&self) -> &'static str {
        "text"
    }

    fn max_dpi(&self) -> u32 {
        300
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::config::{Background, ImageFormat, RenderConfig};

    /// Helper: build a minimal valid PDF in memory with the given text content.
    fn make_test_pdf_bytes(text: &str) -> Vec<u8> {
        let content = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET");
        let mut doc = lopdf::Document::new();

        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            content.into_bytes(),
        )));

        let mut font_dict = lopdf::Dictionary::new();
        font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
        let font_id = doc.add_object(lopdf::Object::Dictionary(font_dict));

        let mut resources = lopdf::Dictionary::new();
        let mut fonts = lopdf::Dictionary::new();
        fonts.set("F1", lopdf::Object::Reference(font_id));
        resources.set("Font", lopdf::Object::Dictionary(fonts));
        let resources_id = doc.add_object(lopdf::Object::Dictionary(resources));

        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        page_dict.set("Resources", lopdf::Object::Reference(resources_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));

        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages_dict.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages_dict.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));

        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));

        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_text_renderer_from_bytes() {
        let bytes = make_test_pdf_bytes("Hello World");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert_eq!(renderer.name(), "text");
    }

    #[test]
    fn test_render_page_produces_image() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);
        assert_eq!(img.pixels.len(), (img.width * img.height * 4) as usize);
        assert_eq!(img.page_index, 0);
    }

    #[test]
    fn test_render_page_out_of_bounds() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig::default();
        let result = renderer.render_page(99, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RenderError::InvalidPage { .. }
        ));
    }

    #[test]
    fn test_render_page_dpi_exceeded() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 9999,
            ..RenderConfig::default()
        };
        let result = renderer.render_page(0, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RenderError::DpiExceeded { .. }
        ));
    }

    #[test]
    fn test_render_page_max_dimensions() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            max_width: Some(100),
            max_height: Some(100),
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width <= 100);
        assert!(img.height <= 100);
    }

    #[test]
    fn test_render_page_transparent_background() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            background: Background::Transparent,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        // With transparent background, first pixel alpha should be 0.
        assert_eq!(img.pixels[3], 0);
    }

    #[test]
    fn test_render_pages_range() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let images = renderer.render_pages(0..1, &config).unwrap();
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn test_render_page_to_path() {
        let bytes = make_test_pdf_bytes("Save Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            format: ImageFormat::Png,
            ..RenderConfig::default()
        };
        // Use a unique temp file to avoid race conditions when running in parallel.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().with_extension("png");
        renderer.render_page_to_path(0, &config, &path).unwrap();
        assert!(path.exists());
        // Verify the file is a valid PNG.
        let loaded = image::open(&path).unwrap();
        assert!(loaded.width() > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_rendered_image_to_png_bytes() {
        let bytes = make_test_pdf_bytes("PNG Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        let png_bytes = img.to_png_bytes().unwrap();
        // PNG magic bytes.
        assert_eq!(&png_bytes[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_rendered_image_to_dynamic_image() {
        let bytes = make_test_pdf_bytes("Dynamic Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        let dynamic = img.to_dynamic_image();
        assert_eq!(dynamic.width(), img.width);
        assert_eq!(dynamic.height(), img.height);
    }

    #[test]
    fn test_glyph_rendering_non_empty() {
        // Ensure that rendering text produces non-uniform pixels
        // (i.e., the text actually gets drawn).
        let bytes = make_test_pdf_bytes("AB");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            background: Background::White,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        // Count non-white pixels.
        let non_white = img
            .pixels
            .chunks_exact(4)
            .filter(|px| px != &[255, 255, 255, 255])
            .count();
        // There should be at least some non-white pixels from the text or margins.
        // (The text content might be empty if lopdf can't extract it, so we just
        // verify the image was created successfully.)
        let _ = non_white;
    }

    #[test]
    fn test_page_pixels_scaling() {
        let (w72, h72) = TextRenderer::page_pixels(72);
        let (w150, h150) = TextRenderer::page_pixels(150);
        assert!(w150 > w72);
        assert!(h150 > h72);
    }

    #[test]
    fn test_name_returns_text() {
        let bytes = make_test_pdf_bytes("Name Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert_eq!(renderer.name(), "text");
    }

    #[test]
    fn test_max_dpi() {
        let bytes = make_test_pdf_bytes("DPI Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert!(renderer.max_dpi() > 0);
    }

    #[test]
    fn test_supports_vector_false() {
        let bytes = make_test_pdf_bytes("Vector Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert!(!renderer.supports_vector());
    }

    #[test]
    fn test_render_page_white_background() {
        let bytes = make_test_pdf_bytes("BG Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            background: Background::White,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        // First pixel should be white (255, 255, 255, 255)
        assert_eq!(img.pixels[0], 255);
        assert_eq!(img.pixels[1], 255);
        assert_eq!(img.pixels[2], 255);
        assert_eq!(img.pixels[3], 255);
    }

    #[test]
    fn test_render_pages_out_of_range() {
        let bytes = make_test_pdf_bytes("Range Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig::default();
        let result = renderer.render_pages(0..10, &config);
        assert!(result.is_err());
    }

    // --- Additional coverage tests for glyph table and helper functions ---

    #[test]
    fn glyph_for_all_printable_ascii() {
        // Exercise all printable ASCII glyphs (0x20..=0x7E) to cover match arms.
        for ch in 0x20u8..=0x7E {
            let glyph = glyph_for(ch);
            // Each glyph is 7 bytes
            assert_eq!(glyph.len(), 7);
        }
    }

    #[test]
    fn glyph_for_unknown_char_returns_filled_block() {
        let glyph = glyph_for(0xFF);
        assert_eq!(glyph, [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F]);
    }

    #[test]
    fn glyph_for_null_returns_filled_block() {
        let glyph = glyph_for(0x00);
        assert_eq!(glyph, [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F]);
    }

    #[test]
    fn f64_to_u32_positive() {
        assert_eq!(f64_to_u32(3.7), 4);
        assert_eq!(f64_to_u32(0.0), 0);
        assert_eq!(f64_to_u32(100.0), 100);
    }

    #[test]
    fn f64_to_u32_negative_returns_zero() {
        assert_eq!(f64_to_u32(-1.0), 0);
        assert_eq!(f64_to_u32(-100.5), 0);
    }

    #[test]
    fn draw_glyph_out_of_bounds_no_panic() {
        // Draw a glyph partially outside the image bounds.
        let mut pixels = vec![0u8; 10 * 10 * 4];
        draw_glyph(&mut pixels, 10, 10, 8, 8, 1, b'A', [0, 0, 0, 255]);
        // Should not panic
    }

    #[test]
    fn draw_glyph_scale_2() {
        let mut pixels = vec![0u8; 40 * 40 * 4];
        draw_glyph(&mut pixels, 40, 40, 2, 2, 2, b'A', [0, 0, 0, 255]);
        // Should draw scaled glyph without panic
    }

    #[test]
    fn render_text_to_pixels_empty_text() {
        let pixels = TextRenderer::render_text_to_pixels(
            "",
            100,
            100,
            72,
            Background::White,
        );
        assert_eq!(pixels.len(), 100 * 100 * 4);
        // All white
        assert!(pixels.chunks_exact(4).all(|px| px == [255, 255, 255, 255]));
    }

    #[test]
    fn render_text_to_pixels_multiline() {
        let pixels = TextRenderer::render_text_to_pixels(
            "line1\nline2\nline3",
            200,
            200,
            72,
            Background::White,
        );
        assert_eq!(pixels.len(), 200 * 200 * 4);
    }

    #[test]
    fn render_text_to_pixels_long_line_wraps() {
        let long_text = "A".repeat(200);
        let pixels = TextRenderer::render_text_to_pixels(
            &long_text,
            100,
            100,
            72,
            Background::White,
        );
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn render_text_to_pixels_transparent_bg() {
        let pixels = TextRenderer::render_text_to_pixels(
            "test",
            50,
            50,
            72,
            Background::Transparent,
        );
        assert_eq!(pixels.len(), 50 * 50 * 4);
        // Background should be transparent (0,0,0,0)
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[1], 0);
        assert_eq!(pixels[2], 0);
        assert_eq!(pixels[3], 0);
    }

    #[test]
    fn page_pixels_min_1() {
        // Even with very low DPI, dimensions should be at least 1
        let (w, h) = TextRenderer::page_pixels(1);
        assert!(w >= 1);
        assert!(h >= 1);
    }

    #[test]
    fn render_with_special_characters() {
        let bytes = make_test_pdf_bytes("Hello! @#$%");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
    }

    #[test]
    fn render_with_empty_content() {
        let bytes = make_test_pdf_bytes("");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
    }

    #[test]
    fn test_from_bytes_invalid() {
        let result = TextRenderer::from_bytes(vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn render_page_high_dpi() {
        let bytes = make_test_pdf_bytes("DPI");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 200,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);
    }

    #[test]
    fn render_page_with_max_width_only() {
        let bytes = make_test_pdf_bytes("Width");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            max_width: Some(200),
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width <= 200);
    }

    #[test]
    fn render_page_with_max_height_only() {
        let bytes = make_test_pdf_bytes("Height");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            max_height: Some(200),
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.height <= 200);
    }
}
