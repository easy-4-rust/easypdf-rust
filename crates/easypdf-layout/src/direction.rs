//! 流式布局方向。

/// 流式布局方向。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// 从上到下排列元素。
    #[default]
    Vertical,
    /// 从左到右排列元素。
    Horizontal,
}
