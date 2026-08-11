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

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_valid_value() {
        let pn = PageNumber::new(1).unwrap();
        assert_eq!(pn.value(), 1);
    }

    #[test]
    fn new_large_value() {
        let pn = PageNumber::new(1000).unwrap();
        assert_eq!(pn.value(), 1000);
    }

    #[test]
    fn new_zero_returns_error() {
        let result = PageNumber::new(0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PdfError::InvalidPage(0)));
    }

    #[test]
    fn from_index_converts_correctly() {
        let idx = PageIndex::new(0);
        let pn = PageNumber::from_index(idx);
        assert_eq!(pn.value(), 1);

        let idx = PageIndex::new(4);
        let pn = PageNumber::from_index(idx);
        assert_eq!(pn.value(), 5);
    }

    #[test]
    fn value_returns_inner() {
        let pn = PageNumber::new(42).unwrap();
        assert_eq!(pn.value(), 42);
    }

    #[test]
    fn index_converts_to_zero_based() {
        let pn = PageNumber::new(1).unwrap();
        assert_eq!(pn.index().value(), 0);

        let pn = PageNumber::new(5).unwrap();
        assert_eq!(pn.index().value(), 4);
    }

    #[test]
    fn roundtrip_page_number_to_index_and_back() {
        let pn = PageNumber::new(10).unwrap();
        let idx = pn.index();
        let pn2 = PageNumber::from_index(idx);
        assert_eq!(pn, pn2);
    }

    #[test]
    fn clone_and_copy() {
        let pn = PageNumber::new(3).unwrap();
        let cloned = pn;
        assert_eq!(pn, cloned);
    }

    #[test]
    fn debug_format() {
        let pn = PageNumber::new(7).unwrap();
        assert_eq!(format!("{:?}", pn), "PageNumber(7)");
    }

    #[test]
    fn ordering() {
        let a = PageNumber::new(1).unwrap();
        let b = PageNumber::new(5).unwrap();
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn equality() {
        let a = PageNumber::new(3).unwrap();
        let b = PageNumber::new(3).unwrap();
        let c = PageNumber::new(4).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn hash_same_values() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PageNumber::new(1).unwrap());
        set.insert(PageNumber::new(1).unwrap());
        set.insert(PageNumber::new(2).unwrap());
        assert_eq!(set.len(), 2);
    }
}
