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

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp, clippy::reversed_empty_ranges)]
mod tests {
    use super::*;

    #[test]
    fn new_valid_range() {
        let pr = PageRange::new(0..10).unwrap();
        assert_eq!(pr.start(), 0);
        assert_eq!(pr.end(), 10);
    }

    #[test]
    fn new_empty_range() {
        let pr = PageRange::new(5..5).unwrap();
        assert_eq!(pr.start(), 5);
        assert_eq!(pr.end(), 5);
    }

    #[test]
    fn new_reversed_range_returns_error() {
        let result = PageRange::new(10..5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PdfError::InvalidPage(5)));
    }

    #[test]
    fn empty_at_creates_empty_range() {
        let pr = PageRange::empty_at(7);
        assert_eq!(pr.start(), 7);
        assert_eq!(pr.end(), 7);
        // Empty range: start == end
        assert_eq!(pr.start(), pr.end());
    }

    #[test]
    fn contains_within_range() {
        let pr = PageRange::new(2..8).unwrap();
        assert!(!pr.contains(1));
        assert!(pr.contains(2));
        assert!(pr.contains(5));
        assert!(!pr.contains(8));
    }

    #[test]
    fn contains_empty_range() {
        let pr = PageRange::empty_at(3);
        assert!(!pr.contains(3));
    }

    #[test]
    fn as_range_returns_inner() {
        let pr = PageRange::new(1..5).unwrap();
        let range = pr.as_range();
        assert_eq!(*range, 1..5);
    }

    #[test]
    fn try_from_valid_range() {
        let pr: PageRange = (0..3).try_into().unwrap();
        assert_eq!(pr.start(), 0);
        assert_eq!(pr.end(), 3);
    }

    #[test]
    fn try_from_invalid_range() {
        let result: Result<PageRange> = (5..2).try_into();
        assert!(result.is_err());
    }

    #[test]
    fn clone_preserves_values() {
        let pr = PageRange::new(1..10).unwrap();
        let cloned = pr.clone();
        assert_eq!(pr, cloned);
    }

    #[test]
    fn partial_eq_works() {
        let a = PageRange::new(0..5).unwrap();
        let b = PageRange::new(0..5).unwrap();
        let c = PageRange::new(0..6).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn debug_format() {
        let pr = PageRange::new(2..8).unwrap();
        let dbg = format!("{:?}", pr);
        assert!(dbg.contains('2'));
        assert!(dbg.contains('8'));
    }

    #[test]
    fn hash_same_values() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PageRange::new(0..5).unwrap());
        set.insert(PageRange::new(0..5).unwrap());
        set.insert(PageRange::new(0..6).unwrap());
        assert_eq!(set.len(), 2);
    }
}
