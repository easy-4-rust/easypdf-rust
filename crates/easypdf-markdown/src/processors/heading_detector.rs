//! 标题检测处理器。

use easypdf_core::Result;
use easypdf_core::PdfInput;
use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel};

use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};

/// 标题检测处理器（Heuristic 级别）。
///
/// 扫描文档中的 [`PdfBlock::Paragraph`] 块，根据启发式规则
/// 将短文本段落提升为 [`PdfBlock::Heading`]：
///
/// - 文本长度 <= `max_heading_length`（默认 80 字符）
/// - 文本不含句号（非完整句子）
/// - 文本首字母大写或全大写
///
/// TODO: 后续增强——基于字体大小与加粗样式识别标题层级。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::processors::HeadingDetectorProcessor;
/// use easypdf_markdown::PdfMarkdownProcessor;
///
/// let proc = HeadingDetectorProcessor::new();
/// assert!(proc.capabilities().reading_order());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct HeadingDetectorProcessor {
    /// 段落被视为标题候选的最大字符数。
    max_heading_length: usize,
}

impl HeadingDetectorProcessor {
    /// 创建默认配置的标题检测处理器。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_heading_length: 80,
        }
    }

    /// 设置标题候选的最大字符数。
    #[must_use]
    pub const fn with_max_length(mut self, max: usize) -> Self {
        self.max_heading_length = max;
        self
    }

    /// 判断文本是否符合标题候选条件。
    fn is_heading_candidate(self, text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > self.max_heading_length {
            return false;
        }
        // 不含句号（排除完整句子）
        if trimmed.contains('.') || trimmed.contains('。') {
            return false;
        }
        // 首字母大写或全大写
        trimmed
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
    }
}

impl Default for HeadingDetectorProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfMarkdownProcessor for HeadingDetectorProcessor {
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        // 标题检测也属于结构识别，归入 reading_order 能力。
        MarkdownProcessorCapabilities::new().with_reading_order()
    }

    fn process(
        &self,
        _input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
        let mut new_pages = Vec::with_capacity(document.page_count());
        for page in document.pages() {
            let mut new_blocks = Vec::new();
            for block in page.blocks() {
                match block {
                    PdfBlock::Paragraph { text, source } => {
                        if self.is_heading_candidate(text) {
                            // TODO: 基于字体大小确定 level
                            new_blocks.push(PdfBlock::heading(2, text, *source));
                        } else {
                            new_blocks.push(block.clone());
                        }
                    }
                    other => new_blocks.push(other.clone()),
                }
            }
            let mut new_page = PdfPageModel::new(page.index());
            if let (Some(w), Some(h)) = (page.width_pt(), page.height_pt()) {
                new_page = new_page.with_dimensions(w, h);
            }
            new_page = new_page.with_rotation(page.rotation());
            for block in new_blocks {
                new_page = new_page.with_block(block);
            }
            new_pages.push(new_page);
        }
        Ok((
            PdfDocumentModel::new(document.metadata().clone(), new_pages),
            Vec::new(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypdf_core::{PageIndex, PdfMetadata};
    use easypdf_core::{PdfPageModel, SourceLocation};

    fn loc() -> SourceLocation {
        SourceLocation::new(PageIndex::new(0), 1.0)
    }

    #[test]
    fn capabilities_include_reading_order() {
        let proc = HeadingDetectorProcessor::new();
        assert!(proc.capabilities().reading_order());
    }

    #[test]
    fn short_uppercase_becomes_heading() {
        let proc = HeadingDetectorProcessor::new();
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("INTRODUCTION", loc()));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, _) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0].1, PdfBlock::Heading { .. }));
    }

    #[test]
    fn sentence_with_period_stays_paragraph() {
        let proc = HeadingDetectorProcessor::new();
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("This is a full sentence.", loc()));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, _) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert!(matches!(blocks[0].1, PdfBlock::Paragraph { .. }));
    }

    #[test]
    fn long_text_stays_paragraph() {
        let proc = HeadingDetectorProcessor::new();
        let long_text = "A".repeat(100);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph(&long_text, loc()));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, _) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert!(matches!(blocks[0].1, PdfBlock::Paragraph { .. }));
    }

    #[test]
    fn empty_text_stays_paragraph() {
        let proc = HeadingDetectorProcessor::new();
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("", loc()));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, _) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn custom_max_length() {
        let proc = HeadingDetectorProcessor::new().with_max_length(10);
        // 11 chars, exceeds custom max
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("ABCDEFGHIJK", loc()));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, _) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert!(matches!(blocks[0].1, PdfBlock::Paragraph { .. }));
    }

    #[test]
    fn is_heading_candidate_rules() {
        let proc = HeadingDetectorProcessor::new();
        assert!(proc.is_heading_candidate("Introduction"));
        assert!(proc.is_heading_candidate("CHAPTER ONE"));
        assert!(!proc.is_heading_candidate("this is lowercase"));
        assert!(!proc.is_heading_candidate("Has a period."));
        assert!(!proc.is_heading_candidate(""));
        assert!(!proc.is_heading_candidate(&"A".repeat(100)));
    }
}
