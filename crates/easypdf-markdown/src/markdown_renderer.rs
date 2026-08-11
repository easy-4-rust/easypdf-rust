//! 确定性 Markdown 渲染器。

use easypdf_core::{ListItem, PdfBlock, PdfDocumentModel};

use crate::{ImagePolicy, MarkdownProfile, TablePolicy};

/// 将语义文档模型渲染为 Markdown 字符串。
#[derive(Clone, Debug)]
pub struct MarkdownRenderer {
    profile: MarkdownProfile,
    table_policy: TablePolicy,
    image_policy: ImagePolicy,
}

impl MarkdownRenderer {
    /// 创建指定配置档的渲染器。
    #[must_use]
    pub fn new(profile: MarkdownProfile) -> Self {
        Self {
            profile,
            table_policy: TablePolicy::default(),
            image_policy: ImagePolicy::default(),
        }
    }

    /// 设置表格输出策略。
    #[must_use]
    pub const fn with_table_policy(mut self, policy: TablePolicy) -> Self {
        self.table_policy = policy;
        self
    }

    /// 设置图片输出策略。
    #[must_use]
    pub fn with_image_policy(mut self, policy: ImagePolicy) -> Self {
        self.image_policy = policy;
        self
    }

    /// 将文档模型渲染为确定性字符串。
    #[must_use]
    pub fn render(&self, document: &PdfDocumentModel) -> String {
        let mut output = String::new();
        if let Some(title) = document.metadata().title.as_deref() {
            match self.profile {
                MarkdownProfile::Plain => push_section(&mut output, title),
                MarkdownProfile::Gfm | MarkdownProfile::Llm => {
                    push_section(&mut output, &format!("# {}", escape_text(title)));
                }
            }
        }

        for page in document.pages() {
            match self.profile {
                MarkdownProfile::Gfm => push_section(
                    &mut output,
                    &format!("<!-- page: {} -->", page.number().value()),
                ),
                MarkdownProfile::Llm => {
                    push_section(&mut output, &format!("## Page {}", page.number().value()));
                }
                MarkdownProfile::Plain => {}
            }
            for block in page.blocks() {
                if let Some(rendered) = self.render_block(block) {
                    push_section(&mut output, &rendered);
                }
            }
        }
        output
    }

    fn render_block(&self, block: &PdfBlock) -> Option<String> {
        match block {
            PdfBlock::Heading { level, text, .. } => match self.profile {
                MarkdownProfile::Plain => Some(text.clone()),
                MarkdownProfile::Gfm | MarkdownProfile::Llm => Some(format!(
                    "{} {}",
                    "#".repeat(usize::from((*level).clamp(1, 6))),
                    escape_text(text)
                )),
            },
            PdfBlock::Paragraph { text, .. } => Some(normalize_text(text)),
            PdfBlock::List { ordered, items, .. } => {
                Some(render_list(*ordered, items, self.profile))
            }
            PdfBlock::Table { headers, rows, .. } => self.render_table(headers, rows),
            PdfBlock::Image { data, .. } => match self.image_policy {
                ImagePolicy::Ignore => None,
                ImagePolicy::Reference | ImagePolicy::ExtractTo(_) => {
                    let alt = data.alt_text().unwrap_or("image");
                    Some(format!("![{}]({})", escape_text(alt), escape_target("image")))
                }
            },
            PdfBlock::Code { language, text, .. } => {
                let lang = language.as_deref().unwrap_or("");
                Some(format!("```{lang}\n{text}\n```"))
            }
            PdfBlock::Formula { latex, .. } => Some(format!("$${latex}$$")),
            PdfBlock::BlockQuote { text, .. } => {
                let quoted = text
                    .lines()
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(quoted)
            }
            PdfBlock::HorizontalRule { .. } => Some("---".to_string()),
            PdfBlock::Link { url, text, .. } => {
                Some(format!("[{}]({})", escape_text(text), escape_target(url)))
            }
            PdfBlock::Footnote {
                reference_id,
                text,
                ..
            } => Some(format!("[^{reference_id}]: {text}")),
            PdfBlock::PageBreak { .. } => match self.profile {
                MarkdownProfile::Plain => None,
                MarkdownProfile::Gfm | MarkdownProfile::Llm => Some("---".to_string()),
            },
            PdfBlock::TableCell {
                row_span,
                col_span,
                text,
                ..
            } => {
                // 独立 TableCell 块渲染为纯文本；合并单元格用 HTML 兜底。
                if *row_span > 1 || *col_span > 1 {
                    Some(format!(
                        "<td rowspan=\"{row_span}\" colspan=\"{col_span}\">{text}</td>"
                    ))
                } else {
                    Some(text.clone())
                }
            }
            // Unknown 及未来变体不渲染（`#[non_exhaustive]` 要求通配分支）。
            PdfBlock::Unknown { .. } | _ => None,
        }
    }

