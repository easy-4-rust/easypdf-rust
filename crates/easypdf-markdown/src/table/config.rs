//! 表格检测配置。

/// 启发式表格检测的列分隔策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnSeparator {
    /// 管道符（`|`）分隔——要求行中至少有两个 `|`。
    Pipe,
    /// 制表符分隔。
    Tab,
    /// 两个或更多连续空格视为列边界。
    Whitespace,
    /// 按顺序尝试所有策略：Pipe、Tab、Whitespace。
    #[default]
    Auto,
}

/// 启发式表格检测器的配置。
#[derive(Debug, Clone)]
pub struct TableDetectionConfig {
    /// 一行被视为表格行的最小列数（默认值：2）。
    pub min_columns: usize,
    /// 一个区域被视为表格的最小行数（含表头，默认值：2）。
    pub min_rows: usize,
    /// 列分隔策略。
    pub separator: ColumnSeparator,
    /// 为 `true` 时允许行具有不同的列数。
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
    /// 使用默认值创建新配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最小列数。
    #[must_use]
    pub const fn with_min_columns(mut self, min: usize) -> Self {
        self.min_columns = min;
        self
    }

    /// 设置最小行数（含表头）。
    #[must_use]
    pub const fn with_min_rows(mut self, min: usize) -> Self {
        self.min_rows = min;
        self
    }

    /// 设置列分隔策略。
    #[must_use]
    pub const fn with_separator(mut self, separator: ColumnSeparator) -> Self {
        self.separator = separator;
        self
    }

    /// 允许行具有不同的列数。
    #[must_use]
    pub const fn allow_irregular(mut self) -> Self {
        self.allow_irregular = true;
        self
    }
}
