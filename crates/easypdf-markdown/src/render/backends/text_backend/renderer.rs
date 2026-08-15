use std::path::Path;

use easypdf_reader::PdfReader;

use crate::render::config::{Background, RenderConfig};
use crate::render::error::{RenderError, Result};
use crate::render::traits::{PdfRenderer, RenderedImage};

use super::glyph::glyph_for;

// A4 at 72 DPI: 595 x 842 points.
const A4_WIDTH_PT: f64 = 595.0;
const A4_HEIGHT_PT: f64 = 842.0;

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
        let reader = PdfReader::from_bytes(pdf_bytes.clone())
            .map_err(|e| RenderError::Parse(e.to_string()))?;
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
    pub(crate) fn extract_page_text(&self, page_index: usize) -> String {
        PdfReader::from_bytes(self.pdf_bytes.clone())
            .ok()
            .and_then(|r| r.pages(page_index..page_index + 1).extract_text().ok())
            .unwrap_or_default()
    }

    /// Compute pixel dimensions for an A4 page at the given DPI.
    pub(crate) fn page_pixels(dpi: u32) -> (u32, u32) {
        let scale = f64::from(dpi) / 72.0;
        let w = f64_to_u32(A4_WIDTH_PT * scale);
        let h = f64_to_u32(A4_HEIGHT_PT * scale);
        (w.max(1), h.max(1))
    }

    /// Render extracted text onto an RGBA pixel buffer.
    pub(crate) fn render_text_to_pixels(
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

        let cols = usize::try_from((width.saturating_sub(margin * 2)) / glyph_w)
            .unwrap_or(1)
            .max(1);
        let rows = usize::try_from((height.saturating_sub(margin * 2)) / line_height)
            .unwrap_or(1)
            .max(1);

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
                    draw_glyph(
                        &mut pixels,
                        width,
                        height,
                        x_offset,
                        y_offset,
                        scale,
                        ch,
                        fg_color,
                    );
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
pub(crate) fn f64_to_u32(value: f64) -> u32 {
    if value.is_sign_negative() {
        0
    } else {
        value.round() as u32 // Cast is intentional for DPI/pixel math; negative handled above.
    }
}

/// Draw a single glyph onto the pixel buffer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_glyph(
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

        let pixels =
            Self::render_text_to_pixels(&text, width, height, config.dpi, config.background);

        Ok(RenderedImage::new(
            width,
            height,
            config.format,
            pixels,
            page_index,
        ))
    }

    fn name(&self) -> &'static str {
        "text"
    }

    fn max_dpi(&self) -> u32 {
        300
    }
}
