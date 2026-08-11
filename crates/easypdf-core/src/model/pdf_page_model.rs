//! 单页 PDF 语义模型。

use crate::{PageIndex, PageNumber};

use crate::{PdfBlock, PdfBlockType};

/// 一页 PDF 的语义内容。
///
/// 通过 [`with_block`](Self::with_block) 逐块构建页面，
/// 可选设置页面尺寸与旋转角度。
#[derive(Clone, Debug, PartialEq)]
pub struct PdfPageModel {
    index: PageIndex,
    blocks: Vec<PdfBlock>,
    width_pt: Option<f64>,
    height_pt: Option<f64>,
    rotation: u16,
}

impl PdfPageModel {
    /// 创建空页面模型。
    #[must_use]
    pub const fn new(index: PageIndex) -> Self {
        Self {
            index,
            blocks: Vec::new(),
            width_pt: None,
            height_pt: None,
            rotation: 0,
        }
    }

    /// 追加语义内容块。
    #[must_use]
    pub fn with_block(mut self, block: PdfBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// 设置页面尺寸（PDF points）。
    #[must_use]
    pub const fn with_dimensions(mut self, width_pt: f64, height_pt: f64) -> Self {
        self.width_pt = Some(width_pt);
        self.height_pt = Some(height_pt);
        self
    }

    /// 设置页面旋转角度（0、90、180、270）。
    ///
    /// 非标准值会被静默保留，但调用方应仅传入四个标准角度。
    #[must_use]
    pub const fn with_rotation(mut self, degrees: u16) -> Self {
        self.rotation = degrees;
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

    /// 返回一基页码数值。
    #[must_use]
    pub fn page_number(&self) -> usize {
        self.index.value() + 1
    }

    /// 返回页面宽度（PDF points）。
    #[must_use]
    pub const fn width_pt(&self) -> Option<f64> {
        self.width_pt
    }

    /// 返回页面高度（PDF points）。
    #[must_use]
    pub const fn height_pt(&self) -> Option<f64> {
        self.height_pt
    }

    /// 返回页面旋转角度。
    #[must_use]
    pub const fn rotation(&self) -> u16 {
        self.rotation
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

    /// 按语义分类筛选内容块。
    ///
    /// 返回迭代器，仅产出匹配指定 [`PdfBlockType`] 的块引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::{PdfBlock, PdfBlockType, PdfPageModel, SourceLocation};
    /// use easypdf_core::PageIndex;
    ///
    /// let loc = SourceLocation::new(PageIndex::new(0), 1.0);
    /// let page = PdfPageModel::new(PageIndex::new(0))
    ///     .with_block(PdfBlock::heading(1, "Title", loc))
    ///     .with_block(PdfBlock::paragraph("Body", loc));
    ///
    /// let headings: Vec<_> = page.blocks_by_type(PdfBlockType::Heading).collect();
    /// assert_eq!(headings.len(), 1);
    /// ```
    pub fn blocks_by_type(
        &self,
        filter: PdfBlockType,
    ) -> impl Iterator<Item = &PdfBlock> {
        self.blocks.iter().filter(move |b| b.block_type() == filter)
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::SourceLocation;

    fn loc(page: usize) -> SourceLocation {
        SourceLocation::new(PageIndex::new(page), 1.0)
    }

    #[test]
    fn page_number_is_one_based() {
        let page = PdfPageModel::new(PageIndex::new(0));
        assert_eq!(page.page_number(), 1);

        let page = PdfPageModel::new(PageIndex::new(4));
        assert_eq!(page.page_number(), 5);
    }

    #[test]
    fn dimensions_default_none() {
        let page = PdfPageModel::new(PageIndex::new(0));
        assert!(page.width_pt().is_none());
        assert!(page.height_pt().is_none());
    }

    #[test]
    fn dimensions_builder() {
        let page = PdfPageModel::new(PageIndex::new(0)).with_dimensions(595.0, 842.0);
        assert_eq!(page.width_pt(), Some(595.0));
        assert_eq!(page.height_pt(), Some(842.0));
    }

    #[test]
    fn rotation_default_zero() {
        let page = PdfPageModel::new(PageIndex::new(0));
        assert_eq!(page.rotation(), 0);
    }

    #[test]
    fn rotation_builder() {
        let page = PdfPageModel::new(PageIndex::new(0)).with_rotation(90);
        assert_eq!(page.rotation(), 90);
    }

    #[test]
    fn blocks_by_type_filters_correctly() {
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "Title", loc(0)))
            .with_block(PdfBlock::paragraph("Body", loc(0)))
            .with_block(PdfBlock::code("fn main() {}", loc(0)));

        let headings: Vec<_> = page.blocks_by_type(PdfBlockType::Heading).collect();
        assert_eq!(headings.len(), 1);

        let paragraphs: Vec<_> = page.blocks_by_type(PdfBlockType::Paragraph).collect();
        assert_eq!(paragraphs.len(), 1);

        let code: Vec<_> = page.blocks_by_type(PdfBlockType::Code).collect();
        assert_eq!(code.len(), 1);

        let tables: Vec<_> = page.blocks_by_type(PdfBlockType::Table).collect();
        assert!(tables.is_empty());
    }

    #[test]
    fn index_returns_zero_based() {
        let page = PdfPageModel::new(PageIndex::new(3));
        assert_eq!(page.index().value(), 3);
    }

    #[test]
    fn number_returns_page_number() {
        let page = PdfPageModel::new(PageIndex::new(2));
        assert_eq!(page.number().value(), 3);
    }

    #[test]
    fn iter_returns_blocks() {
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "H", loc(0)))
            .with_block(PdfBlock::paragraph("P", loc(0)));
        let blocks: Vec<_> = page.iter().collect();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn empty_page_blocks() {
        let page = PdfPageModel::new(PageIndex::new(0));
        assert!(page.blocks().is_empty());
        assert!(page.iter().next().is_none());
    }

    #[test]
    fn with_block_chain() {
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "A", loc(0)))
            .with_block(PdfBlock::paragraph("B", loc(0)))
            .with_block(PdfBlock::code("C", loc(0)));
        assert_eq!(page.blocks().len(), 3);
    }

    #[test]
    fn clone_eq() {
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("X", loc(0)));
        let cloned = page.clone();
        assert_eq!(page, cloned);
    }

    #[test]
    fn debug_format() {
        let page = PdfPageModel::new(PageIndex::new(0));
        let dbg = format!("{:?}", page);
        assert!(dbg.contains("PdfPageModel"));
    }
}
