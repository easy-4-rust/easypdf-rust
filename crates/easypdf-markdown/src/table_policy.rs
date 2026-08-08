//! PDF 表格转换策略。

/// PDF 表格转换策略。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TablePolicy {
    /// 尝试识别并输出 GFM 表格。
    #[default]
    Detect,
    /// 将表格降级为普通文本。
    PlainText,
    /// 忽略表格。
    Ignore,
}
