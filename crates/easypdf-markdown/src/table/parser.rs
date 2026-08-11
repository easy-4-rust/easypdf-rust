//! Cell-level line parsers for each separator strategy.

/// Parse a pipe-separated table line into cells.
///
/// Returns `None` if the line does not contain at least two unescaped `|`
/// characters, or if the line is a Markdown separator row (e.g. `|---|---|`).
///
/// Handles leading and trailing `|` as delimiters (Markdown table style):
/// `| a | b |` yields `["a", "b"]`.  Escaped `\|` within cell content is
/// unescaped to a literal `|`.
pub(crate) fn parse_pipe_separated(text: &str) -> Option<Vec<String>> {
    // Character-level scan that respects `\|` escapes.
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let mut pipe_count: usize = 0;

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            // Escaped pipe: push the backslash, the '|' is consumed by the peek.
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

    // Need at least 2 pipes to form a table row (`|a|b|` pattern).
    if pipe_count < 2 {
        return None;
    }

    // Trim leading / trailing empty segments caused by outer `|` delimiters.
    let start = usize::from(segments.first().is_some_and(String::is_empty));
    let end = if segments.last().is_some_and(String::is_empty) {
        segments.len() - 1
    } else {
        segments.len()
    };

    if start >= end {
        return None;
    }

    let cells: Vec<String> = segments[start..end]
        .iter()
        .map(|s| clean_cell(s))
        .collect();

    // Skip separator rows like `|---|---|` or `| --- | --- |`.
    if cells.iter().all(|c| is_separator_cell(c)) {
        return None;
    }

    if cells.len() < 2 {
        return None;
    }

    Some(cells)
}

/// Parse a tab-separated table line into cells.
pub(crate) fn parse_tab_separated(text: &str) -> Option<Vec<String>> {
    let cells: Vec<String> = text.split('\t').map(clean_cell).collect();
    let non_empty = cells.iter().filter(|c| !c.is_empty()).count();
    if non_empty < 2 {
        return None;
    }
    Some(cells)
}

/// Parse a whitespace-aligned table line into cells.
///
/// Two or more consecutive spaces are treated as a column boundary.
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
    // Trailing spaces — ignore.

    if boundaries.is_empty() {
        return None;
    }

    let mut cells = Vec::new();
    let mut last_end = 0;

    for &boundary in &boundaries {
        let segment = &text[last_end..boundary];
        cells.push(clean_cell(segment));
        last_end = boundary;
        // Skip the spaces to find the next content start.
        while last_end < text.len() && text.as_bytes().get(last_end) == Some(&b' ') {
            last_end += 1;
        }
    }
    // Remaining text after last boundary.
    if last_end <= text.len() {
        cells.push(clean_cell(&text[last_end..]));
    }

    let non_empty = cells.iter().filter(|c| !c.is_empty()).count();
    if non_empty < 2 {
        return None;
    }

    Some(cells)
}

/// Trim whitespace from a cell and unescape `\|` to `|`.
fn clean_cell(cell: &str) -> String {
    cell.trim().replace("\\|", "|")
}

/// Check if a cell looks like a Markdown table separator (e.g., `---`, `:---:`, `---|`).
fn is_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    // A separator cell contains only `-`, `:`, and whitespace.
    trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ')
}

/// Check if a line is a Markdown table separator row (e.g. `|---|---|` or `| --- | --- |`).
///
/// Returns `true` for lines that contain only `|`, `-`, `:`, and whitespace.
/// Such rows are structural delimiters in Markdown tables, not data.
pub(crate) fn is_separator_line(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Must contain at least one pipe or dash to be a separator candidate.
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
        // Requires at least 2 unescaped pipes.
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
        // Single space should NOT be treated as boundary.
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
