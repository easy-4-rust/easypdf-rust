//! Markdown 转换结果。

use std::path::{Path, PathBuf};

use crate::MarkdownExportReport;

/// 已完成的 Markdown 文件及转换报告。
#[derive(Clone, Debug)]
pub struct MarkdownExportResult {
    output: PathBuf,
    report: MarkdownExportReport,
}

impl MarkdownExportResult {
    pub(crate) const fn new(output: PathBuf, report: MarkdownExportReport) -> Self {
        Self { output, report }
    }

    /// 返回输出文件路径。
    #[must_use]
    pub fn output(&self) -> &Path {
        &self.output
    }

    /// 返回转换报告。
    #[must_use]
    pub const fn report(&self) -> &MarkdownExportReport {
        &self.report
    }
}
