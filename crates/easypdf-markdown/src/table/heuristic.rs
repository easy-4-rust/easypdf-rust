//! Heuristic table region detection.

use easypdf_core::PdfBlock;

use super::config::{ColumnSeparator, TableDetectionConfig};
use super::parser;

/// A detected table region spanning consecutive paragraph blocks.
pub(crate) struct TableRegion {
    /// Index of the last block in the region (inclusive).
    pub end_index: usize,
    /// First row treated as headers.
    pub headers: Vec<String>,
    /// Remaining rows.
    pub rows: Vec<Vec<String>>,
}

/// Try to detect a table region starting at `start` within `blocks`.
///
/// Scans consecutive `PdfBlock::Paragraph` blocks. If enough consecutive
/// lines parse as table rows (with at least `min_rows` total rows and
/// `min_columns` columns), returns a [`TableRegion`].
///
/// Non-paragraph blocks or non-table paragraphs break the scan.
pub(crate) fn detect_table_region(
    blocks: &[PdfBlock],
    start: usize,
    config: &TableDetectionConfig,
) -> Option<TableRegion> {
    // The starting block must be a paragraph — try_strategy will check this.
    // Try each separator strategy to parse the first line.
    let strategies = match config.separator {
        ColumnSeparator::Pipe => &[ColumnSeparator::Pipe][..],
        ColumnSeparator::Tab => &[ColumnSeparator::Tab][..],
        ColumnSeparator::Whitespace => &[ColumnSeparator::Whitespace][..],
        ColumnSeparator::Auto => &[ColumnSeparator::Pipe, ColumnSeparator::Tab, ColumnSeparator::Whitespace][..],
    };

    for &strategy in strategies {
        if let Some(region) = try_strategy(blocks, start, config, strategy) {
            return Some(region);
        }
    }

    None
}

/// Try a single separator strategy.
fn try_strategy(
    blocks: &[PdfBlock],
    start: usize,
    config: &TableDetectionConfig,
    strategy: ColumnSeparator,
) -> Option<TableRegion> {
    let first_text = match &blocks[start] {
        PdfBlock::Paragraph { text, .. } => text.as_str(),
        _ => return None,
    };

    let first_cells = parse_with_strategy(first_text, strategy)?;
    if first_cells.len() < config.min_columns {
        return None;
    }

    let expected_columns = first_cells.len();
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    all_rows.push(first_cells);
    let mut last_block_index = start;

    // Scan forward for consecutive table rows.
    for (offset, block) in blocks.iter().enumerate().skip(start + 1) {
        match block {
            PdfBlock::Paragraph { text, .. } => {
                if let Some(cells) = parse_with_strategy(text, strategy) {
                    if cells.len() < config.min_columns {
                        break;
                    }
                    if !config.allow_irregular && cells.len() != expected_columns {
                        break;
                    }
                    all_rows.push(cells);
                    last_block_index = offset;
                } else if parser::is_separator_line(text) {
                    // Separator row (e.g. `|---|---|`) — skip, don't count.
                    last_block_index = offset;
                } else {
                    // Not a table row — stop scanning.
                    break;
                }
            }
            // Non-paragraph block breaks the table region.
            _ => break,
        }
    }

    if all_rows.len() < config.min_rows {
        return None;
    }

    let headers = all_rows.remove(0);

    Some(TableRegion {
        end_index: last_block_index,
        headers,
        rows: all_rows,
    })
}

