//! PDF 语义内容块。

use crate::{ImageData, ListItem, PdfBlockType, SourceLocation};

/// 从 PDF 页面识别出的语义内容块。
///
/// `#[non_exhaustive]` 保证未来新增变体不会破坏下游代码。
/// 消费方应始终包含通配分支（`_ => ...`）或使用 [`block_type`](Self::block_type)
/// 进行分发。
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PdfBlock {
    /// 分级标题。
    Heading {
        /// 标题级别，范围为 1 到 6。
        level: u8,
        /// 标题文本。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 普通段落。
    Paragraph {
        /// 段落文本。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 有序或无序列表。
    List {
        /// 是否为有序列表。
        ordered: bool,
        /// 列表项（支持嵌套）。
        items: Vec<ListItem>,
        /// 源位置。
        source: SourceLocation,
    },
    /// 表格数据。
    Table {
        /// 表头。
        headers: Vec<String>,
        /// 表格行。
        rows: Vec<Vec<String>>,
        /// 源位置。
        source: SourceLocation,
    },
    /// 图片引用。
    Image {
        /// 图片元数据。
        data: ImageData,
        /// 源位置。
        source: SourceLocation,
    },
    /// 代码块。
    Code {
        /// 代码语言标识（如 `"rust"`、`"python"`）。
        language: Option<String>,
        /// 代码文本。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 数学公式（LaTeX 语法）。
    Formula {
        /// LaTeX 源码。
        latex: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 分页符。
    PageBreak {
        /// 源位置。
        source: SourceLocation,
    },
    /// 脚注。
    Footnote {
        /// 脚注引用标识。
        reference_id: String,
        /// 脚注正文。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 表格单元格（细粒度识别）。
    TableCell {
        /// 行跨度。
        row_span: u32,
        /// 列跨度。
        col_span: u32,
        /// 单元格文本。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 引用块。
    BlockQuote {
        /// 引用文本。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 水平分隔线。
    HorizontalRule {
        /// 源位置。
        source: SourceLocation,
    },
    /// 超链接。
    Link {
        /// 链接地址。
        url: String,
        /// 链接显示文本。
        text: String,
        /// 源位置。
        source: SourceLocation,
    },
    /// 无法识别的内容。
    Unknown {
        /// 原始文本。
        raw: String,
        /// 源位置。
        source: SourceLocation,
    },
}

impl PdfBlock {
    // ------------------------------------------------------------------
    //  便捷构造方法
    // ------------------------------------------------------------------

    /// 创建分级标题。
    #[must_use]
    pub fn heading(level: u8, text: impl Into<String>, source: SourceLocation) -> Self {
        Self::Heading {
            level,
            text: text.into(),
            source,
        }
    }

    /// 创建普通段落。
    #[must_use]
    pub fn paragraph(text: impl Into<String>, source: SourceLocation) -> Self {
        Self::Paragraph {
            text: text.into(),
            source,
        }
    }

    /// 创建有序或无序列表。
    #[must_use]
    pub fn list(ordered: bool, items: Vec<ListItem>, source: SourceLocation) -> Self {
        Self::List {
            ordered,
            items,
            source,
        }
    }

    /// 创建表格。
    #[must_use]
    pub fn table(headers: Vec<String>, rows: Vec<Vec<String>>, source: SourceLocation) -> Self {
        Self::Table {
            headers,
            rows,
            source,
        }
    }

    /// 创建图片引用。
    #[must_use]
    pub fn image(data: ImageData, source: SourceLocation) -> Self {
        Self::Image { data, source }
    }

    /// 创建代码块。
    #[must_use]
    pub fn code(text: impl Into<String>, source: SourceLocation) -> Self {
        Self::Code {
            language: None,
            text: text.into(),
            source,
        }
    }

    /// 创建带语言标识的代码块。
    #[must_use]
    pub fn code_with_language(
        language: impl Into<String>,
        text: impl Into<String>,
        source: SourceLocation,
    ) -> Self {
        Self::Code {
            language: Some(language.into()),
            text: text.into(),
            source,
        }
    }

    /// 创建数学公式。
    #[must_use]
    pub fn formula(latex: impl Into<String>, source: SourceLocation) -> Self {
        Self::Formula {
            latex: latex.into(),
            source,
        }
    }

    /// 创建分页符。
    #[must_use]
    pub const fn page_break(source: SourceLocation) -> Self {
        Self::PageBreak { source }
    }

    /// 创建脚注。
    #[must_use]
    pub fn footnote(
        ref_id: impl Into<String>,
        text: impl Into<String>,
        source: SourceLocation,
    ) -> Self {
        Self::Footnote {
            reference_id: ref_id.into(),
            text: text.into(),
            source,
        }
    }

    /// 创建表格单元格。
    #[must_use]
    pub fn table_cell(
        row_span: u32,
        col_span: u32,
        text: impl Into<String>,
        source: SourceLocation,
    ) -> Self {
        Self::TableCell {
            row_span,
            col_span,
            text: text.into(),
            source,
        }
    }

    /// 创建引用块。
    #[must_use]
    pub fn block_quote(text: impl Into<String>, source: SourceLocation) -> Self {
        Self::BlockQuote {
            text: text.into(),
            source,
        }
    }

    /// 创建水平分隔线。
    #[must_use]
    pub const fn horizontal_rule(source: SourceLocation) -> Self {
        Self::HorizontalRule { source }
    }

    /// 创建超链接。
    #[must_use]
    pub fn link(url: impl Into<String>, text: impl Into<String>, source: SourceLocation) -> Self {
        Self::Link {
            url: url.into(),
            text: text.into(),
            source,
        }
    }

    /// 创建无法识别的内容。
    #[must_use]
    pub fn unknown(raw: impl Into<String>, source: SourceLocation) -> Self {
        Self::Unknown {
            raw: raw.into(),
            source,
        }
    }

    // ------------------------------------------------------------------
    //  查询方法
    // ------------------------------------------------------------------

    /// 返回内容块的源位置。
    #[must_use]
    pub const fn source(&self) -> &SourceLocation {
        match self {
            Self::Heading { source, .. }
            | Self::Paragraph { source, .. }
            | Self::List { source, .. }
            | Self::Table { source, .. }
            | Self::Image { source, .. }
            | Self::Code { source, .. }
            | Self::Formula { source, .. }
            | Self::PageBreak { source }
            | Self::Footnote { source, .. }
            | Self::TableCell { source, .. }
            | Self::BlockQuote { source, .. }
            | Self::HorizontalRule { source }
            | Self::Link { source, .. }
            | Self::Unknown { source, .. } => source,
        }
    }

    /// 返回内容块的语义分类。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::{PdfBlock, PdfBlockType, SourceLocation};
    /// use easypdf_core::PageIndex;
    ///
    /// let loc = SourceLocation::new(PageIndex::new(0), 1.0);
    /// let block = PdfBlock::heading(1, "Title", loc);
    /// assert_eq!(block.block_type(), PdfBlockType::Heading);
    /// ```
    #[must_use]
    pub const fn block_type(&self) -> PdfBlockType {
        match self {
            Self::Heading { .. } => PdfBlockType::Heading,
            Self::Paragraph { .. } => PdfBlockType::Paragraph,
            Self::List { .. } => PdfBlockType::List,
            Self::Table { .. } => PdfBlockType::Table,
            Self::Image { .. } => PdfBlockType::Image,
            Self::Code { .. } => PdfBlockType::Code,
            Self::Formula { .. } => PdfBlockType::Formula,
            Self::PageBreak { .. } => PdfBlockType::PageBreak,
            Self::Footnote { .. } => PdfBlockType::Footnote,
            Self::TableCell { .. } => PdfBlockType::TableCell,
            Self::BlockQuote { .. } => PdfBlockType::BlockQuote,
            Self::HorizontalRule { .. } => PdfBlockType::HorizontalRule,
            Self::Link { .. } => PdfBlockType::Link,
            Self::Unknown { .. } => PdfBlockType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PageIndex;

    fn loc() -> SourceLocation {
        SourceLocation::new(PageIndex::new(0), 1.0)
    }

    #[test]
    fn heading_construction() {
        let b = PdfBlock::heading(2, "Section", loc());
        assert_eq!(b.block_type(), PdfBlockType::Heading);
        assert_eq!(b.source().page_index().value(), 0);
    }

    #[test]
    fn paragraph_construction() {
        let b = PdfBlock::paragraph("Hello", loc());
        assert_eq!(b.block_type(), PdfBlockType::Paragraph);
    }

    #[test]
    fn list_construction() {
        let items = vec![ListItem::new("A"), ListItem::new("B")];
        let b = PdfBlock::list(false, items, loc());
        assert_eq!(b.block_type(), PdfBlockType::List);
    }

    #[test]
    fn table_construction() {
        let b = PdfBlock::table(vec!["H".into()], vec![vec!["C".into()]], loc());
        assert_eq!(b.block_type(), PdfBlockType::Table);
    }

    #[test]
    fn image_construction() {
        let data = ImageData::new(crate::ImageFormat::Png);
        let b = PdfBlock::image(data, loc());
        assert_eq!(b.block_type(), PdfBlockType::Image);
    }

    #[test]
    fn code_construction() {
        let b = PdfBlock::code("fn main() {}", loc());
        assert_eq!(b.block_type(), PdfBlockType::Code);
    }

    #[test]
    fn code_with_language_construction() {
        let b = PdfBlock::code_with_language("rust", "fn main() {}", loc());
        assert_eq!(b.block_type(), PdfBlockType::Code);
    }

    #[test]
    fn formula_construction() {
        let b = PdfBlock::formula("E = mc^2", loc());
        assert_eq!(b.block_type(), PdfBlockType::Formula);
    }

    #[test]
    fn page_break_construction() {
        let b = PdfBlock::page_break(loc());
        assert_eq!(b.block_type(), PdfBlockType::PageBreak);
    }

    #[test]
    fn footnote_construction() {
        let b = PdfBlock::footnote("1", "See page 5", loc());
        assert_eq!(b.block_type(), PdfBlockType::Footnote);
    }

    #[test]
    fn table_cell_construction() {
        let b = PdfBlock::table_cell(2, 1, "Merged", loc());
        assert_eq!(b.block_type(), PdfBlockType::TableCell);
    }

    #[test]
    fn block_quote_construction() {
        let b = PdfBlock::block_quote("A wise saying", loc());
        assert_eq!(b.block_type(), PdfBlockType::BlockQuote);
    }

    #[test]
    fn horizontal_rule_construction() {
        let b = PdfBlock::horizontal_rule(loc());
        assert_eq!(b.block_type(), PdfBlockType::HorizontalRule);
    }

    #[test]
    fn link_construction() {
        let b = PdfBlock::link("https://example.com", "Example", loc());
        assert_eq!(b.block_type(), PdfBlockType::Link);
    }

    #[test]
    fn unknown_construction() {
        let b = PdfBlock::unknown("???binary???", loc());
        assert_eq!(b.block_type(), PdfBlockType::Unknown);
    }

    #[test]
    fn source_returns_correct_location() {
        let loc2 = SourceLocation::new(PageIndex::new(3), 0.85);
        let b = PdfBlock::paragraph("test", loc2);
        assert_eq!(b.source().page_index().value(), 3);
        assert!((b.source().confidence() - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn block_type_covers_all_variants() {
        let loc = loc();
        let variants = [
            PdfBlock::heading(1, "h", loc),
            PdfBlock::paragraph("p", loc),
            PdfBlock::list(false, vec![], loc),
            PdfBlock::table(vec![], vec![], loc),
            PdfBlock::image(ImageData::new(crate::ImageFormat::Png), loc),
            PdfBlock::code("c", loc),
            PdfBlock::formula("f", loc),
            PdfBlock::page_break(loc),
            PdfBlock::footnote("1", "t", loc),
            PdfBlock::table_cell(1, 1, "t", loc),
            PdfBlock::block_quote("q", loc),
            PdfBlock::horizontal_rule(loc),
            PdfBlock::link("u", "t", loc),
            PdfBlock::unknown("r", loc),
        ];
        let types: Vec<_> = variants.iter().map(PdfBlock::block_type).collect();
        assert_eq!(types.len(), 14);
        // 每个变体应映射到不同分类
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j], "duplicate at {i} vs {j}");
            }
        }
    }

    #[test]
    fn clone_and_eq() {
        let a = PdfBlock::paragraph("hello", loc());
        let b = a.clone();
        assert_eq!(a, b);
    }
}
