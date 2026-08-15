//! 定义 `easypdf-rust` 核心扩展点的 trait。
//!
//! 包含 `PdfModel`、`PdfReadListener`、`PdfWriteHandler`、`PdfConverter`、
//! `PdfEngine`、`EngineCapabilities` 和 `CapabilityLevel`。

use crate::error::Result;

/// 可渲染为 PDF 内容的类型 trait。
///
/// 通常通过 `#[derive(PdfModel)]` 派生而非手动实现。
/// 将 Rust 结构体映射为 PDF 元素。
pub trait PdfModel {
    /// 将此模型渲染为 PDF 内容。
    ///
    /// 实现由派生宏生成，处理布局、定位和样式应用。
    ///
    /// # Errors
    ///
    /// 模型转换或内容渲染失败时返回错误。
    fn render(&self) -> Result<Vec<RenderedElement>>;

    /// 返回此模型的元数据（页面尺寸、方向、边距等）。
    fn metadata(&self) -> PdfModelMetadata;

    /// 返回表单填写和数据映射场景的字段描述符。
    ///
    /// 每个标注了 `#[pdf(field = "...")]` 或类似属性的字段
    /// 会生成一个描述符。默认实现返回空向量。
    fn field_descriptors(&self) -> Vec<PdfFieldDescriptor> {
        Vec::new()
    }
}

/// [`PdfModel`] 中单个字段的描述符，由派生宏生成。
///
/// 携带 `#[pdf(field = "...", order = N, ...)]` 属性的元数据，
/// 用于模板填充和验证逻辑。
#[derive(Debug, Clone)]
pub struct PdfFieldDescriptor {
    /// 映射到的 PDF 表单字段名称。
    pub field_name: String,
    /// Rust 字段名称。
    pub rust_field_name: String,
    /// 显示顺序（值越小越靠前）。
    pub order: u32,
    /// 可选的格式字符串（如日期的 "YYYY-MM-DD"）。
    pub format: Option<String>,
    /// 默认值表达式（字符串形式），如果指定。
    pub default_value: Option<String>,
    /// 此字段是否必填。
    pub required: bool,
    /// 是否为嵌套子模型。
    pub nested: bool,
}

impl PdfFieldDescriptor {
    /// 使用最少的必需字段创建新的字段描述符。
    #[must_use]
    pub fn new(field_name: impl Into<String>, rust_field_name: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            rust_field_name: rust_field_name.into(),
            order: u32::MAX,
            format: None,
            default_value: None,
            required: false,
            nested: false,
        }
    }
}

/// 与 `PdfModel` 关联的元数据（通常来自 `#[pdf(...)]` 属性）。
#[derive(Debug, Clone)]
pub struct PdfModelMetadata {
    /// 模型的页面尺寸。
    pub page_size: crate::enums::PageSize,
    /// 页面方向。
    pub orientation: crate::enums::Orientation,
    /// 页面边距（点）。
    pub margins: f64,
}

impl Default for PdfModelMetadata {
    fn default() -> Self {
        Self {
            page_size: crate::enums::PageSize::A4,
            orientation: crate::enums::Orientation::default(),
            margins: 72.0,
        }
    }
}

/// 由 [`PdfModel`] 生成的渲染元素。
///
/// 这是 `PdfModel::render()` 的输出，表示 PDF 页面上
/// 一个已定位的内容片段。
#[derive(Debug, Clone)]
pub enum RenderedElement {
    /// 位于给定 (x, y) 位置的文本元素。
    Text {
        /// 从左下角算起的 x 坐标（PDF 点）。
        x: f64,
        /// 从左下角算起的 y 坐标（PDF 点）。
        y: f64,
        /// 带格式的文本内容。
        text: crate::content::PdfText,
    },
    /// 位于给定 (x, y) 位置的表格。
    Table {
        /// 表格左上角的 x 坐标。
        x: f64,
        /// 表格左上角的 y 坐标。
        y: f64,
        /// 表格数据和配置。
        table: crate::content::PdfTable,
    },
    /// 位于给定 (x, y) 位置的图片。
    Image {
        /// 从左下角算起的 x 坐标。
        x: f64,
        /// 从左下角算起的 y 坐标。
        y: f64,
        /// 图片数据。
        image: crate::content::PdfImage,
    },
}

