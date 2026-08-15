//! 启发式表格区域检测。

use easypdf_core::PdfBlock;

use super::config::{ColumnSeparator, TableDetectionConfig};
use super::parser;

/// 跨连续段落块检测到的表格区域。
pub(crate) struct TableRegion {
    /// 区域中最后一个块的索引（含）。
    pub end_index: usize,
    /// 第一行作为表头。
    pub headers: Vec<String>,
    /// 剩余行。
    pub rows: Vec<Vec<String>>,
}

/// 尝试从 `blocks` 中的 `start` 位置开始检测表格区域。
///
/// 扫描连续的 `PdfBlock::Paragraph` 块。如果有足够多的连续行
/// 被解析为表格行（总行数至少 `min_rows`，列数至少 `min_columns`），
/// 则返回 [`TableRegion`]。
///
/// 非段落块或非表格段落会中断扫描。
pub(crate) fn detect_table_region(
    blocks: &[PdfBlock],
    start: usize,
    config: &TableDetectionConfig,
) -> Option<TableRegion> {
    // 起始块必须是段落——try_strategy 会检查这一点。
    // 尝试每种分隔策略来解析第一行。
    let strategies = match config.separator {
        ColumnSeparator::Pipe => &[ColumnSeparator::Pipe][..],
        ColumnSeparator::Tab => &[ColumnSeparator::Tab][..],
        ColumnSeparator::Whitespace => &[ColumnSeparator::Whitespace][..],
        ColumnSeparator::Auto => &[
            ColumnSeparator::Pipe,
            ColumnSeparator::Tab,
            ColumnSeparator::Whitespace,
        ][..],
    };

    for &strategy in strategies {
        if let Some(region) = try_strategy(blocks, start, config, strategy) {
            return Some(region);
        }
    }

    None
}

/// 尝试单一的分隔策略。
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

    // 向前扫描连续的表格行。
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
                    // 分隔行（如 `|---|---|`）——跳过，不计数。
                    last_block_index = offset;
                } else {
                    // 非表格行——停止扫描。
                    break;
                }
            }
            // 非段落块中断表格区域。
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

/// 使用给定策略解析单行。
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

    // -- 管道检测 --

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

    // -- 制表符检测 --

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

    // -- 空格检测 --

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

    // -- 非表格文本不被误检测 --

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

    // -- 最小行数/列数过滤 --

    #[test]
    fn below_min_columns_rejected() {
        let config = TableDetectionConfig::new().with_min_columns(3);
        let blocks = vec![para("| A | B |"), para("| 1 | 2 |")];
        assert!(detect_table_region(&blocks, 0, &config).is_none());
    }

    #[test]
    fn below_min_rows_rejected() {
        let config = TableDetectionConfig::new().with_min_rows(3);
        let blocks = vec![para("| A | B |"), para("| 1 | 2 |")];
        assert!(detect_table_region(&blocks, 0, &config).is_none());
    }

    // -- 不规则表格 --

    #[test]
    fn irregular_table_rejected_by_default() {
        let blocks = vec![
            para("| A | B | C |"),
            para("| 1 | 2 |"), // 2 列 vs 3——中断扫描
            para("| 4 | 5 | 6 |"),
        ];
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

    // -- 非段落块中断扫描 --

    #[test]
    fn non_paragraph_breaks_region() {
        let blocks = vec![
            para("| A | B |"),
            PdfBlock::heading(1, "Title", loc()),
            para("| 1 | 2 |"),
        ];
        assert!(detect_table_region(&blocks, 0, &default_config()).is_none());
    }

    // -- 从非零索引开始 --

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

    // -- 特定分隔策略配置 --

    #[test]
    fn pipe_only_config() {
        let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Pipe);
        let blocks = vec![para("Name\tAge"), para("Alice\t30")];
        assert!(detect_table_region(&blocks, 0, &config).is_none());
    }

    #[test]
    fn tab_only_config() {
        let config = TableDetectionConfig::new().with_separator(ColumnSeparator::Tab);
        let blocks = vec![para("Name\tAge"), para("Alice\t30")];
        let region = detect_table_region(&blocks, 0, &config).unwrap();
        assert_eq!(region.headers, vec!["Name", "Age"]);
    }
}