/// Parse a single line with the given strategy.
fn parse_with_strategy(text: &str, strategy: ColumnSeparator) -> Option<Vec<String>> {
    match strategy {
        ColumnSeparator::Pipe => parser::parse_pipe_separated(text),
        ColumnSeparator::Tab => parser::parse_tab_separated(text),
        ColumnSeparator::Whitespace => parser::parse_whitespace_aligned(text),
        ColumnSeparator::Auto => unreachable!("Auto should be expanded before calling this"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypdf_core::PageIndex;
    use easypdf_core::SourceLocation;

    fn loc() -> SourceLocation {
        SourceLocation::new(PageIndex::new(0), 1.0)
    }

    fn para(text: &str) -> PdfBlock {
        PdfBlock::paragraph(text, loc())
    }

    fn default_config() -> TableDetectionConfig {
        TableDetectionConfig::default()
    }

    // -- Pipe detection --

    #[test]
    fn pipe_table_detected() {
        let blocks = vec![
            para("| Name | Age |"),
            para("| --- | --- |"),
            para("| Alice | 30 |"),
            para("| Bob | 25 |"),
        ];
        let region = detect_table_region(&blocks, 0, &default_config()).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age"]);
        assert_eq!(region.rows.len(), 2);
        assert_eq!(region.end_index, 3);
    }

    #[test]
    fn pipe_table_no_separator_line() {
        let blocks = vec![
            para("| Name | Age |"),
            para("| Alice | 30 |"),
            para("| Bob | 25 |"),
        ];
        let region = detect_table_region(&blocks, 0, &default_config()).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age"]);
        assert_eq!(region.rows.len(), 2);
    }

    // -- Tab detection --

    #[test]
    fn tab_table_detected() {
        let blocks = vec![
            para("Name\tAge\tCity"),
            para("Alice\t30\tNYC"),
            para("Bob\t25\tLA"),
        ];
        let region = detect_table_region(&blocks, 0, &default_config()).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age", "City"]);
        assert_eq!(region.rows.len(), 2);
    }

    // -- Whitespace detection --

    #[test]
    fn whitespace_table_detected() {
        let blocks = vec![
            para("Name    Age    City"),
            para("Alice   30     NYC"),
            para("Bob     25     LA"),
        ];
        let region = detect_table_region(&blocks, 0, &default_config()).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age", "City"]);
        assert_eq!(region.rows.len(), 2);
    }

    // -- Non-table text not detected --

    #[test]
    fn regular_paragraph_not_detected() {
        let blocks = vec![para("This is a normal sentence with spaces.")];
        assert!(detect_table_region(&blocks, 0, &default_config()).is_none());
    }

    #[test]
    fn single_row_not_detected() {
        let blocks = vec![para("| A | B |")];
        assert!(detect_table_region(&blocks, 0, &default_config()).is_none());
    }

    // -- Min rows / min columns filtering --

    #[test]
    fn below_min_columns_rejected() {
        let config = TableDetectionConfig::new().with_min_columns(3);
        let blocks = vec![
            para("| A | B |"),
            para("| 1 | 2 |"),
        ];
        assert!(detect_table_region(&blocks, 0, &config).is_none());
    }

    #[test]
    fn below_min_rows_rejected() {
        let config = TableDetectionConfig::new().with_min_rows(3);
        let blocks = vec![
            para("| A | B |"),
            para("| 1 | 2 |"),
        ];
        // Only 2 rows (header + 1 data), config requires 3.
        assert!(detect_table_region(&blocks, 0, &config).is_none());
    }

    // -- Irregular tables --

    #[test]
    fn irregular_table_rejected_by_default() {
        let blocks = vec![
            para("| A | B | C |"),
            para("| 1 | 2 |"),       // 2 columns vs 3 — breaks scan
            para("| 4 | 5 | 6 |"),
        ];
        // Default config has allow_irregular = false.
        // The 2-col row breaks the scan after just the header (1 row < min_rows 2).
        assert!(detect_table_region(&blocks, 0, &default_config()).is_none());
    }

    #[test]
    fn irregular_table_allowed() {
        let config = TableDetectionConfig::new().allow_irregular();
        let blocks = vec![
            para("| A | B | C |"),
            para("| 1 | 2 |"),
            para("| 4 | 5 | 6 |"),
        ];
        let region = detect_table_region(&blocks, 0, &config).unwrap();
        assert_eq!(region.rows.len(), 2);
    }

    // -- Non-paragraph blocks break scan --

    #[test]
    fn non_paragraph_breaks_region() {
        let blocks = vec![
            para("| A | B |"),
            PdfBlock::heading(1, "Title", loc()),
            para("| 1 | 2 |"),
        ];
        // Heading at index 1 breaks the scan; only 1 row found (below min_rows=2).
        assert!(detect_table_region(&blocks, 0, &default_config()).is_none());
    }

    // -- Starting mid-blocks --

    #[test]
    fn detection_starting_at_nonzero_index() {
        let blocks = vec![
            para("Some intro text."),
            para("| Name | Age |"),
            para("| Alice | 30 |"),
        ];
        let region = detect_table_region(&blocks, 1, &default_config()).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age"]);
        assert_eq!(region.rows.len(), 1);
        assert_eq!(region.end_index, 2);
    }

    // -- Specific separator config --

    #[test]
    fn pipe_only_config() {
        let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Pipe);
        let blocks = vec![
            para("Name\tAge"),
            para("Alice\t30"),
        ];
        // Tab-separated, but config says Pipe only.
        assert!(detect_table_region(&blocks, 0, &config).is_none());
    }

    #[test]
    fn tab_only_config() {
        let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Tab);
        let blocks = vec![
            para("Name\tAge"),
            para("Alice\t30"),
        ];
        let region = detect_table_region(&blocks, 0, &config).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age"]);
    }
}
