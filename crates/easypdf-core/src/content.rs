//! PDF 元素的内容模型类型——文本、表格、图片和形状。

use crate::enums::{TextAlignment, VerticalAlignment};
use crate::style::{PdfColor, PdfFont};

// --- 文本 ---

/// 带格式的定位文本块。
#[derive(Debug, Clone)]
pub struct PdfText {
    /// 要渲染的文本字符串。
    pub content: String,
    /// 文本块内的水平对齐方式。
    pub alignment: TextAlignment,
    /// 此文本的字体规格。
    pub font: PdfFont,
    /// 文本颜色。
    pub color: PdfColor,
}

impl PdfText {
    /// 使用给定内容创建新的文本元素。
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            alignment: TextAlignment::default(),
            font: PdfFont::default(),
            color: PdfColor::default(),
        }
    }

    /// 设置此文本的字体。
    #[must_use]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.font = font;
        self
    }

    /// 设置此文本的对齐方式。
    #[must_use]
    pub const fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// 设置此文本的颜色。
    #[must_use]
    pub const fn color(mut self, color: PdfColor) -> Self {
        self.color = color;
        self
    }
}

// --- 表格 ---

/// 要在 PDF 中渲染的表格配置。
#[derive(Debug, Clone)]
pub struct PdfTable {
    /// 表头。
    pub headers: Vec<String>,
    /// 行数据（每行为字符串值的向量）。
    pub rows: Vec<Vec<String>>,
    /// 列宽（PDF 点）。为空时列均匀分配。
    pub column_widths: Vec<f64>,
    /// 表格总宽度（PDF 点）。
    pub width: f64,
}

impl PdfTable {
    /// 使用给定表头创建新表格。
    #[must_use]
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
            column_widths: Vec::new(),
            width: 0.0,
        }
    }

    /// 向表格添加一行数据。
    #[must_use]
    pub fn row(mut self, row: Vec<String>) -> Self {
        self.rows.push(row);
        self
    }

    /// 向表格添加多行数据。
    #[must_use]
    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// 设置表格宽度。
    #[must_use]
    pub const fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
}

// --- 表格单元格 ---

/// 表格中的单个单元格。
#[derive(Debug, Clone, Default)]
pub struct PdfTableCell {
    /// 单元格文本内容。
    pub content: String,
    /// 单元格内的水平对齐方式。
    pub h_alignment: TextAlignment,
    /// 单元格内的垂直对齐方式。
    pub v_alignment: VerticalAlignment,
    /// 字体规格。
    pub font: PdfFont,
    /// 文本颜色。
    pub color: PdfColor,
}

// --- 图片 ---

/// 要嵌入 PDF 的图片。
#[derive(Debug, Clone)]
pub struct PdfImage {
    /// 原始图片字节（PNG、JPEG 等——格式自动检测）。
    pub data: Vec<u8>,
    /// 期望宽度（PDF 点，0 = 按 72 DPI 使用原始尺寸）。
    pub width: f64,
    /// 期望高度（PDF 点，0 = 按 72 DPI 使用原始尺寸）。
    pub height: f64,
}

impl PdfImage {
    /// 从原始字节创建图片。
    #[must_use]
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data,
            width: 0.0,
            height: 0.0,
        }
    }

    /// 从文件路径创建图片。
    ///
    /// # Errors
    ///
    /// 文件无法读取时返回 `PdfError::Io`。
    pub fn from_path(path: impl AsRef<std::path::Path>) -> crate::error::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self::from_bytes(data))
    }
}

// --- 形状 ---

/// 线段。
#[derive(Debug, Clone, Copy)]
pub struct PdfLine {
    /// 起点 x 坐标。
    pub x1: f64,
    /// 起点 y 坐标。
    pub y1: f64,
    /// 终点 x 坐标。
    pub x2: f64,
    /// 终点 y 坐标。
    pub y2: f64,
    /// 线宽（PDF 点）。
    pub width: f64,
    /// 线条颜色。
    pub color: PdfColor,
}

/// 矩形。
#[derive(Debug, Clone, Copy)]
pub struct PdfRect {
    /// 左下角 x。
    pub x: f64,
    /// 左下角 y。
    pub y: f64,
    /// 宽度。
    pub w: f64,
    /// 高度。
    pub h: f64,
    /// 边框宽度（0 = 无边框）。
    pub border_width: f64,
    /// 边框颜色。
    pub border_color: PdfColor,
    /// 填充颜色（`None` 时透明）。
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
        let t = PdfTable::new(vec!["A".into()]).row(vec!["1".into()]);
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn pdf_table_rows() {
        let t = PdfTable::new(vec!["A".into()]).rows(vec![vec!["1".into()], vec!["2".into()]]);
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
        let cell = PdfTableCell {
            content: "x".into(),
            ..Default::default()
        };
        let cloned = cell.clone();
        assert_eq!(cell.content, cloned.content);
        let _ = format!("{:?}", cell);
    }

    #[test]
    fn pdf_line_debug_copy() {
        let line = PdfLine {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
            width: 1.0,
            color: PdfColor::default(),
        };
        let copied = line;
        assert_eq!(line.x2, copied.x2);
        let _ = format!("{:?}", line);
    }

    #[test]
    fn pdf_rect_debug_copy() {
        let rect = PdfRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 50.0,
            border_width: 1.0,
            border_color: PdfColor::default(),
            fill_color: None,
        };
        let copied = rect;
        assert_eq!(rect.w, copied.w);
        let _ = format!("{:?}", rect);
    }
}
