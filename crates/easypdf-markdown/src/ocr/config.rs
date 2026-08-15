//! OCR 配置与触发策略。

/// OCR 处理器配置。
///
/// 控制渲染 DPI、OCR 触发条件和质量阈值。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::{OcrConfig, OcrTrigger};
///
/// let config = OcrConfig {
///     render_dpi: 300,
///     trigger: OcrTrigger::Always,
///     min_confidence: 0.8,
///     ..OcrConfig::default()
/// };
/// assert_eq!(config.render_dpi, 300);
/// ```
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// 页面转图像的渲染 DPI。默认值：200。
    ///
    /// 值越高 OCR 准确度越好，但内存占用和处理时间也越大。
    /// 200 DPI 对大多数文档是较好的平衡点。
    pub render_dpi: u32,

    /// OCR 触发条件。默认值：[`OcrTrigger::OnEmptyPage`]。
    pub trigger: OcrTrigger,

    /// 从 OCR 结果中保留的最小文本长度。默认值：0（全部保留）。
    ///
    /// 短于此阈值的 OCR 结果将被视为噪声而丢弃。
    pub min_text_length: usize,

    /// 最小置信度阈值。默认值：0.5。
    ///
    /// 置信度低于此值的 OCR 结果将生成警告。
    pub min_confidence: f32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            render_dpi: 200,
            trigger: OcrTrigger::OnEmptyPage,
            min_text_length: 0,
            min_confidence: 0.5,
        }
    }
}

/// OCR 触发条件。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::OcrTrigger;
///
/// let trigger = OcrTrigger::WhenTextSparse { threshold: 0.3 };
/// assert!(matches!(trigger, OcrTrigger::WhenTextSparse { threshold } if threshold == 0.3));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum OcrTrigger {
    /// 始终对每一页执行 OCR。
    Always,

    /// 仅在页面缺少可提取的原生文本块时执行 OCR。
    ///
    /// 这是默认值，也是主要用例：扫描件 PDF 中文本提取器返回空页面时触发。
    #[default]
    OnEmptyPage,

    /// 当文本块占总块数的比例低于阈值时执行 OCR。
    ///
    /// `threshold` 取值范围为 `0.0..=1.0`。当
    /// `(text_block_count / total_block_count) < threshold` 时视为文本稀疏。
    WhenTextSparse {
        /// 文本块与总块数的比例阈值（0.0 到 1.0）。
        threshold: f32,
    },
}
