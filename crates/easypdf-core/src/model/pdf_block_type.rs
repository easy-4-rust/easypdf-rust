//! PDF 内容块分类枚举。

/// 内容块的语义分类，与 [`PdfBlock`] 变体一一对应。
///
/// 用于按类型筛选、统计内容块，无需持有块数据。
///
/// [`PdfBlock`]: crate::PdfBlock
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PdfBlockType {
    /// 分级标题。
    Heading,
    /// 普通段落。
    Paragraph,
    /// 列表。
    List,
    /// 表格。
    Table,
    /// 图片。
    Image,
    /// 代码块。
    Code,
    /// 数学公式。
    Formula,
    /// 分页符。
    PageBreak,
    /// 脚注。
    Footnote,
    /// 表格单元格（细粒度）。
    TableCell,
    /// 引用块。
    BlockQuote,
    /// 水平分隔线。
    HorizontalRule,
    /// 超链接。
    Link,
    /// 无法识别的内容。
    Unknown,
}
