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
