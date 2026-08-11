//! OCR 回退策略。

/// OCR 回退策略。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OcrPolicy {
    /// 不执行 OCR。
    #[default]
    Disabled,
    /// 仅在页面缺少原生文本时尝试 OCR。
    Auto,
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn default_is_disabled() {
        assert_eq!(OcrPolicy::default(), OcrPolicy::Disabled);
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", OcrPolicy::Disabled), "Disabled");
        assert_eq!(format!("{:?}", OcrPolicy::Auto), "Auto");
    }

    #[test]
    fn clone_copy() {
        let p = OcrPolicy::Auto;
        let copied = p;
        assert_eq!(p, copied);
    }

    #[test]
    fn partial_eq() {
        assert_eq!(OcrPolicy::Disabled, OcrPolicy::Disabled);
        assert_ne!(OcrPolicy::Disabled, OcrPolicy::Auto);
    }
}
