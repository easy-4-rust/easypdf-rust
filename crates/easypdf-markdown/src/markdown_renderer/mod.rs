//! 确定性 Markdown 渲染器。

mod escaping;
mod list;
mod table;

use easypdf_core::{PdfBlock, PdfDocumentModel};

use crate::{ImagePolicy, MarkdownProfile, TablePolicy};

// Re-export helpers for test visibility.
use escaping::{escape_target, escape_text, normalize_text};
use list::render_list;
use table::{render_gfm_table, render_plain_table};

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
                    Some(format!(
                        "![{}]({})",
                        escape_text(alt),
                        escape_target("image")
                    ))
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
                reference_id, text, ..
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

#[cfg(test)]
#[allow(
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::single_char_pattern,
    clippy::unnecessary_wraps
)]
mod tests;
