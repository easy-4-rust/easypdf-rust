//! PDF 图片转换策略。

use std::path::PathBuf;

/// PDF 图片转换策略。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImagePolicy {
    /// 忽略图片。
    #[default]
    Ignore,
    /// 保留已有图片引用。
    Reference,
    /// 将图片提取到指定目录。
    ExtractTo(PathBuf),
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ignore() {
        assert_eq!(ImagePolicy::default(), ImagePolicy::Ignore);
    }

    #[test]
    fn clone_preserves() {
        let p = ImagePolicy::ExtractTo(PathBuf::from("/tmp/images"));
        let cloned = p.clone();
        assert_eq!(p, cloned);
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", ImagePolicy::Ignore), "Ignore");
        assert_eq!(format!("{:?}", ImagePolicy::Reference), "Reference");
    }

    #[test]
    fn partial_eq_works() {
        assert_eq!(ImagePolicy::Ignore, ImagePolicy::Ignore);
        assert_ne!(ImagePolicy::Ignore, ImagePolicy::Reference);
    }
}
