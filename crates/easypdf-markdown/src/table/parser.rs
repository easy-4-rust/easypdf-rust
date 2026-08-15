//! 单元格级行解析器，每种分隔策略一个。

/// 解析管道分隔的表格行为单元格。
///
/// 当行中不包含至少两个未转义的 `|` 字符，或行是 Markdown
/// 分隔行（如 `|---|---|`）时返回 `None`。
///
/// 处理前导和尾部 `|` 作为分隔符（Markdown 表格风格）：
/// `| a | b |` 产生 `["a", "b"]`。单元格内容中转义的 `\|`
/// 会被反转义为字面 `|`。
pub(crate) fn parse_pipe_separated(text: &str) -> Option<Vec<String>> {
    // 字符级扫描，尊重 `\|` 转义。
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let mut pipe_count: usize = 0;

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            // 转义管道符：推入反斜杠，`|` 被 peek 消耗。
            current.push(ch);
            current.push(chars.next().unwrap_or('|'));
        } else if ch == '|' {
            pipe_count += 1;
            segments.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    segments.push(current);

    // 至少需要 2 个管道符才能形成表格行（`|a|b|` 模式）。
    if pipe_count < 2 {
        return None;
    }

    // 修剪由外部 `|` 分隔符引起的前导/尾部空段。
    let start = usize::from(segments.first().is_some_and(String::is_empty));
    let end = if segments.last().is_some_and(String::is_empty) {
        segments.len() - 1
    } else {
        segments.len()
    };

    if start >= end {
        return None;
    }

    let cells: Vec<String> = segments[start..end].iter().map(|s| clean_cell(s)).collect();

    // 跳过分隔行如 `|---|---|` 或 `| --- | --- |`。
    if cells.iter().all(|c| is_separator_cell(c)) {
        return None;
    }

    if cells.len() < 2 {
        return None;
    }

    Some(cells)
}

/// 解析制表符分隔的表格行为单元格。
pub(crate) fn parse_tab_separated(text: &str) -> Option<Vec<String>> {
    let cells: Vec<String> = text.split('\t').map(clean_cell).collect();
    let non_empty = cells.iter().filter(|c| !c.is_empty()).count();
    if non_empty < 2 {
        return None;
    }
    Some(cells)
}

/// 解析空格对齐的表格行为单元格。
///
/// 两个或更多连续空格视为列边界。
pub(crate) fn parse_whitespace_aligned(text: &str) -> Option<Vec<String>> {
    let mut boundaries = Vec::new();
    let mut consecutive_spaces: usize = 0;
    let mut boundary_start: usize = 0;

    for (i, ch) in text.chars().enumerate() {
        if ch == ' ' {
            if consecutive_spaces == 0 {
                boundary_start = i;
            }
            consecutive_spaces += 1;
        } else {
            if consecutive_spaces >= 2 {
                boundaries.push(boundary_start);
            }
            consecutive_spaces = 0;
        }
    }
    // 尾部空格——忽略。

    if boundaries.is_empty() {
        return None;
    }

    let mut cells = Vec::new();
    let mut last_end = 0;

    for &boundary in &boundaries {
        let segment = &text[last_end..boundary];
        cells.push(clean_cell(segment));
        last_end = boundary;
        // 跳过空格以找到下一个内容起始位置。
        while last_end < text.len() && text.as_bytes().get(last_end) == Some(&b' ') {
            last_end += 1;
        }
    }
    // 最后一个边界之后的剩余文本。
    if last_end <= text.len() {
        cells.push(clean_cell(&text[last_end..]));
    }

    let non_empty = cells.iter().filter(|c| !c.is_empty()).count();
    if non_empty < 2 {
        return None;
    }

    Some(cells)
}

/// 修剪单元格空白并将 `\|` 反转义为 `|`。
fn clean_cell(cell: &str) -> String {
    cell.trim().replace("\\|", "|")
}

/// 检查单元格是否像 Markdown 表格分隔符（如 `---`、`:---:`、`---|`）。
fn is_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    // 分隔符单元格仅包含 `-`、`:` 和空白。
    trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ')
}

/// 检查一行是否是 Markdown 表格分隔行（如 `|---|---|` 或 `| --- | --- |`）。
///
/// 对仅包含 `|`、`-`、`:` 和空白的行返回 `true`。
/// 此类行是 Markdown 表格中的结构分隔符，不是数据。
pub(crate) fn is_separator_line(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    // 必须至少包含一个管道符或短横线才可能是分隔行。
    if !trimmed.contains('|') && !trimmed.contains('-') {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_pipe_separated --

    #[test]
    fn pipe_basic() {
        let cells = parse_pipe_separated("| a | b | c |").unwrap();
        assert_eq!(cells, vec!["a", "b", "c"]);
    }

    #[test]
    fn pipe_no_leading_trailing() {
        // 至少需要 2 个未转义的管道符。
        let cells = parse_pipe_separated("a | b | c").unwrap();
        assert_eq!(cells, vec!["a", "b", "c"]);
    }

    #[test]
    fn pipe_separator_row_returns_none() {
        assert!(parse_pipe_separated("| --- | --- |").is_none());
        assert!(parse_pipe_separated("|:---:|:---:|").is_none());
    }

    #[test]
    fn pipe_single_separator_returns_none() {
        assert!(parse_pipe_separated("only one | here").is_none());
    }

    #[test]
    fn pipe_empty_returns_none() {
        assert!(parse_pipe_separated("").is_none());
        assert!(parse_pipe_separated("||").is_none());
    }

    #[test]
    fn pipe_unescapes() {
        let cells = parse_pipe_separated(r"| a \| b | c |").unwrap();
        assert_eq!(cells[0], r"a | b");
        assert_eq!(cells[1], "c");
    }

    // -- parse_tab_separated --

    #[test]
    fn tab_basic() {
        let cells = parse_tab_separated("a\tb\tc").unwrap();
        assert_eq!(cells, vec!["a", "b", "c"]);
    }

    #[test]
    fn tab_single_column_returns_none() {
        assert!(parse_tab_separated("only one").is_none());
    }

    // -- parse_whitespace_aligned --

    #[test]
    fn whitespace_basic() {
        let cells = parse_whitespace_aligned("Name    Age    City").unwrap();
        assert_eq!(cells, vec!["Name", "Age", "City"]);
    }

    #[test]
    fn whitespace_single_space_does_not_split() {
        // 单个空格不应视为边界。
        assert!(parse_whitespace_aligned("Hello World").is_none());
    }

    #[test]
    fn whitespace_two_spaces_splits() {
        let cells = parse_whitespace_aligned("A  B").unwrap();
        assert_eq!(cells, vec!["A", "B"]);
    }

    #[test]
    fn whitespace_single_column_returns_none() {
        assert!(parse_whitespace_aligned("NoColumns").is_none());
    }

    // -- clean_cell --

    #[test]
    fn clean_cell_trims_and_unescapes() {
        assert_eq!(clean_cell("  hello  "), "hello");
        assert_eq!(clean_cell(r"a \| b"), "a | b");
    }

    // -- is_separator_cell --

    #[test]
    fn separator_cell_detection() {
        assert!(is_separator_cell("---"));
        assert!(is_separator_cell(":---:"));
        assert!(is_separator_cell(" --- "));
        assert!(!is_separator_cell("data"));
        assert!(!is_separator_cell(""));
    }
}
