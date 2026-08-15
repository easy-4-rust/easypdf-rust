//! Table detector processor implementing [`PdfMarkdownProcessor`].

use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};
use easypdf_core::PdfInput;
use easypdf_core::Result;
use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel};

use super::config::TableDetectionConfig;
use super::heuristic::detect_table_region;

/// Heuristic table detection processor.
///
/// Scans each page's [`PdfBlock::Paragraph`] blocks for table patterns
/// (pipe-separated, tab-separated, or whitespace-aligned) and replaces
/// consecutive matching rows with a single [`PdfBlock::Table`].
///
/// The first row of a detected region becomes the table header; the
/// remaining rows become data rows. Separator rows (e.g., `|---|---|`)
/// are automatically skipped.
///
/// # Capability level
///
/// This processor declares [`CapabilityLevel::Heuristic`](easypdf_core::CapabilityLevel::Heuristic)
/// for table detection — it works purely on text patterns without
/// inspecting PDF vector graphics or font metrics.
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
    /// Create a processor with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TableDetectionConfig::default(),
        }
    }

    /// Create a processor with custom configuration.
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

/// Process a single page, replacing table regions with `PdfBlock::Table`.
fn process_page(page: &PdfPageModel, config: &TableDetectionConfig) -> PdfPageModel {
    let blocks = page.blocks();
    let mut new_blocks = Vec::new();
    let mut i = 0;

    while i < blocks.len() {
        if let Some(region) = detect_table_region(blocks, i, config) {
            // Use the source location of the first row (the header paragraph).
            let source = *blocks[i].source();
            new_blocks.push(PdfBlock::table(region.headers, region.rows, source));
            i = region.end_index + 1;
        } else {
            new_blocks.push(blocks[i].clone());
            i += 1;
        }
    }

    // Reconstruct the page preserving dimensions and rotation.
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
