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

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::MarkdownExportReport;

    #[test]
    fn output_returns_path() {
        let report = MarkdownExportReport::default();
        let result = MarkdownExportResult::new(PathBuf::from("/tmp/out.md"), report);
        assert_eq!(result.output(), Path::new("/tmp/out.md"));
    }

    #[test]
    fn report_returns_reference() {
        let report = MarkdownExportReport::new(5, 10, 100, vec![]);
        let result = MarkdownExportResult::new(PathBuf::from("/tmp/out.md"), report);
        assert_eq!(result.report().pages_read(), 5);
        assert_eq!(result.report().blocks_written(), 10);
    }

    #[test]
    fn clone_preserves() {
        let report = MarkdownExportReport::default();
        let result = MarkdownExportResult::new(PathBuf::from("/tmp/out.md"), report);
        let cloned = result.clone();
        assert_eq!(result.output(), cloned.output());
    }

    #[test]
    fn debug_format() {
        let report = MarkdownExportReport::default();
        let result = MarkdownExportResult::new(PathBuf::from("/tmp/out.md"), report);
        let dbg = format!("{:?}", result);
        assert!(dbg.contains("MarkdownExportResult"));
    }
}