// --- PdfReadListener ---

/// PDF 读取操作的事件驱动监听器。
///
/// 类似于 easyexcel-rs 中的 `ReadListener<T>`。在读取过程中
/// 遇到每个页面或文本块时被调用。
///
/// 此 trait 要求 `Send`，以便监听器可在异步或并行读取管道中
/// 跨线程边界使用。持有非 `Send` 状态的实现者应将其包装在
/// `Mutex` 或类似结构中。
pub trait PdfReadListener: Send {
    /// 页面开始处理时调用。
    ///
    /// # Errors
    ///
    /// 实现可通过返回错误来停止处理。
    fn on_page_start(&mut self, page_number: usize) -> Result<()> {
        let _ = page_number;
        Ok(())
    }

    /// 从页面提取的每个文本块被调用。
    ///
    /// # Errors
    ///
    /// 实现可通过返回错误来停止处理。
    fn on_text(&mut self, page_number: usize, text: &str) -> Result<()>;

    /// 页面处理完成时调用。
    ///
    /// # Errors
    ///
    /// 实现可通过返回错误来停止处理。
    fn on_page_end(&mut self, page_number: usize) -> Result<()> {
        let _ = page_number;
        Ok(())
    }

    /// 所有页面处理完成后调用。
    ///
    /// # Errors
    ///
    /// 实现可报告终结化失败。
    fn on_document_end(&mut self) -> Result<()> {
        Ok(())
    }
}

// --- PdfWriteHandler ---

/// PDF 写入操作的生命周期钩子。
///
/// 类似于 easyexcel-rs 中的 `WriteHandler`。处理程序在文档创建的
/// 每个阶段按优先级顺序被调用。
pub trait PdfWriteHandler: Send {
    /// 文档创建前调用。
    ///
    /// # Errors
    ///
    /// 实现可中止文档创建。
    fn before_document(&mut self) -> Result<()> {
        Ok(())
    }

    /// 新页面开始前调用。
    ///
    /// # Errors
    ///
    /// 实现可中止页面创建。
    fn before_page(&mut self, page_number: usize) -> Result<()> {
        let _ = page_number;
        Ok(())
    }

    /// 页面完成后调用。
    ///
    /// # Errors
    ///
    /// 实现可报告页面终结化失败。
    fn after_page(&mut self, page_number: usize) -> Result<()> {
        let _ = page_number;
        Ok(())
    }

    /// 文档终结化后调用。
    ///
    /// # Errors
    ///
    /// 实现可报告文档终结化失败。
    fn after_document(&mut self) -> Result<()> {
        Ok(())
    }
}

// --- PdfConverter ---

/// Rust 类型 `T` 与 PDF 字符串表示之间的双向转换器。
///
/// 类似于 easyexcel-rs 中的 `Converter<T>`。
pub trait PdfConverter<T>: Send {
    /// 将 Rust 值转换为其 PDF 字符串表示。
    ///
    /// # Errors
    ///
    /// 值无法表示时返回错误。
    fn to_pdf_string(&self, value: &T) -> Result<String>;

    /// 将 PDF 字符串表示转换回 Rust 值。
    ///
    /// # Errors
    ///
    /// 字符串无法解析时返回错误。
    #[allow(clippy::wrong_self_convention)]
    fn from_pdf_string(&self, s: &str) -> Result<T>;
}

/// 全面实现，使 `Box<dyn PdfConverter<T>>` 可在任何期望
/// `PdfConverter<T>` 的地方使用（如 `ConverterRegistry::register`）。
impl<T> PdfConverter<T> for Box<dyn PdfConverter<T>> {
    fn to_pdf_string(&self, value: &T) -> Result<String> {
        (**self).to_pdf_string(value)
    }

