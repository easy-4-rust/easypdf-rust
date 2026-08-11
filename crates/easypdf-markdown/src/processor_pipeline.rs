//! 语义处理器管道调度器。

use easypdf_core::Result;
use easypdf_core::PdfInput;
use easypdf_core::PdfDocumentModel;

use crate::{
    DetailedProcessorCapabilities, MarkdownProcessorCapabilities, MarkdownWarning,
    PdfMarkdownProcessor,
};

/// 处理器管道：按优先级确定性组合多个 [`PdfMarkdownProcessor`]。
///
/// 多个处理器按 `priority` 升序排列后依次执行；前一个处理器的输出
/// 作为后一个的输入。相同优先级的处理器保持注册顺序（稳定排序）。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ProcessorPipeline;
///
/// let pipeline = ProcessorPipeline::new();
/// assert!(pipeline.is_empty());
/// assert_eq!(pipeline.len(), 0);
/// ```
#[derive(Debug)]
pub struct ProcessorPipeline {
    /// `(priority, processor)` 对，`run()` 前按 priority 稳定排序。
    entries: Vec<PipelineEntry>,
    /// 目标能力等级；处理器能力低于目标时可被跳过。
    target_level: Option<DetailedProcessorCapabilities>,
    /// 单个处理器失败时是否立即返回错误（默认 `false`，收集到 warnings）。
    fail_fast: bool,
}

/// 管道中的单个处理器条目。
struct PipelineEntry {
    priority: f64,
    processor: Box<dyn PdfMarkdownProcessor>,
    registration_order: usize,
}

impl std::fmt::Debug for PipelineEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineEntry")
            .field("priority", &self.priority)
            .field("registration_order", &self.registration_order)
            .finish_non_exhaustive()
    }
}

/// 默认优先级常量：对齐 markitdown 的 `PRIORITY_SPECIFIC_FILE_FORMAT`。
///
/// 用于需要最先执行的特定格式处理器。
pub const PRIORITY_SPECIFIC: f64 = 0.0;

/// 默认优先级常量：对齐 markitdown 的 `PRIORITY_GENERIC_FILE_FORMAT`。
pub const PRIORITY_GENERIC: f64 = 10.0;

impl ProcessorPipeline {
    /// 创建空管道。
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            target_level: None,
            fail_fast: false,
        }
    }

    /// 注册处理器，使用默认优先级 [`PRIORITY_GENERIC`]（`10.0`）。
    ///
    /// 返回 `&mut Self` 以支持链式调用。
    pub fn register(&mut self, processor: Box<dyn PdfMarkdownProcessor>) -> &mut Self {
        self.register_with_priority(processor, PRIORITY_GENERIC)
    }

    /// 注册处理器并指定优先级（值越小越先执行）。
    ///
    /// 对齐 markitdown 的 priority 机制：
    /// - [`PRIORITY_SPECIFIC`]（`0.0`）：特定格式处理器，优先执行
    /// - [`PRIORITY_GENERIC`]（`10.0`）：通用处理器，后执行
    ///
    /// 返回 `&mut Self` 以支持链式调用。
    pub fn register_with_priority(
        &mut self,
        processor: Box<dyn PdfMarkdownProcessor>,
        priority: f64,
    ) -> &mut Self {
        let order = self.entries.len();
        self.entries.push(PipelineEntry {
            priority,
            processor,
            registration_order: order,
        });
        self
    }

    /// 设置目标能力等级，用于降级策略。
    ///
    /// 处理器的能力低于目标等级时仍会执行（不会被跳过），
    /// 但 [`aggregate_capabilities`](Self::aggregate_capabilities)
    /// 返回的结果可用于判断是否满足目标。
    #[must_use]
    pub fn with_target_level(mut self, target: DetailedProcessorCapabilities) -> Self {
        self.target_level = Some(target);
        self
    }

    /// 设置是否在单个处理器失败时立即返回错误。
    ///
    /// 默认为 `false`：处理器错误会被收集到 warnings 中，管道继续执行。
    /// 设为 `true` 时，第一个处理器错误即终止管道。
    #[must_use]
    pub const fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// 执行管道：按优先级升序依次调用每个处理器的 `process()`。
    ///
    /// 返回最终的文档模型与所有处理器产生的警告。
    ///
    /// # Errors
    ///
    /// 当 `fail_fast` 为 `true` 且任一处理器返回错误时，立即传播该错误。
    /// 当 `fail_fast` 为 `false` 时，处理器错误被转为警告，管道继续执行。
    pub fn run(
        &mut self,
        input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
        // 按 priority 稳定排序（升序），同 priority 保持注册顺序。
        self.entries.sort_by(|a, b| {
            a.priority
                .partial_cmp(&b.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.registration_order.cmp(&b.registration_order))
        });

        let mut current_doc = document;
        let mut all_warnings = Vec::new();

        for entry in &self.entries {
            if self.fail_fast {
                // fail_fast 模式：直接传播错误。
                let (processed, mut warnings) =
                    entry.processor.process(input, current_doc)?;
                current_doc = processed;
                all_warnings.append(&mut warnings);
            } else {
                // 宽容模式：处理器失败时保留当前文档，收集警告。
                // 需要 clone 以在失败时保留文档。
                let doc_snapshot = current_doc.clone();
                match entry.processor.process(input, current_doc) {
                    Ok((processed, mut warnings)) => {
                        current_doc = processed;
                        all_warnings.append(&mut warnings);
                    }
                    Err(err) => {
                        current_doc = doc_snapshot;
                        all_warnings.push(MarkdownWarning::ProcessorFailed {
                            message: err.to_string(),
                        });
                    }
                }
            }
        }

        Ok((current_doc, all_warnings))
    }

    /// 聚合所有处理器的能力（每项取最高 level）。
    ///
    /// 遍历已注册的处理器，调用其 `capabilities()` 并取并集。
    /// 结果以 [`DetailedProcessorCapabilities`] 返回。
    #[must_use]
    pub fn aggregate_capabilities(&self) -> DetailedProcessorCapabilities {
        let mut merged = DetailedProcessorCapabilities::new();
        for entry in &self.entries {
            let caps = entry.processor.capabilities();
            let detailed = DetailedProcessorCapabilities::from(caps);
            merged = merged.merge(&detailed);
        }
        merged
    }

    /// 聚合所有处理器的能力（布尔值版本，保持向后兼容）。
    #[must_use]
    pub fn aggregate_bool_capabilities(&self) -> MarkdownProcessorCapabilities {
        let mut merged = MarkdownProcessorCapabilities::new();
        for entry in &self.entries {
            merged = merged.union(entry.processor.capabilities());
        }
        merged
    }

    /// 返回已注册的处理器数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 判断管道是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 返回目标能力等级（如果已设置）。
    #[must_use]
    pub const fn target_level(&self) -> Option<&DetailedProcessorCapabilities> {
        self.target_level.as_ref()
    }
}

