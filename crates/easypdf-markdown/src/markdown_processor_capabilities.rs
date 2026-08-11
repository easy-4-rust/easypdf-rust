//! Markdown 语义处理器能力声明。

/// PDF 到 Markdown 语义处理器能够提供的增强能力。
///
/// 使用布尔值表示是否支持某项能力。对于更细粒度的等级描述，
/// 请使用 [`DetailedProcessorCapabilities`](crate::DetailedProcessorCapabilities)。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct MarkdownProcessorCapabilities {
    table_detection: bool,
    image_extraction: bool,
    ocr: bool,
    reading_order: bool,
    formula: bool,
    link: bool,
}

impl MarkdownProcessorCapabilities {
    /// 创建空能力集合。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            table_detection: false,
            image_extraction: false,
            ocr: false,
            reading_order: false,
            formula: false,
            link: false,
        }
    }

    /// 声明处理器能够检测表格。
    #[must_use]
    pub const fn with_table_detection(mut self) -> Self {
        self.table_detection = true;
        self
    }

    /// 声明处理器能够提取图片并写入语义模型。
    #[must_use]
    pub const fn with_image_extraction(mut self) -> Self {
        self.image_extraction = true;
        self
    }

    /// 声明处理器能够为无原生文本的页面执行 OCR。
    #[must_use]
    pub const fn with_ocr(mut self) -> Self {
        self.ocr = true;
        self
    }

    /// 返回是否支持表格检测。
    #[must_use]
    pub const fn table_detection(self) -> bool {
        self.table_detection
    }

    /// 返回是否支持图片提取。
    #[must_use]
    pub const fn image_extraction(self) -> bool {
        self.image_extraction
    }

    /// 返回是否支持 OCR。
    #[must_use]
    pub const fn ocr(self) -> bool {
        self.ocr
    }

    /// 声明处理器能够检测阅读顺序。
    #[must_use]
    pub const fn with_reading_order(mut self) -> Self {
        self.reading_order = true;
        self
    }

    /// 返回是否支持阅读顺序检测。
    #[must_use]
    pub const fn reading_order(self) -> bool {
        self.reading_order
    }

    /// 声明处理器能够识别数学公式。
    #[must_use]
    pub const fn with_formula(mut self) -> Self {
        self.formula = true;
        self
    }

    /// 返回是否支持公式识别。
    #[must_use]
    pub const fn formula(self) -> bool {
        self.formula
    }

    /// 声明处理器能够提取超链接。
    #[must_use]
    pub const fn with_link(mut self) -> Self {
        self.link = true;
        self
    }

    /// 返回是否支持链接提取。
    #[must_use]
    pub const fn link(self) -> bool {
        self.link
    }

    /// 两组能力的布尔并集（任一为 `true` 则结果为 `true`）。
    pub(crate) const fn union(self, other: Self) -> Self {
        Self {
            table_detection: self.table_detection || other.table_detection,
            image_extraction: self.image_extraction || other.image_extraction,
            ocr: self.ocr || other.ocr,
            reading_order: self.reading_order || other.reading_order,
            formula: self.formula || other.formula,
            link: self.link || other.link,
        }
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_all_false() {
        let caps = MarkdownProcessorCapabilities::new();
        assert!(!caps.table_detection());
        assert!(!caps.image_extraction());
        assert!(!caps.ocr());
        assert!(!caps.reading_order());
        assert!(!caps.formula());
        assert!(!caps.link());
    }

    #[test]
    fn default_is_all_false() {
        let caps = MarkdownProcessorCapabilities::default();
        assert!(!caps.table_detection());
        assert!(!caps.image_extraction());
        assert!(!caps.ocr());
    }

    #[test]
    fn with_table_detection() {
        let caps = MarkdownProcessorCapabilities::new().with_table_detection();
        assert!(caps.table_detection());
        assert!(!caps.image_extraction());
    }

    #[test]
    fn with_image_extraction() {
        let caps = MarkdownProcessorCapabilities::new().with_image_extraction();
        assert!(caps.image_extraction());
        assert!(!caps.table_detection());
    }

    #[test]
    fn with_ocr() {
        let caps = MarkdownProcessorCapabilities::new().with_ocr();
        assert!(caps.ocr());
    }

    #[test]
    fn with_reading_order() {
        let caps = MarkdownProcessorCapabilities::new().with_reading_order();
        assert!(caps.reading_order());
    }

    #[test]
    fn with_formula() {
        let caps = MarkdownProcessorCapabilities::new().with_formula();
        assert!(caps.formula());
    }

    #[test]
    fn with_link() {
        let caps = MarkdownProcessorCapabilities::new().with_link();
        assert!(caps.link());
    }

    #[test]
    fn chaining_all_capabilities() {
        let caps = MarkdownProcessorCapabilities::new()
            .with_table_detection()
            .with_image_extraction()
            .with_ocr()
            .with_reading_order()
            .with_formula()
            .with_link();
        assert!(caps.table_detection());
        assert!(caps.image_extraction());
        assert!(caps.ocr());
        assert!(caps.reading_order());
        assert!(caps.formula());
        assert!(caps.link());
    }

    #[test]
    fn union_both_false() {
        let a = MarkdownProcessorCapabilities::new();
        let b = MarkdownProcessorCapabilities::new();
        let u = a.union(b);
        assert!(!u.table_detection());
    }

    #[test]
    fn union_one_true() {
        let a = MarkdownProcessorCapabilities::new().with_table_detection();
        let b = MarkdownProcessorCapabilities::new();
        let u = a.union(b);
        assert!(u.table_detection());
    }

    #[test]
    fn union_both_true() {
        let a = MarkdownProcessorCapabilities::new().with_ocr();
        let b = MarkdownProcessorCapabilities::new().with_ocr();
        let u = a.union(b);
        assert!(u.ocr());
    }

    #[test]
    fn union_mixed() {
        let a = MarkdownProcessorCapabilities::new().with_table_detection();
        let b = MarkdownProcessorCapabilities::new().with_image_extraction();
        let u = a.union(b);
        assert!(u.table_detection());
        assert!(u.image_extraction());
        assert!(!u.ocr());
    }

    #[test]
    fn clone_preserves_values() {
        let caps = MarkdownProcessorCapabilities::new().with_table_detection();
        let cloned = caps;
        assert_eq!(caps, cloned);
    }

    #[test]
    fn partial_eq_works() {
        let a = MarkdownProcessorCapabilities::new().with_ocr();
        let b = MarkdownProcessorCapabilities::new().with_ocr();
        let c = MarkdownProcessorCapabilities::new().with_table_detection();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn debug_format() {
        let caps = MarkdownProcessorCapabilities::new();
        let dbg = format!("{:?}", caps);
        assert!(dbg.contains("MarkdownProcessorCapabilities"));
    }
}
