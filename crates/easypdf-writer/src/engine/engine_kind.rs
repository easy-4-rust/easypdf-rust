//! PDF 写入引擎的公共选择器枚举。

use super::write_engine::WriteEngine;

/// PDF 写入引擎选择。
///
/// 用于 [`PdfWriterBuilder::engine`](crate::PdfWriterBuilder::engine) 方法，
/// 在构建 [`PdfWriter`](crate::PdfWriter) 时指定底层 PDF 引擎。
///
/// # 默认行为
///
/// 不调用 `engine()` 时默认使用 [`Printpdf`](WriteEngineKind::Printpdf)。
///
/// # Examples
///
/// ```
/// use easypdf_writer::{PdfWriterBuilder, WriteEngineKind};
///
/// // 默认引擎（printpdf）。
/// let w = PdfWriterBuilder::new("Default").build().unwrap();
///
/// // 显式选择 printpdf。
/// let w = PdfWriterBuilder::new("Explicit")
///     .engine(WriteEngineKind::Printpdf)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WriteEngineKind {
    /// printpdf 引擎（默认）。
    ///
    /// 支持 PDF 标准 14 内置字体、SVG 和完整的图片格式。
    /// 适用于大多数 PDF 创建场景。
    #[default]
    Printpdf,

    /// krilla 引擎（需要 `writer-krilla` feature）。
    ///
    /// 提供字体子集化和 CJK 文档体积优化。
    /// 限制：不支持 base14 内置字体（需提供真实字体文件）、不支持 SVG。
    #[cfg(feature = "writer-krilla")]
    Krilla,
}

impl WriteEngineKind {
    /// 创建对应的写入引擎实例。
    pub(crate) fn create_engine(self, title: &str) -> Box<dyn WriteEngine> {
        match self {
            Self::Printpdf => Box::new(super::printpdf_engine::PrintpdfEngine::new(title)),
            #[cfg(feature = "writer-krilla")]
            Self::Krilla => Box::new(super::krilla_engine::KrillaEngine::new()),
        }
    }
}
