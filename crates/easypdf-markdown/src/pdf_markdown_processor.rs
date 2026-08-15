//! PDF 到 Markdown 的可插拔语义处理器。

use easypdf_core::PdfDocumentModel;
use easypdf_core::PdfInput;
use easypdf_core::Result;

use crate::{MarkdownProcessorCapabilities, MarkdownWarning};

/// 在基础文本提取之后、Markdown 渲染之前增强语义文档模型。
///
/// 实现可对接表格检测、图片提取、页面渲染 OCR 或领域专用结构识别。
/// 处理器接收原始输入描述和上一个处理器产出的模型，并返回新的模型；
/// 因而多个处理器可按注册顺序确定性组合，且无需让核心 crate 依赖具体 OCR/AI SDK。
pub trait PdfMarkdownProcessor: Send + Sync {
    /// 返回处理器能够真正提供的能力。
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        MarkdownProcessorCapabilities::new()
    }

    /// 增强语义模型并返回本处理器产生的非致命警告。
    ///
    /// # Errors
    ///
    /// 后端无法读取输入或无法完成声明的处理时返回错误。
    fn process(
        &self,
        input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)>;
}
