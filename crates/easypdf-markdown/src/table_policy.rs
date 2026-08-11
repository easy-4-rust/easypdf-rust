//! PDF 表格转换策略。

/// PDF 表格转换策略。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TablePolicy {
    /// 尝试识别并输出 GFM 表格。
    #[default]
    Detect,
    /// 将表格降级为普通文本。
    PlainText,
    /// 忽略表格。
    Ignore,
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn default_is_detect() {
        assert_eq!(TablePolicy::default(), TablePolicy::Detect);
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", TablePolicy::Detect), "Detect");
        assert_eq!(format!("{:?}", TablePolicy::PlainText), "PlainText");
        assert_eq!(format!("{:?}", TablePolicy::Ignore), "Ignore");
    }

    #[test]
    fn clone_copy() {
        let p = TablePolicy::PlainText;
        let copied = p;
        assert_eq!(p, copied);
    }

    #[test]
    fn partial_eq() {
        assert_eq!(TablePolicy::Detect, TablePolicy::Detect);
        assert_ne!(TablePolicy::Detect, TablePolicy::Ignore);
    }
}
