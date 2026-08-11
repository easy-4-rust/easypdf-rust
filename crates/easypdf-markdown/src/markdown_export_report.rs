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

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::MarkdownWarning;

    #[test]
    fn default_is_empty() {
        let report = MarkdownExportReport::default();
        assert_eq!(report.pages_read(), 0);
        assert_eq!(report.blocks_written(), 0);
        assert_eq!(report.bytes_written(), 0);
        assert!(report.warnings().is_empty());
    }

    #[test]
    fn new_sets_all_fields() {
        let warnings = vec![MarkdownWarning::ProcessorFailed {
            message: "test failure".to_string(),
        }];
        let report = MarkdownExportReport::new(5, 100, 4096, warnings);
        assert_eq!(report.pages_read(), 5);
        assert_eq!(report.blocks_written(), 100);
        assert_eq!(report.bytes_written(), 4096);
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn clone_preserves_values() {
        let report = MarkdownExportReport::new(1, 2, 3, vec![]);
        let cloned = report.clone();
        assert_eq!(report, cloned);
    }

    #[test]
    fn partial_eq_works() {
        let a = MarkdownExportReport::new(1, 2, 3, vec![]);
        let b = MarkdownExportReport::new(1, 2, 3, vec![]);
        let c = MarkdownExportReport::new(1, 2, 4, vec![]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn debug_format() {
        let report = MarkdownExportReport::new(1, 2, 3, vec![]);
        let dbg = format!("{:?}", report);
        assert!(dbg.contains("MarkdownExportReport"));
    }
}
