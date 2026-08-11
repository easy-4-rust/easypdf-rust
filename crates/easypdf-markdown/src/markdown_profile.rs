//! Markdown 输出配置档与管道预设。

use crate::{
    ImagePolicy, OcrPolicy, ProcessorPipeline, TablePolicy,
};

/// Markdown 输出配置档。
///
/// 控制渲染器的输出格式。使用 [`MarkdownProfile::builder()`] 获取
/// 包含完整管道配置的 [`MarkdownProfileBuilder`]。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownProfile {
    /// 标准 GitHub Flavored Markdown。
    #[default]
    Gfm,
    /// 面向大模型分块与引用的输出，显式保留页标题。
    Llm,
    /// 仅保留可读文本，不生成表格等 Markdown 结构。
    Plain,
}

impl MarkdownProfile {
    /// 创建配置构建器，包含空管道与默认策略。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_markdown::MarkdownProfile;
    ///
    /// let builder = MarkdownProfile::builder();
    /// assert_eq!(builder.profile_value(), MarkdownProfile::Gfm);
    /// ```
    #[must_use]
    pub fn builder() -> MarkdownProfileBuilder {
        MarkdownProfileBuilder::new()
    }

    /// 快速预设：仅 Heuristic 级处理器，无 OCR/Cloud。
    ///
    /// 适用于对速度要求高、文档结构简单的场景。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_markdown::MarkdownProfile;
    ///
    /// let config = MarkdownProfile::fast();
    /// assert_eq!(config.profile_value(), MarkdownProfile::Gfm);
    /// assert!(config.pipeline().is_empty());
    /// ```
    #[must_use]
    pub fn fast() -> MarkdownProfileBuilder {
        MarkdownProfileBuilder {
            profile: Self::Gfm,
            pipeline: ProcessorPipeline::new(),
            table_policy: TablePolicy::Detect,
            image_policy: ImagePolicy::Ignore,
            ocr_policy: OcrPolicy::Disabled,
            fail_fast: false,
        }
    }

    /// 均衡预设：Heuristic + Structural 级处理器，本地 OCR。
    ///
    /// 适用于大多数文档转换场景。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_markdown::MarkdownProfile;
    ///
    /// let config = MarkdownProfile::balanced();
    /// assert_eq!(config.profile_value(), MarkdownProfile::Gfm);
    /// ```
    #[must_use]
    pub fn balanced() -> MarkdownProfileBuilder {
        MarkdownProfileBuilder {
            profile: Self::Gfm,
            pipeline: ProcessorPipeline::new(),
            table_policy: TablePolicy::Detect,
            image_policy: ImagePolicy::Reference,
            ocr_policy: OcrPolicy::Auto,
            fail_fast: false,
        }
    }

    /// 全面预设：全部能力等级，含 Cloud OCR。
    ///
    /// 适用于需要最高质量输出的场景（需额外注册 Cloud 级处理器）。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_markdown::MarkdownProfile;
    ///
    /// let config = MarkdownProfile::thorough();
    /// assert_eq!(config.profile_value(), MarkdownProfile::Llm);
    /// ```
    #[must_use]
    pub fn thorough() -> MarkdownProfileBuilder {
        MarkdownProfileBuilder {
            profile: Self::Llm,
            pipeline: ProcessorPipeline::new(),
            table_policy: TablePolicy::Detect,
            image_policy: ImagePolicy::Reference,
            ocr_policy: OcrPolicy::Auto,
            fail_fast: true,
        }
    }
}

/// Markdown 转换管道配置构建器。
///
/// 将 [`MarkdownProfile`]（输出格式）、[`ProcessorPipeline`]（处理器管道）
/// 与各项策略组合为统一配置。可通过 [`MarkdownProfile::builder()`] 或
/// 预设方法（[`fast`](MarkdownProfile::fast)/[`balanced`](MarkdownProfile::balanced)/[`thorough`](MarkdownProfile::thorough)）创建。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::{MarkdownProfile, TablePolicy, OcrPolicy};
///
/// let config = MarkdownProfile::builder()
///     .profile(MarkdownProfile::Llm)
///     .tables(TablePolicy::PlainText)
///     .ocr(OcrPolicy::Auto);
/// assert_eq!(config.profile_value(), MarkdownProfile::Llm);
/// assert_eq!(config.table_policy(), TablePolicy::PlainText);
/// ```
#[derive(Debug)]
pub struct MarkdownProfileBuilder {
    profile: MarkdownProfile,
    pipeline: ProcessorPipeline,
    table_policy: TablePolicy,
    image_policy: ImagePolicy,
    ocr_policy: OcrPolicy,
    fail_fast: bool,
}

