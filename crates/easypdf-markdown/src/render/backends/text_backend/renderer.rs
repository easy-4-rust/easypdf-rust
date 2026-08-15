use std::path::Path;

use easypdf_reader::PdfReader;

use crate::render::config::{Background, RenderConfig};
use crate::render::error::{RenderError, Result};
use crate::render::traits::{PdfRenderer, RenderedImage};

use super::glyph::glyph_for;

// A4 在 72 DPI 下：595 x 842 点。
const A4_WIDTH_PT: f64 = 595.0;
const A4_HEIGHT_PT: f64 = 842.0;

/// 纯 Rust 文本回退渲染器。
///
/// 使用 [`PdfReader`] 打开 PDF，逐页提取文本，并使用内置 5x7 位图字体
/// 将其渲染为简单的光栅图像。输出适用于视觉保真度要求不高的 OCR 流水线。
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
    /// 保留原始 PDF 字节以便每次渲染调用时创建新的 `PdfReader`
    ///（因为 `PdfReader::pages()` 会消耗 `self`）。
    pdf_bytes: Vec<u8>,
    page_count: usize,
}

impl TextRenderer {
    /// 打开 PDF 文件进行基于文本的渲染。
    ///
    /// # Errors
    ///
    /// 当文件无法读取时返回 [`RenderError::Io`]，
    /// 当 PDF 格式错误时返回 [`RenderError::Parse`]。
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

    /// 从内存字节打开 PDF。
    ///
    /// # Errors
    ///
    /// 当字节不是有效的 PDF 时返回 [`RenderError::Parse`]。
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

    /// 提取单页文本（从 0 开始的索引）。
    pub(crate) fn extract_page_text(&self, page_index: usize) -> String {
        PdfReader::from_bytes(self.pdf_bytes.clone())
            .ok()
            .and_then(|r| r.pages(page_index..page_index + 1).extract_text().ok())
            .unwrap_or_default()
    }

    /// 计算给定 DPI 下 A4 页面的像素尺寸。
    pub(crate) fn page_pixels(dpi: u32) -> (u32, u32) {
        let scale = f64::from(dpi) / 72.0;
        let w = f64_to_u32(A4_WIDTH_PT * scale);
        let h = f64_to_u32(A4_HEIGHT_PT * scale);
        (w.max(1), h.max(1))
    }

    /// 将提取的文本渲染到 RGBA 像素缓冲区上。
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

        // 填充背景。
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&bg_color);
        }

        // 缩放因子：72 DPI 时字体为 1x，150 DPI 时约 2x，依此类推。
        let scale = f64_to_u32((f64::from(dpi) / 72.0).max(1.0));
        let glyph_w = 5 * scale + scale; // 5 像素字形 + 1 像素间距
        let glyph_h = 7 * scale + scale; // 7 像素字形 + 1 像素间距
        let line_height = glyph_h + scale;
        let margin = 2 * scale;

        let cols = usize::try_from((width.saturating_sub(margin * 2)) / glyph_w)
            .unwrap_or(1)
            .max(1);
        let rows = usize::try_from((height.saturating_sub(margin * 2)) / line_height)
            .unwrap_or(1)
            .max(1);

        // 将文本拆分为适合页面宽度的行。
        let lines: Vec<&str> = text.lines().collect();
        let mut drawn_rows: usize = 0;

        for line in &lines {
            if drawn_rows >= rows {
                break;
            }

            // 自动换行。
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

            // 空行处理：如果原始行为空，前进一行。
            if line.is_empty() && drawn_rows < rows {
                drawn_rows += 1;
            }
        }

        pixels
    }
}

/// 将 `f64` 转换为 `u32`（饱和模式：负值截断为 0，溢出截断为 `u32::MAX`）。
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn f64_to_u32(value: f64) -> u32 {
    if value.is_sign_negative() {
        0
    } else {
        value.round() as u32 // 有意转换用于 DPI/像素计算；负值已在上方处理。
    }
}

/// 将单个字形绘制到像素缓冲区上。
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
                // 填放缩后的像素块。
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

        // 应用最大尺寸约束。
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

        // 提取此页的文本。
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
