//! 布局操作的写入后端接口。

use std::path::Path;

use crate::{Orientation, PageSize, PdfText, Result};

/// 接收中立布局操作的 PDF 写入后端。
pub trait LayoutSink {
    /// 新增页面并返回一基页码。
    ///
    /// # Errors
    ///
    /// 后端无法创建页面时返回错误。
    fn add_page(&mut self, size: PageSize, orientation: Orientation) -> Result<usize>;

    /// 在指定坐标写入文本。
    ///
    /// # Errors
    ///
    /// 后端无法写入文本时返回错误。
    fn write_text(&mut self, text: &PdfText, x: f64, y: f64) -> Result<()>;

    /// 完成文档并保存。
    ///
    /// # Errors
    ///
    /// 后端无法完成或保存文档时返回错误。
    fn finish(self, path: &Path) -> Result<()>
    where
        Self: Sized;
}
