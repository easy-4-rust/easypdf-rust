//! Markdown 转换警告。

use easypdf_core::PageIndex;

/// 不阻止转换完成的结构化警告。
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownWarning {
    /// 页面没有可提取的原生文本。
    EmptyPage {
        /// 零基页索引。
        page_index: PageIndex,
    },
    /// 当前后端尚不能执行表格检测。
    TableDetectionUnavailable,
    /// 请求了图片提取，但当前读取后端尚不能提取图片资产。
    ImageExtractionUnavailable,
    /// 请求了 OCR，但没有启用 OCR 实现。
    OcrUnavailable {
        /// 零基页索引。
        page_index: PageIndex,
    },
}
