//! 内存 Markdown 转换结果。

use crate::MarkdownExportReport;

/// PDF 到 Markdown 的内存转换结果。
#[derive(Clone, Debug)]
pub struct MarkdownConversionResult {
    markdown: String,
    report: MarkdownExportReport,
}

impl MarkdownConversionResult {
    pub(crate) const fn new(markdown: String, report: MarkdownExportReport) -> Self {
        Self { markdown, report }
    }

    /// 返回生成的 Markdown。
    #[must_use]
    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    /// 消费结果并返回生成的 Markdown。
    #[must_use]
    pub fn into_markdown(self) -> String {
        self.markdown
    }

    /// 返回转换统计与警告。
    #[must_use]
    pub const fn report(&self) -> &MarkdownExportReport {
        &self.report
    }
}

impl std::fmt::Display for MarkdownConversionResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.markdown)
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::MarkdownExportReport;

    fn make_result(markdown: &str) -> MarkdownConversionResult {
        MarkdownConversionResult::new(
            markdown.to_string(),
            MarkdownExportReport::new(1, 5, markdown.len(), vec![]),
        )
    }

    #[test]
    fn markdown_returns_content() {
        let result = make_result("# Hello\n\nWorld");
        assert_eq!(result.markdown(), "# Hello\n\nWorld");
    }

    #[test]
    fn into_markdown_consumes_and_returns() {
        let result = make_result("# Title");
        let md = result.into_markdown();
        assert_eq!(md, "# Title");
    }

    #[test]
    fn report_returns_reference() {
        let result = make_result("content");
        let report = result.report();
        assert_eq!(report.pages_read(), 1);
        assert_eq!(report.blocks_written(), 5);
    }

    #[test]
    fn display_trait_outputs_markdown() {
        let result = make_result("# Hello");
        assert_eq!(format!("{}", result), "# Hello");
    }

    #[test]
    fn clone_preserves_content() {
        let result = make_result("clone me");
        let cloned = result.clone();
        assert_eq!(result.markdown(), cloned.markdown());
    }

    #[test]
    fn debug_format() {
        let result = make_result("debug");
        let dbg = format!("{:?}", result);
        assert!(dbg.contains("MarkdownConversionResult"));
    }
}
