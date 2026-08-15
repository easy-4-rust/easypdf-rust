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

fn table_row(cells: &[String]) -> String {
    let cells = cells
        .iter()
        .map(|cell| cell.replace('|', "\\|").replace('\n', "<br>"))
        .collect::<Vec<_>>();
    format!("| {} |", cells.join(" | "))
}

pub(super) fn render_plain_table(headers: &[String], rows: &[Vec<String>]) -> String {
    std::iter::once(headers)
        .chain(rows.iter().map(Vec::as_slice))
        .filter(|row| !row.is_empty())
        .map(|row| row.join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
}
