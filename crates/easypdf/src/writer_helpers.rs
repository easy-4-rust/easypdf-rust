//! 内置写入处理器与表格渲染辅助函数。

/// 向每页添加页码的写入处理器。
pub struct PageNumberHandler {
    font: easypdf_core::PdfFont,
    /// 距底部居中位置的偏移量，单位为 PDF point。
    offset_y: f64,
}

impl PageNumberHandler {
    /// 创建一个新的页码处理器。
    #[must_use = "builder method"]
    pub fn new() -> Self {
        Self {
            font: easypdf_core::PdfFont::helvetica(10.0),
            offset_y: 30.0,
        }
    }

    /// 设置页码的字体。
    #[must_use = "builder method"]
    pub fn font(mut self, font: easypdf_core::PdfFont) -> Self {
        self.font = font;
        self
    }

    /// 设置距页面底部的 Y 偏移量。
    #[must_use = "builder method"]
    pub fn offset_y(mut self, offset: f64) -> Self {
        self.offset_y = offset;
        self
    }

    /// 返回页码字体。
    #[must_use]
    pub const fn page_number_font(&self) -> &easypdf_core::PdfFont {
        &self.font
    }

    /// 返回距页面底部的偏移量（PDF point）。
    #[must_use]
    pub const fn page_number_offset_y(&self) -> f64 {
        self.offset_y
    }
}

impl Default for PageNumberHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl easypdf_core::PdfWriteHandler for PageNumberHandler {
    fn after_page(&mut self, page_number: usize) -> easypdf_core::Result<()> {
        // 处理器返回 Ok；实际的页码渲染由写入器完成
        let _ = page_number;
        Ok(())
    }
}

/// 使用线条和文本在写入器上渲染表格。
///
/// # 参数
///
/// * `writer` - PDF 写入器。
/// * `table` - 要渲染的表格数据。
/// * `x` - 表格左上角的 X 坐标（PDF point）。
/// * `y` - 表格左上角的 Y 坐标（PDF point）。
/// * `col_widths` - 列宽数组（PDF point），为空时自动计算。
/// * `row_height` - 行高（PDF point）。
/// * `font` - 单元格文本的字体。
///
/// # Errors
///
/// 如果任何写入操作失败，返回错误。
pub fn write_table(
    writer: &mut easypdf_writer::PdfWriter,
    table: &easypdf_core::PdfTable,
    x: f64,
    y: f64,
    col_widths: &[f64],
    row_height: f64,
    font: &easypdf_core::PdfFont,
) -> easypdf_core::Result<()> {
    let ncols = table.headers.len();
    if ncols == 0 {
        return Ok(());
    }

    let widths: Vec<f64> = if col_widths.is_empty() {
        let default_w = 500.0 / ncols as f64;
        vec![default_w; ncols]
    } else {
        col_widths.to_vec()
    };

    // 绘制表头行
    let header_y = y;
    for (i, header) in table.headers.iter().enumerate() {
        let cell_x = x + widths.iter().take(i).sum::<f64>();
        writer.draw_rect_stroke(cell_x, header_y, widths[i], row_height, 0.5);
        let txt = easypdf_core::PdfText::new(header.as_str()).font(font.clone().bold());
        writer.write_text(&txt, cell_x + 4.0, header_y + row_height - font.size - 2.0)?;
    }

    // 绘制数据行
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_y = y - (row_idx as f64 + 1.0) * row_height;
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                let cell_x = x + widths.iter().take(i).sum::<f64>();
                writer.draw_rect_stroke(cell_x, row_y, widths[i], row_height, 0.5);
                let txt = easypdf_core::PdfText::new(cell.as_str()).font(font.clone());
                writer.write_text(&txt, cell_x + 4.0, row_y + row_height - font.size - 2.0)?;
            }
        }
    }
    Ok(())
}