impl MarkdownProfileBuilder {
    /// 创建默认配置构建器。
    #[must_use]
    pub fn new() -> Self {
        Self {
            profile: MarkdownProfile::default(),
            pipeline: ProcessorPipeline::new(),
            table_policy: TablePolicy::default(),
            image_policy: ImagePolicy::default(),
            ocr_policy: OcrPolicy::default(),
            fail_fast: false,
        }
    }

    /// 设置输出配置档。
    #[must_use]
    pub const fn profile(mut self, profile: MarkdownProfile) -> Self {
        self.profile = profile;
        self
    }

    /// 设置表格策略。
    #[must_use]
    pub const fn tables(mut self, policy: TablePolicy) -> Self {
        self.table_policy = policy;
        self
    }

    /// 设置图片策略。
    #[must_use]
    pub fn images(mut self, policy: ImagePolicy) -> Self {
        self.image_policy = policy;
        self
    }

    /// 设置 OCR 策略。
    #[must_use]
    pub const fn ocr(mut self, policy: OcrPolicy) -> Self {
        self.ocr_policy = policy;
        self
    }

    /// 设置是否在处理器失败时立即返回错误。
    #[must_use]
    pub const fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// 注册处理器到内部管道。
    #[must_use]
    pub fn processor(mut self, processor: Box<dyn crate::PdfMarkdownProcessor>) -> Self {
        self.pipeline.register(processor);
        self
    }

    /// 注册处理器到内部管道并指定优先级。
    #[must_use]
    pub fn processor_with_priority(
        mut self,
        processor: Box<dyn crate::PdfMarkdownProcessor>,
        priority: f64,
    ) -> Self {
        self.pipeline.register_with_priority(processor, priority);
        self
    }

    /// 返回当前配置的输出配置档。
    #[must_use]
    pub const fn profile_value(&self) -> MarkdownProfile {
        self.profile
    }

    /// 返回当前配置的表格策略。
    #[must_use]
    pub const fn table_policy(&self) -> TablePolicy {
        self.table_policy
    }

    /// 返回当前配置的图片策略。
    #[must_use]
    pub const fn image_policy(&self) -> &ImagePolicy {
        &self.image_policy
    }

    /// 返回当前配置的 OCR 策略。
    #[must_use]
    pub const fn ocr_policy(&self) -> OcrPolicy {
        self.ocr_policy
    }

    /// 返回内部管道的引用。
    #[must_use]
    pub const fn pipeline(&self) -> &ProcessorPipeline {
        &self.pipeline
    }

    /// 返回内部管道的可变引用。
    pub fn pipeline_mut(&mut self) -> &mut ProcessorPipeline {
        &mut self.pipeline
    }

    /// 消费构建器，返回 `(MarkdownProfile, ProcessorPipeline, 策略)` 元组。
    #[must_use]
    pub fn build(self) -> (MarkdownProfile, ProcessorPipeline, BuildPolicies) {
        (
            self.profile,
            self.pipeline,
            BuildPolicies {
                table_policy: self.table_policy,
                image_policy: self.image_policy,
                ocr_policy: self.ocr_policy,
                fail_fast: self.fail_fast,
            },
        )
    }
}

