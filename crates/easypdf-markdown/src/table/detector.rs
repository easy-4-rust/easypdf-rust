//! 实现 [`PdfMarkdownProcessor`] 的表格检测处理器。

use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};
use easypdf_core::PdfInput;
use easypdf_core::Result;
use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel};

use super::config::TableDetectionConfig;
use super::heuristic::detect_table_region;

/// 启发式表格检测处理器。
///
/// 扫描每个页面的 [`PdfBlock::Paragraph`] 块，查找表格模式
///（管道分隔、制表符分隔或空格对齐），并将连续匹配的行
/// 替换为单个 [`PdfBlock::Table`]。
///
/// 检测到的区域的第一行成为表头；其余行成为数据行。
/// 分隔行（如 `|---|---|`）会被自动跳过。
///
/// # 能力等级
///
/// 此处理器声明 [`CapabilityLevel::Heuristic`](easypdf_core::CapabilityLevel::Heuristic)
/// 的表格检测能力——纯粹基于文本模式工作，不检查 PDF 矢量图形或字体度量。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::PdfMarkdownProcessor;
/// use easypdf_markdown::table::TableDetectorProcessor;
///
/// let proc = TableDetectorProcessor::new();
/// assert!(proc.capabilities().table_detection());
/// ```
#[derive(Clone, Debug)]
pub struct TableDetectorProcessor {
    config: TableDetectionConfig,
}

impl TableDetectorProcessor {
    /// 使用默认配置创建处理器。
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TableDetectionConfig::default(),
        }
    }

    /// 使用自定义配置创建处理器。
    #[must_use]
    pub fn with_config(config: TableDetectionConfig) -> Self {
        Self { config }
    }
}

impl Default for TableDetectorProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfMarkdownProcessor for TableDetectorProcessor {
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        MarkdownProcessorCapabilities::new().with_table_detection()
    }

    fn process(
        &self,
        _input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
        let mut new_pages = Vec::with_capacity(document.page_count());

        for page in document.pages() {
            let new_page = process_page(page, &self.config);
            new_pages.push(new_page);
        }

        Ok((
            PdfDocumentModel::new(document.metadata().clone(), new_pages),
            Vec::new(),
        ))
    }
}

/// 处理单页，将表格区域替换为 `PdfBlock::Table`。
fn process_page(page: &PdfPageModel, config: &TableDetectionConfig) -> PdfPageModel {
    let blocks = page.blocks();
    let mut new_blocks = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        if let Some(region) = detect_table_region(blocks, i, config) {
            // 使用第一行（表头段落）的源位置。
            let source = *blocks[i].source();
            new_blocks.push(PdfBlock::table(region.headers, region.rows, source));
            i = region.end_index + 1;
        } else {
            new_blocks.push(blocks[i].clone());
            i += 1;
        }
    }

    // 重建页面，保留尺寸和旋转。
    let mut new_page = PdfPageModel::new(page.index());
    if let (Some(w), Some(h)) = (page.width_pt(), page.height_pt()) {
        new_page = new_page.with_dimensions(w, h);
    }
    new_page = new_page.with_rotation(page.rotation());
    for block in new_blocks {
        new_page = new_page.with_block(block);
    }
    new_page
}
