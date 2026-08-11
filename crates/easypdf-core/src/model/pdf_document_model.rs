//! PDF 文档语义模型。

use std::collections::HashMap;

use crate::PdfMetadata;

use crate::{PdfBlock, PdfBlockType, PdfPageModel};

/// PDF 文档的引擎无关语义表示。
///
/// 包含元数据与按页组织的语义内容块。
#[derive(Clone, Debug, Default)]
pub struct PdfDocumentModel {
    metadata: PdfMetadata,
    pages: Vec<PdfPageModel>,
}

impl PdfDocumentModel {
    /// 创建文档模型。
    #[must_use]
    pub const fn new(metadata: PdfMetadata, pages: Vec<PdfPageModel>) -> Self {
        Self { metadata, pages }
    }

    /// 返回文档元数据。
    #[must_use]
    pub const fn metadata(&self) -> &PdfMetadata {
        &self.metadata
    }

    /// 返回页面列表。
    #[must_use]
    pub fn pages(&self) -> &[PdfPageModel] {
        &self.pages
    }

    /// 返回页面数量。
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// 判断文档是否不含页面。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    /// 返回页面迭代器。
    pub fn iter(&self) -> impl Iterator<Item = &PdfPageModel> {
        self.pages.iter()
    }

    /// 返回所有页面中内容块的总数。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel, SourceLocation};
    /// use easypdf_core::{PageIndex, PdfMetadata};
    ///
    /// let loc = SourceLocation::new(PageIndex::new(0), 1.0);
    /// let page = PdfPageModel::new(PageIndex::new(0))
    ///     .with_block(PdfBlock::paragraph("A", loc))
    ///     .with_block(PdfBlock::paragraph("B", loc));
    /// let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
    /// assert_eq!(doc.total_blocks(), 2);
    /// ```
    #[must_use]
    pub fn total_blocks(&self) -> usize {
        self.pages.iter().map(|p| p.blocks().len()).sum()
    }

    /// 扁平遍历所有页面的内容块，附带一基页码。
    ///
    /// 返回迭代器产出 `(page_number, &PdfBlock)` 元组。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel, SourceLocation};
    /// use easypdf_core::{PageIndex, PdfMetadata};
    ///
    /// let loc = SourceLocation::new(PageIndex::new(0), 1.0);
    /// let page = PdfPageModel::new(PageIndex::new(0))
    ///     .with_block(PdfBlock::paragraph("Hello", loc));
    /// let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
    /// let blocks: Vec<_> = doc.iter_all_blocks().collect();
    /// assert_eq!(blocks.len(), 1);
    /// assert_eq!(blocks[0].0, 1); // 1-based page number
    /// ```
    pub fn iter_all_blocks(&self) -> impl Iterator<Item = (usize, &PdfBlock)> {
        self.pages
            .iter()
            .flat_map(|page| {
                let num = page.page_number();
                page.blocks().iter().map(move |block| (num, block))
            })
    }

    /// 统计每种 [`PdfBlockType`] 的出现次数。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::{PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel, SourceLocation};
    /// use easypdf_core::{PageIndex, PdfMetadata};
    ///
    /// let loc = SourceLocation::new(PageIndex::new(0), 1.0);
    /// let page = PdfPageModel::new(PageIndex::new(0))
    ///     .with_block(PdfBlock::heading(1, "Title", loc))
    ///     .with_block(PdfBlock::paragraph("Body", loc));
    /// let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
    /// let counts = doc.block_count_by_type();
    /// assert_eq!(counts[&PdfBlockType::Heading], 1);
    /// assert_eq!(counts[&PdfBlockType::Paragraph], 1);
    /// ```
    #[must_use]
    pub fn block_count_by_type(&self) -> HashMap<PdfBlockType, usize> {
        let mut map = HashMap::new();
        for page in &self.pages {
            for block in page.blocks() {
                *map.entry(block.block_type()).or_insert(0) += 1;
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceLocation;
    use crate::PageIndex;

    fn loc(page: usize) -> SourceLocation {
        SourceLocation::new(PageIndex::new(page), 1.0)
    }

    fn make_doc() -> PdfDocumentModel {
        let page0 = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "Title", loc(0)))
            .with_block(PdfBlock::paragraph("Body", loc(0)))
            .with_block(PdfBlock::code("fn main() {}", loc(0)));
        let page1 = PdfPageModel::new(PageIndex::new(1))
            .with_block(PdfBlock::paragraph("More text", loc(1)))
            .with_block(PdfBlock::formula("E=mc^2", loc(1)));
        PdfDocumentModel::new(PdfMetadata::default(), vec![page0, page1])
    }

    #[test]
    fn total_blocks_counts_all() {
        let doc = make_doc();
        assert_eq!(doc.total_blocks(), 5);
    }

    #[test]
    fn iter_all_blocks_flattens_with_page_number() {
        let doc = make_doc();
        let blocks: Vec<_> = doc.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].0, 1);
        assert_eq!(blocks[3].0, 2);
    }

    #[test]
    fn block_count_by_type_groups() {
        let doc = make_doc();
        let counts = doc.block_count_by_type();
        assert_eq!(counts[&PdfBlockType::Heading], 1);
        assert_eq!(counts[&PdfBlockType::Paragraph], 2);
        assert_eq!(counts[&PdfBlockType::Code], 1);
        assert_eq!(counts[&PdfBlockType::Formula], 1);
        assert!(!counts.contains_key(&PdfBlockType::Table));
    }

    #[test]
    fn empty_doc_totals() {
        let doc = PdfDocumentModel::default();
        assert_eq!(doc.total_blocks(), 0);
        assert!(doc.iter_all_blocks().next().is_none());
        assert!(doc.block_count_by_type().is_empty());
    }

    #[test]
    fn new_constructor_stores_metadata_and_pages() {
        let meta = PdfMetadata::default();
        let page = PdfPageModel::new(PageIndex::new(0));
        let doc = PdfDocumentModel::new(meta, vec![page]);
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn metadata_returns_reference() {
        let doc = make_doc();
        let _meta = doc.metadata();
    }

    #[test]
    fn pages_returns_slice() {
        let doc = make_doc();
        assert_eq!(doc.pages().len(), 2);
    }

    #[test]
    fn page_count_reflects_vec_length() {
        let doc = PdfDocumentModel::default();
        assert_eq!(doc.page_count(), 0);
        let doc2 = make_doc();
        assert_eq!(doc2.page_count(), 2);
    }

    #[test]
    fn is_empty_true_for_default() {
        assert!(PdfDocumentModel::default().is_empty());
    }

    #[test]
    fn is_empty_false_with_pages() {
        let doc = make_doc();
        assert!(!doc.is_empty());
    }

    #[test]
    fn iter_yields_all_pages() {
        let doc = make_doc();
        let count = doc.iter().count();
        assert_eq!(count, 2);
    }
}
