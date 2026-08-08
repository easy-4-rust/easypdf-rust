//! Validated zero-based page range.

use std::ops::Range;

use crate::{PdfError, Result};

/// PDF 页的左闭右开零基范围。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageRange(Range<usize>);

impl PageRange {
    /// 创建并校验页范围。
    ///
    /// # Errors
    ///
    /// 当结束位置小于起始位置时返回 [`PdfError::InvalidPage`]。
    pub fn new(range: Range<usize>) -> Result<Self> {
        if range.end < range.start {
            return Err(PdfError::InvalidPage(range.end));
        }
        Ok(Self(range))
    }

    /// 在指定索引创建空范围。
    #[must_use]
    pub const fn empty_at(index: usize) -> Self {
        Self(index..index)
    }

    /// 返回范围起始索引。
    #[must_use]
    pub const fn start(&self) -> usize {
        self.0.start
    }

    /// 返回范围结束索引，不包含该位置。
    #[must_use]
    pub const fn end(&self) -> usize {
        self.0.end
    }

    /// 判断零基页索引是否位于范围内。
    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        self.0.contains(&index)
    }

    /// 返回标准库范围的借用。
    #[must_use]
    pub const fn as_range(&self) -> &Range<usize> {
        &self.0
    }
}

impl TryFrom<Range<usize>> for PageRange {
    type Error = PdfError;

    fn try_from(value: Range<usize>) -> Result<Self> {
        Self::new(value)
    }
}