    fn from_pdf_string(&self, s: &str) -> Result<T> {
        (**self).from_pdf_string(s)
    }
}

// --- PdfEngine (C1) ---

/// 用于后端切换的抽象 PDF 引擎接口。
///
/// 允许不同的 PDF 后端（lopdf、printpdf、justpdf）互换使用。
/// 目前处于实验阶段——完整的抽象等待第二个成熟引擎实现。
pub trait PdfEngine: Send + Sync {
    /// 人类可读的引擎名称。
    fn name(&self) -> &str;

    /// 此引擎支持的功能。
    fn capabilities(&self) -> EngineCapabilities;
}

/// 描述 PDF 引擎支持哪些操作。
#[derive(Debug, Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct EngineCapabilities {
    /// 可以从零创建新的 PDF 文档。
    pub create: bool,
    /// 可以读取和解析现有 PDF 文档。
    pub read: bool,
    /// 可以操作（合并、拆分、旋转）现有 PDF。
    pub manipulate: bool,
    /// 可以填写 PDF 模板中的表单字段。
    pub fill_forms: bool,
    /// 支持加密。
    pub encrypt: bool,
    /// 支持数字签名。
    pub sign: bool,
    /// 支持 PDF/A 验证。
    pub pdfa: bool,
}

impl EngineCapabilities {
    /// lopdf 的能力集。
    #[must_use]
    pub const fn lopdf() -> Self {
        Self {
            create: false,
            read: true,
            manipulate: true,
            fill_forms: true,
            encrypt: false,
            sign: false,
            pdfa: false,
        }
    }

    /// printpdf 的能力集。
    #[must_use]
    pub const fn printpdf() -> Self {
        Self {
            create: true,
            read: false,
            manipulate: false,
            fill_forms: false,
            encrypt: false,
            sign: false,
            pdfa: false,
        }
    }

    /// 转换为使用 [`CapabilityLevel`] 值的 [`DetailedEngineCapabilities`]。
    ///
    /// 布尔值 `true` 映射到 [`CapabilityLevel::Structural`]；
    /// `false` 映射到 [`CapabilityLevel::None`]。
    #[must_use]
    pub const fn to_detailed(&self) -> DetailedEngineCapabilities {
        DetailedEngineCapabilities {
            text_extraction: if self.read {
                CapabilityLevel::Structural
            } else {
                CapabilityLevel::None
            },
            metadata: if self.read {
                CapabilityLevel::Structural
            } else {
                CapabilityLevel::None
            },
            image_extraction: if self.read {
                CapabilityLevel::Heuristic
            } else {
                CapabilityLevel::None
            },
            table_detection: if self.read {
                CapabilityLevel::Heuristic
            } else {
                CapabilityLevel::None
            },
            rendering: if self.create {
                CapabilityLevel::Structural
            } else {
                CapabilityLevel::None
            },
        }
    }
}

/// 用于细粒度引擎功能报告的渐进式能力级别。
///
/// 与布尔值不同，这传达了引擎对某项功能的支持*程度*，
/// 使下游代码（如 markdown 处理器链）能选择最佳可用引擎
/// 或优雅降级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum CapabilityLevel {
    /// 不支持该功能。
    #[default]
    None,
    /// 通过启发式或近似方法支持该功能。
    Heuristic,
    /// 以完整的结构精度支持该功能。
    Structural,
    /// 通过云端加速或 AI 辅助处理支持该功能。
    Cloud,
}

impl CapabilityLevel {
    /// 返回此级别是否代表任何程度的支持（非 `None`）。
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        !matches!(self, Self::None)
    }
}

