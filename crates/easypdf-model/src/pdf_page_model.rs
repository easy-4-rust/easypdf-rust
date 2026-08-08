//! 单页 PDF 语义模型。

use easypdf_core::{PageIndex, PageNumber};

use crate::PdfBlock;

/// 一页 PDF 的语义内容。
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPageModel {
    index: PageIndex,
    blocks: Vec<PdfBlock>,
}

impl PdfPageModel {
    /// 创建空页面模型。
    #[must_use]
    pub const fn new(index: PageIndex) -> Self {
        Self {
            index,
            blocks: Vec::new(),
        }
    }

    /// 追加语义内容块。
    #[must_use]
    pub fn with_block(mut self, block: PdfBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// 返回零基页索引。
    #[must_use]
    pub const fn index(&self) -> PageIndex {
        self.index
    }

    /// 返回一基页码。
    #[must_use]
    pub const fn number(&self) -> PageNumber {
        PageNumber::from_index(self.index)
    }

    /// 返回页面内容块。
    #[must_use]
    pub fn blocks(&self) -> &[PdfBlock] {
        &self.blocks
    }

    /// 返回页面内容块的迭代器。
    pub fn iter(&self) -> impl Iterator<Item = &PdfBlock> {
        self.blocks.iter()
    }
}
