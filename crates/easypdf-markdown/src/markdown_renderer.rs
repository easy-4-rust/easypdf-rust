//! 确定性 Markdown 渲染器。

use easypdf_model::{PdfBlock, PdfDocumentModel};

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
            PdfBlock::Image { alt, target, .. } => match self.image_policy {
                ImagePolicy::Ignore => None,
                ImagePolicy::Reference | ImagePolicy::ExtractTo(_) => Some(format!(
                    "![{}]({})",
                    escape_text(alt),
                    escape_target(target)
                )),
            },
            _ => None,
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

fn render_list(ordered: bool, items: &[String], profile: MarkdownProfile) -> String {
    if profile == MarkdownProfile::Plain {
        return items.join("\n");
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            if ordered {
                format!("{}. {}", index + 1, escape_text(item))
            } else {
                format!("- {}", escape_text(item))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
