//! Markdown 表格渲染辅助函数。

/// 渲染 GFM（GitHub Flavored Markdown）表格。
pub(super) fn render_gfm_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return render_plain_table(headers, rows);
    }
    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(table_row(headers));
    lines.push(format!("| {} |", vec!["---"; headers.len()].join(" | ")));
    lines.extend(rows.iter().map(|row| table_row(row)));
    lines.join("\n")
}

/// 渲染单行表格（单元格用 `|` 分隔）。
fn table_row(cells: &[String]) -> String {
    let cells = cells
        .iter()
        .map(|cell| cell.replace('|', "\\|").replace('\n', "<br>"))
        .collect::<Vec<_>>();
    format!("| {} |", cells.join(" | "))
}

/// 渲染纯文本表格（单元格用制表符分隔）。
pub(super) fn render_plain_table(headers: &[String], rows: &[Vec<String>]) -> String {
    std::iter::once(headers)
        .chain(rows.iter().map(Vec::as_slice))
        .filter(|row| !row.is_empty())
        .map(|row| row.join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
}
