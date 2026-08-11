//! Table detection configuration.

/// Column separator strategy for heuristic table detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnSeparator {
    /// Pipe character (`|`) separator — requires at least two `|` in the line.
    Pipe,
    /// Tab character separator.
    Tab,
    /// Two-or-more consecutive spaces treated as column boundary.
    Whitespace,
    /// Try all strategies in order: Pipe, Tab, Whitespace.
    #[default]
    Auto,
}

/// Configuration for the heuristic table detector.
#[derive(Debug, Clone)]
pub struct TableDetectionConfig {
    /// Minimum number of columns for a line to qualify as a table row (default: 2).
    pub min_columns: usize,
    /// Minimum number of rows (including header) for a region to be a table (default: 2).
    pub min_rows: usize,
    /// Column separator strategy.
    pub separator: ColumnSeparator,
    /// When `true`, allow rows with varying column counts.
    pub allow_irregular: bool,
}

impl Default for TableDetectionConfig {
    fn default() -> Self {
        Self {
            min_columns: 2,
            min_rows: 2,
            separator: ColumnSeparator::Auto,
            allow_irregular: false,
        }
    }
}

impl TableDetectionConfig {
    /// Create a new config with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the minimum column count.
    #[must_use]
    pub const fn with_min_columns(mut self, min: usize) -> Self {
        self.min_columns = min;
        self
    }

    /// Set the minimum row count (including header).
    #[must_use]
    pub const fn with_min_rows(mut self, min: usize) -> Self {
        self.min_rows = min;
        self
    }

    /// Set the column separator strategy.
    #[must_use]
    pub const fn with_separator(mut self, separator: ColumnSeparator) -> Self {
        self.separator = separator;
        self
    }

    /// Allow rows with varying column counts.
    #[must_use]
    pub const fn allow_irregular(mut self) -> Self {
        self.allow_irregular = true;
        self
    }
}
