//! PDF 内容的来源位置。

use easypdf_core::PageIndex;

/// 语义内容在源 PDF 中的位置与识别置信度。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceLocation {
    page_index: PageIndex,
    confidence: f32,
}

impl SourceLocation {
    /// 创建来源位置，置信度会限制在 `0.0..=1.0`。
    #[must_use]
    pub fn new(page_index: PageIndex, confidence: f32) -> Self {
        Self {
            page_index,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// 返回零基页索引。
    #[must_use]
    pub const fn page_index(&self) -> PageIndex {
        self.page_index
    }

    /// 返回识别置信度。
    #[must_use]
    pub const fn confidence(&self) -> f32 {
        self.confidence
    }
}
