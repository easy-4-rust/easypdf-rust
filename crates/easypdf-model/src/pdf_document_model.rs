//! PDF 文档语义模型。

use easypdf_core::PdfMetadata;

use crate::PdfPageModel;

/// PDF 文档的引擎无关语义表示。
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
}