impl Default for ProcessorPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;
    use easypdf_core::{PageIndex, PdfMetadata};
    use easypdf_core::{PdfBlock, PdfPageModel, SourceLocation};

    /// 简单测试处理器：在文档中追加一个段落。
    struct AppendProcessor {
        text: String,
    }

    impl PdfMarkdownProcessor for AppendProcessor {
        fn process(
            &self,
            _input: &PdfInput,
            document: PdfDocumentModel,
        ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
            let loc = SourceLocation::new(PageIndex::new(0), 1.0);
            let page = PdfPageModel::new(PageIndex::new(0))
                .with_block(PdfBlock::paragraph(&self.text, loc));
            Ok((
                PdfDocumentModel::new(document.metadata().clone(), vec![page]),
                Vec::new(),
            ))
        }
    }

    /// 总是返回错误的处理器。
    struct FailProcessor;

    impl PdfMarkdownProcessor for FailProcessor {
        fn process(
            &self,
            _input: &PdfInput,
            _document: PdfDocumentModel,
        ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
            Err(easypdf_core::PdfError::Other("test failure".into()))
        }
    }

    fn empty_doc() -> PdfDocumentModel {
        PdfDocumentModel::new(PdfMetadata::default(), Vec::new())
    }

    fn empty_input() -> PdfInput {
        PdfInput::from_bytes(Vec::new())
    }

    #[test]
    fn empty_pipeline_returns_unchanged() {
        let mut pipeline = ProcessorPipeline::new();
        let doc = empty_doc();
        let (result, warnings) = pipeline.run(&empty_input(), doc).unwrap();
        assert!(result.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn processors_execute_in_priority_order() {
        let mut pipeline = ProcessorPipeline::new();
        // 注册顺序：generic (10.0) 先, specific (0.0) 后
        pipeline.register(Box::new(AppendProcessor {
            text: "generic".into(),
        }));
        pipeline.register_with_priority(
            Box::new(AppendProcessor {
                text: "specific".into(),
            }),
            0.0,
        );
        // 排序后 specific (0.0) 先执行，generic (10.0) 后执行
        // 后执行的覆盖前一个的输出（因为 AppendProcessor 替换整个文档）
        let doc = empty_doc();
        let (result, _) = pipeline.run(&empty_input(), doc).unwrap();
        // generic 后执行，所以它的文本出现在最终结果中
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
        if let PdfBlock::Paragraph { text, .. } = blocks[0].1 {
            assert_eq!(text, "generic");
        } else {
            panic!("expected Paragraph");
        }
    }

    #[test]
    fn fail_fast_returns_error() {
        let mut pipeline = ProcessorPipeline::new().fail_fast(true);
        pipeline.register(Box::new(FailProcessor));
        let doc = empty_doc();
        let result = pipeline.run(&empty_input(), doc);
        assert!(result.is_err());
    }

    #[test]
    fn fail_collects_warning() {
        let mut pipeline = ProcessorPipeline::new();
        pipeline.register(Box::new(FailProcessor));
        let doc = empty_doc();
        let (_, warnings) = pipeline.run(&empty_input(), doc).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            MarkdownWarning::ProcessorFailed { .. }
        ));
    }

    #[test]
    fn len_and_is_empty() {
        let mut pipeline = ProcessorPipeline::new();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
        pipeline.register(Box::new(AppendProcessor {
            text: "x".into(),
        }));
        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.len(), 1);
    }

    #[test]
    fn aggregate_capabilities_merges() {
        let mut pipeline = ProcessorPipeline::new();
        pipeline.register(Box::new(AppendProcessor {
            text: "x".into(),
        }));
        let caps = pipeline.aggregate_capabilities();
        // AppendProcessor 的 capabilities() 返回默认（全 None）
        assert!(!caps.supports(crate::ProcessorCapability::TableDetection));
    }

    #[test]
    fn default_pipeline_fail_fast_false() {
        let pipeline = ProcessorPipeline::new();
        assert!(!pipeline.fail_fast);
    }

    #[test]
    fn fail_fast_setter() {
        let pipeline = ProcessorPipeline::new().fail_fast(true);
        assert!(pipeline.fail_fast);
    }

    #[test]
    fn new_is_empty() {
        let pipeline = ProcessorPipeline::new();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[test]
    fn register_increases_len() {
        let mut pipeline = ProcessorPipeline::new();
        pipeline.register(Box::new(AppendProcessor { text: "a".into() }));
        pipeline.register(Box::new(AppendProcessor { text: "b".into() }));
        assert_eq!(pipeline.len(), 2);
        assert!(!pipeline.is_empty());
    }

    #[test]
    fn multiple_processors_execute() {
        let mut pipeline = ProcessorPipeline::new();
        // Both at same priority (10.0), execution order is stable
        pipeline.register(Box::new(AppendProcessor { text: "first".into() }));
        pipeline.register(Box::new(AppendProcessor { text: "second".into() }));
        let doc = empty_doc();
        let (result, warnings) = pipeline.run(&empty_input(), doc).unwrap();
        // Last processor wins since AppendProcessor replaces doc
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn fail_fast_stops_on_first_error() {
        let mut pipeline = ProcessorPipeline::new().fail_fast(true);
        pipeline.register(Box::new(FailProcessor));
        pipeline.register(Box::new(AppendProcessor { text: "never".into() }));
        let doc = empty_doc();
        let result = pipeline.run(&empty_input(), doc);
        assert!(result.is_err());
    }

    #[test]
    fn no_fail_fast_continues_after_error() {
        let mut pipeline = ProcessorPipeline::new().fail_fast(false);
        pipeline.register(Box::new(FailProcessor));
        pipeline.register(Box::new(AppendProcessor { text: "continued".into() }));
        let doc = empty_doc();
        let (result, warnings) = pipeline.run(&empty_input(), doc).unwrap();
        // Second processor still runs
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(warnings.len(), 1);
    }

    // --- Additional coverage tests ---

    #[test]
    fn with_target_level_stores_value() {
        let caps = DetailedProcessorCapabilities::new();
        let pipeline = ProcessorPipeline::new().with_target_level(caps);
        assert!(pipeline.target_level().is_some());
    }

    #[test]
    fn target_level_none_by_default() {
        let pipeline = ProcessorPipeline::new();
        assert!(pipeline.target_level().is_none());
    }

    #[test]
    fn aggregate_bool_capabilities_merges() {
        let mut pipeline = ProcessorPipeline::new();
        pipeline.register(Box::new(AppendProcessor {
            text: "x".into(),
        }));
        let caps = pipeline.aggregate_bool_capabilities();
        // AppendProcessor returns default capabilities
        assert!(!caps.ocr());
    }

    #[test]
    fn default_creates_empty_pipeline() {
        let pipeline = ProcessorPipeline::default();
        assert!(pipeline.is_empty());
    }

    #[test]
    fn same_priority_preserves_registration_order() {
        let mut pipeline = ProcessorPipeline::new();
        pipeline.register(Box::new(AppendProcessor { text: "first".into() }));
        pipeline.register(Box::new(AppendProcessor { text: "second".into() }));
        // Both at PRIORITY_GENERIC (10.0), second should run last
        let doc = empty_doc();
        let (result, _) = pipeline.run(&empty_input(), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
        if let PdfBlock::Paragraph { text, .. } = blocks[0].1 {
            assert_eq!(text, "second");
        }
    }
}