    fn render_table(&self, headers: &[String], rows: &[Vec<String>]) -> Option<String> {
        match self.table_policy {
            TablePolicy::Ignore => None,
            TablePolicy::PlainText => Some(render_plain_table(headers, rows)),
            TablePolicy::Detect => match self.profile {
                MarkdownProfile::Plain => Some(render_plain_table(headers, rows)),
                MarkdownProfile::Gfm | MarkdownProfile::Llm => {
                    Some(render_gfm_table(headers, rows))
                }
            },
        }
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new(MarkdownProfile::default())
    }
}

fn push_section(output: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(value.trim());
    output.push('\n');
}

fn normalize_text(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n")
}

fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_target(target: &str) -> String {
    target.replace(' ', "%20").replace(')', "%29")
}

fn render_list(ordered: bool, items: &[ListItem], profile: MarkdownProfile) -> String {
    if profile == MarkdownProfile::Plain {
        return items
            .iter()
            .map(|i| i.text().to_owned())
            .collect::<Vec<_>>()
            .join("\n");
    }
    let mut lines = Vec::new();
    render_list_items(items, ordered, 0, &mut lines);
    lines.join("\n")
}

/// Recursively render nested list items.
fn render_list_items(items: &[ListItem], ordered: bool, depth: usize, out: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    for (index, item) in items.iter().enumerate() {
        let prefix = if ordered {
            format!("{}{}. ", indent, index + 1)
        } else {
            format!("{indent}- ")
        };
        out.push(format!("{prefix}{}", escape_text(item.text())));
        if !item.children().is_empty() {
            render_list_items(item.children(), ordered, depth + 1, out);
        }
    }
}

fn render_gfm_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return render_plain_table(headers, rows);
    }
    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(table_row(headers));
    lines.push(format!("| {} |", vec!["---"; headers.len()].join(" | ")));
    lines.extend(rows.iter().map(|row| table_row(row)));
    lines.join("\n")
}

fn table_row(cells: &[String]) -> String {
    let cells = cells
        .iter()
        .map(|cell| cell.replace('|', "\\|").replace('\n', "<br>"))
        .collect::<Vec<_>>();
    format!("| {} |", cells.join(" | "))
}

