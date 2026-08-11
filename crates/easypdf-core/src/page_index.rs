//! Zero-based PDF page index.

/// PDF 页的零基索引，适用于 Rust API 与范围选择。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageIndex(usize);

impl PageIndex {
    /// 创建零基页索引。
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// 返回底层零基索引。
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for PageIndex {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_index() {
        let idx = PageIndex::new(0);
        assert_eq!(idx.value(), 0);
    }

    #[test]
    fn new_large_value() {
        let idx = PageIndex::new(9999);
        assert_eq!(idx.value(), 9999);
    }

    #[test]
    fn value_returns_inner() {
        let idx = PageIndex::new(42);
        assert_eq!(idx.value(), 42);
    }

    #[test]
    fn default_is_zero() {
        let idx = PageIndex::default();
        assert_eq!(idx.value(), 0);
    }

    #[test]
    fn from_usize_conversion() {
        let idx: PageIndex = 5_usize.into();
        assert_eq!(idx.value(), 5);
    }

    #[test]
    fn from_usize_zero() {
        let idx: PageIndex = 0_usize.into();
        assert_eq!(idx.value(), 0);
    }

    #[test]
    fn clone_and_copy() {
        let idx = PageIndex::new(3);
        let cloned = idx;
        assert_eq!(idx, cloned);
    }

    #[test]
    fn partial_eq() {
        let a = PageIndex::new(5);
        let b = PageIndex::new(5);
        let c = PageIndex::new(6);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn ordering() {
        let a = PageIndex::new(1);
        let b = PageIndex::new(10);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn debug_format() {
        let idx = PageIndex::new(7);
        assert_eq!(format!("{:?}", idx), "PageIndex(7)");
    }

    #[test]
    fn hash_same_values() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PageIndex::new(1));
        set.insert(PageIndex::new(1));
        set.insert(PageIndex::new(2));
        assert_eq!(set.len(), 2);
    }
}
