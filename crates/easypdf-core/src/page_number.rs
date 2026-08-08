//! One-based PDF page number.

use crate::{PageIndex, PdfError, Result};

/// PDF 页的一基显示编号，适用于 PDF 引擎及用户展示。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageNumber(usize);

impl PageNumber {
    /// 创建一基页码。
    ///
    /// # Errors
    ///
    /// 当 `value` 为零时返回 [`PdfError::InvalidPage`]。
    pub const fn new(value: usize) -> Result<Self> {
        if value == 0 {
            return Err(PdfError::InvalidPage(0));
        }
        Ok(Self(value))
    }

    /// 从零基页索引创建页码。
    #[must_use]
    pub const fn from_index(index: PageIndex) -> Self {
        Self(index.value() + 1)
    }

    /// 返回底层一基页码。
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }

    /// 转换为零基页索引。
    #[must_use]
    pub const fn index(self) -> PageIndex {
        PageIndex::new(self.0 - 1)
    }
}
