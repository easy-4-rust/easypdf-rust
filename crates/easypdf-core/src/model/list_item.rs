//! PDF 列表项，支持嵌套。

/// 列表中的单个条目，支持递归嵌套子列表。
///
/// # Examples
///
/// ```
/// use easypdf_core::ListItem;
///
/// let item = ListItem::new("Top level");
/// assert_eq!(item.text(), "Top level");
/// assert_eq!(item.level(), 0);
/// assert!(item.children().is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    /// 条目文本。
    text: String,
    /// 嵌套层级，0 表示顶层。
    level: u8,
    /// 子条目（嵌套列表）。
    children: Vec<ListItem>,
}

impl ListItem {
    /// 创建指定层级的列表条目。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::ListItem;
    ///
    /// let item = ListItem::new("Hello");
    /// assert_eq!(item.text(), "Hello");
    /// ```
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: 0,
            children: Vec::new(),
        }
    }

    /// 设置嵌套层级。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::ListItem;
    ///
    /// let item = ListItem::new("Nested").with_level(2);
    /// assert_eq!(item.level(), 2);
    /// ```
    #[must_use]
    pub const fn with_level(mut self, level: u8) -> Self {
        self.level = level;
        self
    }

    /// 追加子条目。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_core::ListItem;
    ///
    /// let item = ListItem::new("Parent").with_child(ListItem::new("Child"));
    /// assert_eq!(item.children().len(), 1);
    /// ```
    #[must_use]
    pub fn with_child(mut self, child: ListItem) -> Self {
        self.children.push(child);
        self
    }

    /// 返回条目文本。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 返回嵌套层级。
    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// 返回子条目列表。
    #[must_use]
    pub fn children(&self) -> &[ListItem] {
        &self.children
    }

    /// 返回可变子条目列表引用。
    pub fn children_mut(&mut self) -> &mut Vec<ListItem> {
        &mut self.children
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_default_item() {
        let item = ListItem::new("Hello");
        assert_eq!(item.text(), "Hello");
        assert_eq!(item.level(), 0);
        assert!(item.children().is_empty());
    }

    #[test]
    fn new_accepts_string() {
        let item = ListItem::new(String::from("Owned"));
        assert_eq!(item.text(), "Owned");
    }

    #[test]
    fn with_level_sets_level() {
        let item = ListItem::new("Nested").with_level(3);
        assert_eq!(item.level(), 3);
    }

    #[test]
    fn with_level_zero_is_default() {
        let item = ListItem::new("Top").with_level(0);
        assert_eq!(item.level(), 0);
    }

    #[test]
    fn with_child_adds_child() {
        let item = ListItem::new("Parent").with_child(ListItem::new("Child"));
        assert_eq!(item.children().len(), 1);
        assert_eq!(item.children()[0].text(), "Child");
    }

    #[test]
    fn with_child_multiple_children() {
        let item = ListItem::new("Parent")
            .with_child(ListItem::new("A"))
            .with_child(ListItem::new("B"))
            .with_child(ListItem::new("C"));
        assert_eq!(item.children().len(), 3);
        assert_eq!(item.children()[0].text(), "A");
        assert_eq!(item.children()[1].text(), "B");
        assert_eq!(item.children()[2].text(), "C");
    }

    #[test]
    fn nested_children() {
        let item = ListItem::new("L0").with_level(0).with_child(
            ListItem::new("L1")
                .with_level(1)
                .with_child(ListItem::new("L2").with_level(2)),
        );
        assert_eq!(item.children().len(), 1);
        assert_eq!(item.children()[0].children().len(), 1);
        assert_eq!(item.children()[0].children()[0].text(), "L2");
    }

    #[test]
    fn children_mut_returns_mutable_ref() {
        let mut item = ListItem::new("Parent");
        item.children_mut().push(ListItem::new("MutChild"));
        assert_eq!(item.children().len(), 1);
        assert_eq!(item.children()[0].text(), "MutChild");
    }

    #[test]
    fn clone_preserves_structure() {
        let item = ListItem::new("Original")
            .with_level(1)
            .with_child(ListItem::new("Child"));
        let cloned = item.clone();
        assert_eq!(item, cloned);
    }

    #[test]
    fn partial_eq_works() {
        let a = ListItem::new("Same").with_level(1);
        let b = ListItem::new("Same").with_level(1);
        let c = ListItem::new("Different").with_level(1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn debug_includes_fields() {
        let item = ListItem::new("Debug").with_level(2);
        let dbg = format!("{:?}", item);
        assert!(dbg.contains("Debug"));
        assert!(dbg.contains('2'));
    }
}
