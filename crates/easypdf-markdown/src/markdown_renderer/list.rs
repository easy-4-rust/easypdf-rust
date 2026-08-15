//! Markdown 列表渲染辅助函数。

use easypdf_core::ListItem;

use crate::MarkdownProfile;

use super::escaping::escape_text;

/// 将列表项渲染为 Markdown 字符串。
pub(super) fn render_list(ordered: bool, items: &[ListItem], profile: MarkdownProfile) -> String {
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

/// 递归渲染嵌套列表项。
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
