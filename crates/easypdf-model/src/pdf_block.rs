//! PDF 语义内容块。

use crate::SourceLocation;

/// 从 PDF 页面识别出的语义内容块。
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
        /// 列表项。
        items: Vec<String>,
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
        /// 图片替代文本。
        alt: String,
        /// 图片资源路径或标识。
        target: String,
        /// 源位置。
        source: SourceLocation,
    },
}

impl PdfBlock {
    /// 创建普通段落。
    #[must_use]
    pub fn paragraph(text: impl Into<String>, source: SourceLocation) -> Self {
        Self::Paragraph {
            text: text.into(),
            source,
        }
    }

    /// 返回内容块的源位置。
    #[must_use]
    pub const fn source(&self) -> &SourceLocation {
        match self {
            Self::Heading { source, .. }
            | Self::Paragraph { source, .. }
            | Self::List { source, .. }
            | Self::Table { source, .. }
            | Self::Image { source, .. } => source,
        }
    }
}