/// 使用渐进式 [`CapabilityLevel`] 值的详细引擎能力。
///
/// 这补充了基于布尔值的 [`EngineCapabilities`]，
/// 适用于支持*质量*很重要的场景（如在 markdown 处理器链中
/// 选择提取策略）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DetailedEngineCapabilities {
    /// 文本提取支持的质量。
    pub text_extraction: CapabilityLevel,
    /// 元数据（XMP、文档信息）提取的质量。
    pub metadata: CapabilityLevel,
    /// 图片提取支持的质量。
    pub image_extraction: CapabilityLevel,
    /// 表格检测和提取的质量。
    pub table_detection: CapabilityLevel,
    /// 渲染（文档创建）支持的质量。
    pub rendering: CapabilityLevel,
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn capability_level_ordering() {
        assert!(CapabilityLevel::None < CapabilityLevel::Heuristic);
        assert!(CapabilityLevel::Heuristic < CapabilityLevel::Structural);
        assert!(CapabilityLevel::Structural < CapabilityLevel::Cloud);
    }

    #[test]
    fn capability_level_default_is_none() {
        assert_eq!(CapabilityLevel::default(), CapabilityLevel::None);
    }

    #[test]
    fn capability_level_is_supported() {
        assert!(!CapabilityLevel::None.is_supported());
        assert!(CapabilityLevel::Heuristic.is_supported());
        assert!(CapabilityLevel::Structural.is_supported());
        assert!(CapabilityLevel::Cloud.is_supported());
    }

    #[test]
    fn capability_level_debug() {
        assert_eq!(format!("{:?}", CapabilityLevel::Cloud), "Cloud");
    }

    #[test]
    fn detailed_capabilities_default() {
        let dc = DetailedEngineCapabilities::default();
        assert_eq!(dc.text_extraction, CapabilityLevel::None);
        assert_eq!(dc.metadata, CapabilityLevel::None);
        assert_eq!(dc.image_extraction, CapabilityLevel::None);
        assert_eq!(dc.table_detection, CapabilityLevel::None);
        assert_eq!(dc.rendering, CapabilityLevel::None);
    }

    #[test]
    fn engine_capabilities_to_detailed_lopdf() {
        let caps = EngineCapabilities::lopdf();
        let detailed = caps.to_detailed();
        assert_eq!(detailed.text_extraction, CapabilityLevel::Structural);
        assert_eq!(detailed.metadata, CapabilityLevel::Structural);
        assert_eq!(detailed.image_extraction, CapabilityLevel::Heuristic);
        assert_eq!(detailed.table_detection, CapabilityLevel::Heuristic);
        assert_eq!(detailed.rendering, CapabilityLevel::None);
    }

    #[test]
    fn engine_capabilities_to_detailed_printpdf() {
        let caps = EngineCapabilities::printpdf();
        let detailed = caps.to_detailed();
        assert_eq!(detailed.text_extraction, CapabilityLevel::None);
        assert_eq!(detailed.metadata, CapabilityLevel::None);
        assert_eq!(detailed.image_extraction, CapabilityLevel::None);
        assert_eq!(detailed.table_detection, CapabilityLevel::None);
        assert_eq!(detailed.rendering, CapabilityLevel::Structural);
    }

    #[test]
    fn detailed_capabilities_equality() {
        let a = DetailedEngineCapabilities {
            text_extraction: CapabilityLevel::Structural,
            ..Default::default()
        };
        let b = DetailedEngineCapabilities {
            text_extraction: CapabilityLevel::Structural,
            ..Default::default()
        };
        assert_eq!(a, b);
    }

    #[test]
    fn detailed_capabilities_inequality() {
        let a = DetailedEngineCapabilities {
            text_extraction: CapabilityLevel::Structural,
            ..Default::default()
        };
        let b = DetailedEngineCapabilities {
            text_extraction: CapabilityLevel::Heuristic,
            ..Default::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn detailed_capabilities_debug() {
        let dc = DetailedEngineCapabilities::default();
        let dbg = format!("{:?}", dc);
        assert!(dbg.contains("DetailedEngineCapabilities"));
    }

    #[test]
    fn detailed_capabilities_clone() {
        let dc = DetailedEngineCapabilities {
            text_extraction: CapabilityLevel::Cloud,
            metadata: CapabilityLevel::Structural,
            image_extraction: CapabilityLevel::Heuristic,
            table_detection: CapabilityLevel::None,
            rendering: CapabilityLevel::Structural,
        };
        let cloned = dc;
        assert_eq!(dc, cloned);
    }

    #[test]
    fn engine_capabilities_default() {
        let caps = EngineCapabilities::default();
        assert!(!caps.create);
        assert!(!caps.read);
        assert!(!caps.manipulate);
        assert!(!caps.fill_forms);
        assert!(!caps.encrypt);
        assert!(!caps.sign);
        assert!(!caps.pdfa);
    }

    #[test]
    fn engine_capabilities_debug() {
        let caps = EngineCapabilities::lopdf();
        let dbg = format!("{:?}", caps);
        assert!(dbg.contains("EngineCapabilities"));
    }

    #[test]
    fn engine_capabilities_clone() {
        let caps = EngineCapabilities::printpdf();
        let cloned = caps;
        assert_eq!(caps.create, cloned.create);
    }

    #[test]
    fn pdf_field_descriptor_new() {
        let desc = PdfFieldDescriptor::new("pdf_name", "rust_name");
        assert_eq!(desc.field_name, "pdf_name");
        assert_eq!(desc.rust_field_name, "rust_name");
        assert_eq!(desc.order, u32::MAX);
        assert!(desc.format.is_none());
        assert!(desc.default_value.is_none());
        assert!(!desc.required);
        assert!(!desc.nested);
    }

    #[test]
    fn pdf_field_descriptor_debug() {
        let desc = PdfFieldDescriptor::new("f", "r");
        let dbg = format!("{:?}", desc);
        assert!(dbg.contains("PdfFieldDescriptor"));
    }

    #[test]
    fn pdf_field_descriptor_clone() {
        let desc = PdfFieldDescriptor::new("f", "r");
        let cloned = desc.clone();
        assert_eq!(desc.field_name, cloned.field_name);
    }

    #[test]
    fn pdf_model_metadata_default() {
        let meta = PdfModelMetadata::default();
        assert_eq!(meta.page_size, crate::enums::PageSize::A4);
        assert_eq!(meta.margins, 72.0);
    }

    #[test]
    fn pdf_model_metadata_debug() {
        let meta = PdfModelMetadata::default();
        let dbg = format!("{:?}", meta);
        assert!(dbg.contains("PdfModelMetadata"));
    }

    #[test]
    fn pdf_model_metadata_clone() {
        let meta = PdfModelMetadata::default();
        let cloned = meta.clone();
        assert_eq!(meta.margins, cloned.margins);
    }

    #[test]
    fn capability_level_clone_copy() {
        let level = CapabilityLevel::Structural;
        let copied = level;
        assert_eq!(level, copied);
    }

    #[test]
    fn capability_level_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CapabilityLevel::None);
        set.insert(CapabilityLevel::None);
        set.insert(CapabilityLevel::Heuristic);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn engine_capabilities_to_detailed_custom() {
        let caps = EngineCapabilities {
            create: true,
            read: true,
            manipulate: false,
            fill_forms: false,
            encrypt: false,
            sign: false,
            pdfa: false,
        };
        let detailed = caps.to_detailed();
        assert_eq!(detailed.text_extraction, CapabilityLevel::Structural);
        assert_eq!(detailed.metadata, CapabilityLevel::Structural);
        assert_eq!(detailed.image_extraction, CapabilityLevel::Heuristic);
        assert_eq!(detailed.table_detection, CapabilityLevel::Heuristic);
        assert_eq!(detailed.rendering, CapabilityLevel::Structural);
    }

    #[test]
    fn engine_capabilities_to_detailed_none() {
        let caps = EngineCapabilities::default();
        let detailed = caps.to_detailed();
        assert_eq!(detailed.text_extraction, CapabilityLevel::None);
        assert_eq!(detailed.metadata, CapabilityLevel::None);
        assert_eq!(detailed.image_extraction, CapabilityLevel::None);
        assert_eq!(detailed.table_detection, CapabilityLevel::None);
        assert_eq!(detailed.rendering, CapabilityLevel::None);
    }
}
