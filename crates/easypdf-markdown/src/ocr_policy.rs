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
