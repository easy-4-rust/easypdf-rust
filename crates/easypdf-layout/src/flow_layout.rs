//! 自动流式布局。

use std::path::Path;

use easypdf_core::{Orientation, PageSize, PdfFont, PdfText, Result};

use crate::{Direction, LayoutSink};

/// 在页面内自动定位内容的流式布局器。
pub struct FlowLayout<S> {
    direction: Direction,
    margins: f64,
    spacing: f64,
    cursor: f64,
    page_width: f64,
    page_height: f64,
    sink: S,
}

impl<S: LayoutSink> FlowLayout<S> {
    /// 创建默认边距的纵向流式布局。
    #[must_use]
    pub fn vertical(sink: S, page_size: PageSize) -> Self {
        let (width, height) = page_size.dimensions();
        Self {
            direction: Direction::Vertical,
            margins: 72.0,
            spacing: 12.0,
            cursor: height - 72.0,
            page_width: width,
            page_height: height,
            sink,
        }
    }

    /// 设置页面边距。
    #[must_use]
    pub fn margins(mut self, margins: f64) -> Self {
        self.margins = margins;
        self.cursor = self.page_height - margins;
        self
    }

    /// 设置元素间距。
    #[must_use]
    pub fn spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing;
        self
    }

    /// 返回布局方向。
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    /// 写入文本并自动推进游标。
    ///
    /// # Errors
    ///
    /// 写入后端失败时返回错误。
    pub fn add_text(&mut self, content: &str, font: &PdfFont, estimated_height: f64) -> Result<()> {
        let y = self.cursor - estimated_height;
        self.sink
            .write_text(&PdfText::new(content).font(font.clone()), self.margins, y)?;
        self.cursor = y - self.spacing;
        Ok(())
    }

    /// 返回当前页面剩余纵向空间。
    #[must_use]
    pub fn remaining_space(&self) -> f64 {
        self.cursor - self.margins
    }

    /// 新增页面并重置游标。
    ///
    /// # Errors
    ///
    /// 写入后端无法创建页面时返回错误。
    pub fn new_page(&mut self) -> Result<()> {
        self.sink.add_page(
            PageSize::Custom(self.page_width, self.page_height),
            Orientation::Portrait,
        )?;
        self.cursor = self.page_height - self.margins;
        Ok(())
    }

    /// 完成布局并保存文档。
    ///
    /// # Errors
    ///
    /// 写入后端无法完成文档时返回错误。
    pub fn finish(self, path: impl AsRef<Path>) -> Result<()> {
        self.sink.finish(path.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use easypdf_core::{Orientation, PageSize, PdfFont, PdfText, Result};

    use super::FlowLayout;
    use crate::LayoutSink;

    #[derive(Default)]
    struct RecordingSink {
        pages: usize,
        texts: Vec<String>,
    }

    impl LayoutSink for RecordingSink {
        fn add_page(&mut self, _size: PageSize, _orientation: Orientation) -> Result<usize> {
            self.pages += 1;
            Ok(self.pages)
        }

        fn write_text(&mut self, text: &PdfText, _x: f64, _y: f64) -> Result<()> {
            self.texts.push(text.content.clone());
            Ok(())
        }

        fn finish(self, _path: &Path) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn lays_out_text_and_new_pages_without_writer_dependency() {
        let mut layout = FlowLayout::vertical(RecordingSink::default(), PageSize::A4)
            .margins(50.0)
            .spacing(10.0);
        layout
            .add_text("Hello", &PdfFont::helvetica(12.0), 20.0)
            .expect("add text");
        assert!(layout.remaining_space() > 0.0);
        layout.new_page().expect("new page");
        layout.finish("unused.pdf").expect("finish");
    }
}
