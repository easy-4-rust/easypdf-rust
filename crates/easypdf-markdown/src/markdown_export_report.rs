//! Markdown 转换报告。

use crate::MarkdownWarning;

/// PDF 到 Markdown 转换的统计与警告。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkdownExportReport {
    pages_read: usize,
    blocks_written: usize,
    bytes_written: usize,
    warnings: Vec<MarkdownWarning>,
}

impl MarkdownExportReport {
    pub(crate) fn new(
        pages_read: usize,
        blocks_written: usize,
        bytes_written: usize,
        warnings: Vec<MarkdownWarning>,
    ) -> Self {
        Self {
            pages_read,
            blocks_written,
            bytes_written,
            warnings,
        }
    }

    /// 返回已处理页面数。
    #[must_use]
    pub const fn pages_read(&self) -> usize {
        self.pages_read
    }

    /// 返回已输出语义块数。
    #[must_use]
    pub const fn blocks_written(&self) -> usize {
        self.blocks_written
    }

    /// 返回最终 Markdown 字节数。
    #[must_use]
    pub const fn bytes_written(&self) -> usize {
        self.bytes_written
    }

    /// 返回转换警告。
    #[must_use]
    pub fn warnings(&self) -> &[MarkdownWarning] {
        &self.warnings
    }
}
