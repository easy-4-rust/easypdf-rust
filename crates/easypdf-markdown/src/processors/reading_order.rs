//! 阅读顺序检测处理器。

use easypdf_core::PdfInput;
use easypdf_core::Result;
use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel};

use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};

/// 阅读顺序检测处理器（Heuristic 级别）。
///
/// 根据内容块在源 PDF 中的出现顺序重新排列，
/// 确保自上而下的阅读顺序。当前实现保留原始顺序，
/// 因为 `PdfPageModel` 中的块已按提取顺序排列。
///
/// TODO: 后续增强——基于 Y 坐标对文本块排序，处理双栏布局。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::processors::ReadingOrderProcessor;
/// use easypdf_markdown::PdfMarkdownProcessor;
///
/// let proc = ReadingOrderProcessor;
/// let caps = proc.capabilities();
/// assert!(caps.reading_order());
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadingOrderProcessor;

impl PdfMarkdownProcessor for ReadingOrderProcessor {
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        MarkdownProcessorCapabilities::new().with_reading_order()
    }

    fn process(
        &self,
        _input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
        // 当前实现：保持原顺序（PDF 提取器已按顺序排列）。
        // TODO: 基于 SourceLocation 的 Y 坐标排序，处理双栏布局。
        let mut new_pages = Vec::with_capacity(document.page_count());
        for page in document.pages() {
            let sorted_blocks = Self::sort_blocks_by_reading_order(page);
            let new_page = rebuild_page(page, sorted_blocks);
            new_pages.push(new_page);
        }
        Ok((
            PdfDocumentModel::new(document.metadata().clone(), new_pages),
            Vec::new(),
        ))
    }
}

impl ReadingOrderProcessor {
    /// 按阅读顺序排列页面内容块。
    ///
    /// 当前实现保留原始顺序。TODO: 基于 Y 坐标排序。
    fn sort_blocks_by_reading_order(page: &PdfPageModel) -> Vec<&PdfBlock> {
        page.blocks().iter().collect()
    }
}

/// 用新块列表重建页面。
fn rebuild_page(page: &PdfPageModel, blocks: Vec<&PdfBlock>) -> PdfPageModel {
    let mut new_page = PdfPageModel::new(page.index());
    if let (Some(w), Some(h)) = (page.width_pt(), page.height_pt()) {
        new_page = new_page.with_dimensions(w, h);
    }
    new_page = new_page.with_rotation(page.rotation());
    for block in blocks {
        new_page = new_page.with_block(block.clone());
    }
    new_page
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypdf_core::SourceLocation;
    use easypdf_core::{PageIndex, PdfMetadata};

    #[test]
    fn capabilities_include_reading_order() {
        let proc = ReadingOrderProcessor;
        assert!(proc.capabilities().reading_order());
    }

    #[test]
    fn preserves_block_count() {
        let proc = ReadingOrderProcessor;
        let loc = SourceLocation::new(PageIndex::new(0), 1.0);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::heading(1, "Title", loc))
            .with_block(PdfBlock::paragraph("Body", loc));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, warnings) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(result.total_blocks(), 2);
    }
}