impl Default for MarkdownProfileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建器输出的策略集合。
#[derive(Clone, Debug)]
pub struct BuildPolicies {
    /// 表格策略。
    pub table_policy: TablePolicy,
    /// 图片策略。
    pub image_policy: ImagePolicy,
    /// OCR 策略。
    pub ocr_policy: OcrPolicy,
    /// 是否 fail-fast。
    pub fail_fast: bool,
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let builder = MarkdownProfile::builder();
        assert_eq!(builder.profile_value(), MarkdownProfile::Gfm);
        assert_eq!(builder.table_policy(), TablePolicy::Detect);
        assert_eq!(builder.ocr_policy(), OcrPolicy::Disabled);
        assert!(builder.pipeline().is_empty());
    }

    #[test]
    fn fast_preset() {
        let config = MarkdownProfile::fast();
        assert_eq!(config.profile_value(), MarkdownProfile::Gfm);
        assert_eq!(config.ocr_policy(), OcrPolicy::Disabled);
    }

    #[test]
    fn balanced_preset() {
        let config = MarkdownProfile::balanced();
        assert_eq!(config.profile_value(), MarkdownProfile::Gfm);
        assert_eq!(config.ocr_policy(), OcrPolicy::Auto);
    }

    #[test]
    fn thorough_preset() {
        let config = MarkdownProfile::thorough();
        assert_eq!(config.profile_value(), MarkdownProfile::Llm);
        assert_eq!(config.ocr_policy(), OcrPolicy::Auto);
    }

    #[test]
    fn builder_chaining() {
        let config = MarkdownProfile::builder()
            .profile(MarkdownProfile::Llm)
            .tables(TablePolicy::PlainText)
            .ocr(OcrPolicy::Auto)
            .fail_fast(true);
        assert_eq!(config.profile_value(), MarkdownProfile::Llm);
        assert_eq!(config.table_policy(), TablePolicy::PlainText);
    }

    #[test]
    fn build_returns_tuple() {
        let (profile, pipeline, policies) = MarkdownProfile::balanced().build();
        assert_eq!(profile, MarkdownProfile::Gfm);
        assert!(pipeline.is_empty());
        assert_eq!(policies.ocr_policy, OcrPolicy::Auto);
    }

    #[test]
    fn builder_default_impl() {
        let builder = MarkdownProfileBuilder::default();
        assert_eq!(builder.profile_value(), MarkdownProfile::Gfm);
    }

    #[test]
    fn builder_images_sets_policy() {
        let builder = MarkdownProfile::builder()
            .images(ImagePolicy::Reference);
        assert_eq!(*builder.image_policy(), ImagePolicy::Reference);
    }

    #[test]
    fn builder_fail_fast_sets_flag() {
        let builder = MarkdownProfile::builder().fail_fast(true);
        let (_, _, policies) = builder.build();
        assert!(policies.fail_fast);
    }

    #[test]
    fn builder_pipeline_mut_returns_mutable() {
        let mut builder = MarkdownProfile::builder();
        let _pipeline = builder.pipeline_mut();
        // Just verify it compiles and doesn't panic
    }

    #[test]
    fn fast_preset_has_detect_tables() {
        let config = MarkdownProfile::fast();
        assert_eq!(config.table_policy(), TablePolicy::Detect);
    }

    #[test]
    fn balanced_preset_has_detect_tables() {
        let config = MarkdownProfile::balanced();
        assert_eq!(config.table_policy(), TablePolicy::Detect);
    }

    #[test]
    fn thorough_preset_has_detect_tables() {
        let config = MarkdownProfile::thorough();
        assert_eq!(config.table_policy(), TablePolicy::Detect);
    }

    #[test]
    fn thorough_preset_fail_fast_true() {
        let (_, _, policies) = MarkdownProfile::thorough().build();
        assert!(policies.fail_fast);
    }

    #[test]
    fn balanced_preset_fail_fast_false() {
        let (_, _, policies) = MarkdownProfile::balanced().build();
        assert!(!policies.fail_fast);
    }

    #[test]
    fn build_policies_debug() {
        let (_, _, policies) = MarkdownProfile::balanced().build();
        let dbg = format!("{:?}", policies);
        assert!(dbg.contains("BuildPolicies"));
    }

    #[test]
    fn build_policies_clone() {
        let (_, _, policies) = MarkdownProfile::balanced().build();
        let cloned = policies.clone();
        assert_eq!(policies.ocr_policy, cloned.ocr_policy);
        assert_eq!(policies.fail_fast, cloned.fail_fast);
    }

    #[test]
    fn profile_clone_copy() {
        let p = MarkdownProfile::Llm;
        let copied = p;
        assert_eq!(p, copied);
    }

    #[test]
    fn profile_debug() {
        assert_eq!(format!("{:?}", MarkdownProfile::Gfm), "Gfm");
        assert_eq!(format!("{:?}", MarkdownProfile::Llm), "Llm");
        assert_eq!(format!("{:?}", MarkdownProfile::Plain), "Plain");
    }
}