fn render_plain_table(headers: &[String], rows: &[Vec<String>]) -> String {
    std::iter::once(headers)
        .chain(rows.iter().map(Vec::as_slice))
        .filter(|row| !row.is_empty())
        .map(|row| row.join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
}
#[cfg(test)]
#[allow(
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::single_char_pattern,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;
    use easypdf_core::{ListItem, PdfBlock, PdfDocumentModel, PdfMetadata, PdfPageModel, SourceLocation, PageIndex, ImageData, ImageFormat};

    fn loc(page: usize) -> SourceLocation {
        SourceLocation::new(PageIndex::new(page), 1.0)
    }

    fn make_doc_with_blocks() -> PdfDocumentModel {
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "Title", loc(0)))
            .with_block(PdfBlock::paragraph("Hello world", loc(0)))
            .with_block(PdfBlock::code("fn main() {}", loc(0)))
            .with_block(PdfBlock::formula("E=mc^2", loc(0)))
            .with_block(PdfBlock::horizontal_rule(loc(0)))
            .with_block(PdfBlock::PageBreak { source: loc(0) });
        PdfDocumentModel::new(PdfMetadata::default(), vec![page])
    }

    fn make_doc_with_title() -> PdfDocumentModel {
        let meta = PdfMetadata::default().title("My Document");
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("Body text", loc(0)));
        PdfDocumentModel::new(meta, vec![page])
    }

    // --- Constructor tests ---

    #[test]
    fn new_creates_renderer() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let doc = PdfDocumentModel::default();
        let output = renderer.render(&doc);
        assert!(output.is_empty());
    }

    #[test]
    fn default_creates_gfm_renderer() {
        let renderer = MarkdownRenderer::default();
        let doc = make_doc_with_blocks();
        let output = renderer.render(&doc);
        assert!(output.contains("# Title"));
    }

    #[test]
    fn with_table_policy() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_table_policy(TablePolicy::Ignore);
        let headers = vec!["A".to_string(), "B".to_string()];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(!output.contains("| A |"));
    }

    #[test]
    fn with_image_policy_ignore() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_image_policy(ImagePolicy::Ignore);
        let data = ImageData::new(ImageFormat::Png).with_alt_text("logo");
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::image(data, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(!output.contains("!["));
    }

    #[test]
    fn with_image_policy_reference() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_image_policy(ImagePolicy::Reference);
        let data = ImageData::new(ImageFormat::Png).with_alt_text("logo");
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::image(data, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("![logo]"));
    }

    // --- Profile tests ---

    #[test]
    fn gfm_profile_renders_page_markers() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("<!-- page: 1 -->"));
    }

    #[test]
    fn llm_profile_renders_page_headers() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Llm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("## Page 1"));
    }

    #[test]
    fn plain_profile_no_page_markers() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(!output.contains("<!-- page"));
        assert!(!output.contains("## Page"));
    }

    // --- Block rendering tests ---

    #[test]
    fn render_heading_gfm() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(2, "Section", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("## Section"));
    }

    #[test]
    fn render_heading_plain() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "Title", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("Title"));
        assert!(!output.contains("# "));
    }

    #[test]
    fn render_paragraph() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("Hello world", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn render_code_block() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::code("println!(\"hi\");", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("```"));
        assert!(output.contains("println!(\"hi\");"));
    }

    #[test]
    fn render_formula() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::formula("x^2", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("$$x^2$$"));
    }

    #[test]
    fn render_horizontal_rule() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::horizontal_rule(loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("---"));
    }

    #[test]
    fn render_horizontal_rule_plain_profile() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::horizontal_rule(loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("---"));
    }

    #[test]
    fn render_list_unordered_gfm() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let items = vec![
            ListItem::new("Item 1"),
            ListItem::new("Item 2"),
        ];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::list(false, items, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("- Item 1"));
        assert!(output.contains("- Item 2"));
    }

    #[test]
    fn render_list_ordered_gfm() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let items = vec![
            ListItem::new("First"),
            ListItem::new("Second"),
        ];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::list(true, items, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("1. First"));
        assert!(output.contains("2. Second"));
    }

    #[test]
    fn render_list_plain_profile() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let items = vec![
            ListItem::new("A"),
            ListItem::new("B"),
        ];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::list(false, items, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("A"));
        assert!(output.contains("B"));
        assert!(!output.contains("- "));
    }

    #[test]
    fn render_list_nested() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let child = ListItem::new("Child");
        let parent = ListItem::new("Parent").with_child(child);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::list(false, vec![parent], loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("- Parent"));
        assert!(output.contains("  - Child"));
    }

    // --- Table rendering tests ---

    #[test]
    fn render_table_gfm_detect() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_table_policy(TablePolicy::Detect);
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let rows = vec![vec!["Alice".to_string(), "30".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("| Name |"));
        assert!(output.contains("| --- |"));
        assert!(output.contains("| Alice |"));
    }

    #[test]
    fn render_table_plain_text_policy() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_table_policy(TablePolicy::PlainText);
        let headers = vec!["A".to_string()];
        let rows = vec![vec!["1".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("A"));
        assert!(output.contains("1"));
    }

    #[test]
    fn render_table_plain_profile() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain)
            .with_table_policy(TablePolicy::Detect);
        let headers = vec!["X".to_string()];
        let rows = vec![vec!["Y".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("X"));
    }

    // --- Title rendering tests ---

    #[test]
    fn gfm_title_with_hash() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let doc = make_doc_with_title();
        let output = renderer.render(&doc);
        assert!(output.contains("# My Document"));
    }

    #[test]
    fn llm_title_with_hash() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Llm);
        let doc = make_doc_with_title();
        let output = renderer.render(&doc);
        assert!(output.contains("# My Document"));
    }

    #[test]
    fn plain_title_no_hash() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let doc = make_doc_with_title();
        let output = renderer.render(&doc);
        assert!(output.contains("My Document"));
        assert!(!output.contains("# My Document"));
    }

    // --- Blockquote ---

    #[test]
    fn render_blockquote() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::block_quote("quoted text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("> quoted text"));
    }

    // --- Link ---

    #[test]
    fn render_link() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::link("https://example.com", "Example", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("[Example]"));
        assert!(output.contains("https://example.com"));
    }

    // --- Footnote ---

    #[test]
    fn render_footnote() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::footnote("fn1", "This is a footnote", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("[^fn1]: This is a footnote"));
    }

    // --- PageBreak ---

    #[test]
    fn render_page_break_gfm() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::PageBreak { source: loc(0) });
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("---"));
    }

    #[test]
    fn render_page_break_plain() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::PageBreak { source: loc(0) });
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        // Plain profile skips page breaks
        assert!(output.is_empty() || !output.contains("---"));
    }

    // --- TableCell ---

    #[test]
    fn render_table_cell_simple() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table_cell(1, 1, "cell text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("cell text"));
    }

    #[test]
    fn render_table_cell_merged() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        // table_cell(row_span, col_span, text, source)
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table_cell(3, 2, "merged", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("rowspan=\"3\""));
        assert!(output.contains("colspan=\"2\""));
    }

    // --- Multiple pages ---

    #[test]
    fn render_multiple_pages() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page0 = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("Page 1 text", loc(0)));
        let page1 = PdfPageModel::new(PageIndex::new(1))
            .with_block(PdfBlock::paragraph("Page 2 text", loc(1)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page0, page1]);
        let output = renderer.render(&doc);
        assert!(output.contains("<!-- page: 1 -->"));
        assert!(output.contains("<!-- page: 2 -->"));
        assert!(output.contains("Page 1 text"));
        assert!(output.contains("Page 2 text"));
    }

    // --- Empty doc ---

    #[test]
    fn render_empty_doc() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let doc = PdfDocumentModel::default();
        let output = renderer.render(&doc);
        assert!(output.is_empty());
    }

    // --- Additional coverage tests ---

    #[test]
    fn render_image_extract_to_policy() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_image_policy(ImagePolicy::ExtractTo("/tmp".into()));
        let data = ImageData::new(ImageFormat::Png).with_alt_text("logo");
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::image(data, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("![logo]"));
    }

    #[test]
    fn render_image_no_alt_text() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_image_policy(ImagePolicy::Reference);
        let data = ImageData::new(ImageFormat::Png);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::image(data, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("![image]"));
    }

    #[test]
    fn render_code_with_language() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::code_with_language("rust", "fn main() {}", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("```rust"));
    }

    #[test]
    fn render_code_without_language() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::code("fn main() {}", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("```\nfn main() {}\n```"));
    }

    #[test]
    fn render_blockquote_multiline() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::block_quote("line1\nline2\nline3", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("> line1"));
        assert!(output.contains("> line2"));
        assert!(output.contains("> line3"));
    }

    #[test]
    fn render_heading_level_clamped() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(0, "Zero", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("# Zero"));
    }

    #[test]
    fn render_heading_level_over_6() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(10, "Deep", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        // Clamped to 6
        assert!(output.contains("###### Deep"));
    }

    #[test]
    fn render_table_detect_plain_profile() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain)
            .with_table_policy(TablePolicy::Detect);
        let headers = vec!["A".to_string()];
        let rows = vec![vec!["1".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("A"));
    }

    #[test]
    fn render_gfm_table_empty_headers_falls_back() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_table_policy(TablePolicy::Detect);
        let headers: Vec<String> = vec![];
        let rows = vec![vec!["1".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        // Falls back to plain table
        assert!(output.contains("1"));
    }

    #[test]
    fn render_table_with_pipe_in_cell() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_table_policy(TablePolicy::Detect);
        let headers = vec!["Col".to_string()];
        let rows = vec![vec!["a | b".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        // Pipe should be escaped
        assert!(output.contains("a \\| b"));
    }

    #[test]
    fn render_table_with_newline_in_cell() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm)
            .with_table_policy(TablePolicy::Detect);
        let headers = vec!["Col".to_string()];
        let rows = vec![vec!["line1\nline2".to_string()]];
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table(headers, rows, loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        // Newline should become <br>
        assert!(output.contains("line1<br>line2"));
    }

    #[test]
    fn render_llm_profile_page_header() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Llm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("text", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("## Page 1"));
    }

    #[test]
    fn render_plain_profile_title() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let doc = make_doc_with_title();
        let output = renderer.render(&doc);
        assert!(output.contains("My Document"));
        assert!(!output.contains("#"));
    }

    #[test]
    fn push_section_empty_value() {
        let mut output = String::from("existing");
        push_section(&mut output, "");
        assert_eq!(output, "existing");
    }

    #[test]
    fn push_section_non_empty_to_existing() {
        let mut output = String::from("existing");
        push_section(&mut output, "new");
        assert!(output.contains("existing"));
        assert!(output.contains("new"));
    }

    #[test]
    fn normalize_text_multiline() {
        let result = normalize_text("  line1  \n  line2  ");
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn escape_text_special_chars() {
        let result = escape_text("hello \\world* _test_ [link]");
        assert!(result.contains("\\\\"));
        assert!(result.contains("\\*"));
        assert!(result.contains("\\_"));
        assert!(result.contains("\\["));
        assert!(result.contains("\\]"));
    }

    #[test]
    fn escape_target_spaces_and_parens() {
        let result = escape_target("http://example.com/path (1).pdf");
        assert!(result.contains("%20"));
        assert!(result.contains("%29"));
    }

    #[test]
    fn render_plain_table_with_data() {
        let headers = vec!["A".to_string(), "B".to_string()];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let result = render_plain_table(&headers, &rows);
        assert!(result.contains("A\tB"));
        assert!(result.contains("1\t2"));
    }

    #[test]
    fn render_plain_table_empty_rows() {
        let headers = vec!["A".to_string()];
        let rows: Vec<Vec<String>> = vec![];
        let result = render_plain_table(&headers, &rows);
        assert!(result.contains("A"));
    }

    #[test]
    fn render_table_cell_with_rowspan_1_colspan_1() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table_cell(1, 1, "simple", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("simple"));
        assert!(!output.contains("rowspan"));
    }

    #[test]
    fn render_unknown_block_none() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Plain);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::unknown("", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        // Unknown block produces no output; Plain profile has no page markers
        assert!(output.is_empty());
    }

    #[test]
    fn render_list_ordered_nested() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let child = ListItem::new("Sub");
        let parent = ListItem::new("Main").with_child(child);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::list(true, vec![parent], loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("1. Main"));
        assert!(output.contains("  1. Sub"));
    }

    #[test]
    fn render_table_cell_merged_colspan_only() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table_cell(1, 3, "wide", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("colspan=\"3\""));
    }

    #[test]
    fn render_table_cell_merged_rowspan_only() {
        let renderer = MarkdownRenderer::new(MarkdownProfile::Gfm);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::table_cell(2, 1, "tall", loc(0)));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let output = renderer.render(&doc);
        assert!(output.contains("rowspan=\"2\""));
    }
}
