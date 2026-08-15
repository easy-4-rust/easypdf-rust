//! PDF 渲染错误类型。

use std::path::PathBuf;

/// PDF 页面渲染过程中可能发生的错误。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// 发生 I/O 错误（读取 PDF 或写入输出）。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// PDF 无法解析或格式错误。
    #[error("PDF parse error: {0}")]
    Parse(String),

    /// 请求的页码索引超出范围。
    #[error("page index {index} out of bounds (total {total})")]
    InvalidPage {
        /// 请求的页码索引（从 0 开始）。
        index: usize,
        /// 文档总页数。
        total: usize,
    },

    /// 请求的渲染后端不可用。
    #[error("render backend '{name}' is not available: {reason}")]
    BackendUnavailable {
        /// 后端名称。
        name: &'static str,
        /// 不可用原因。
        reason: String,
    },

    /// pdfium 动态库无法加载。
    #[cfg(feature = "pdfium")]
    #[error("pdfium library error: {0}")]
    Pdfium(String),

    /// 图像编码失败（PNG/JPEG）。
    #[error("image encoding error: {0}")]
    ImageEncode(String),

    /// 请求的 DPI 超过后端最大值。
    #[error("DPI {requested} exceeds backend maximum {max}")]
    DpiExceeded {
        /// 请求的 DPI。
        requested: u32,
        /// 后端最大 DPI。
        max: u32,
    },

    /// 输出路径不是有效的目标。
    #[error("invalid output path: {0}")]
    InvalidOutput(PathBuf),

    /// 其他渲染错误的兜底变体。
    #[error("{0}")]
    Other(String),
}

/// 渲染操作的便捷 `Result` 类型。
pub type Result<T, E = RenderError> = std::result::Result<T, E>;
