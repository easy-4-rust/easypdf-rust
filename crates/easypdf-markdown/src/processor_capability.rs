//! 语义处理器能力查询枚举。

use easypdf_core::CapabilityLevel;

use crate::MarkdownProcessorCapabilities;

/// 可查询的处理器能力维度。
///
/// 与 [`DetailedProcessorCapabilities`] 的字段一一对应，
/// 用于 [`ProcessorPipeline::aggregate_capabilities`](crate::ProcessorPipeline::aggregate_capabilities)
/// 和降级策略中的能力过滤。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProcessorCapability {
    /// 表格检测与提取。
    TableDetection,
    /// 图片提取。
    ImageExtraction,
    /// 光学字符识别（OCR）。
    Ocr,
    /// 阅读顺序检测。
    ReadingOrder,
    /// 数学公式识别。
    Formula,
    /// 超链接提取。
    Link,
}

/// 基于 [`CapabilityLevel`] 的细粒度处理器能力声明。
///
/// 相比布尔值的 [`MarkdownProcessorCapabilities`]，本类型能够表达
/// "处理器以何种质量提供某项能力"，从而支持管道中的降级策略。
///
/// # Examples
///
/// ```
/// use easypdf_core::CapabilityLevel;
/// use easypdf_markdown::{DetailedProcessorCapabilities, ProcessorCapability};
///
/// let mut caps = DetailedProcessorCapabilities::default();
/// caps.table_detection = CapabilityLevel::Structural;
/// caps.ocr = CapabilityLevel::Cloud;
/// assert!(caps.supports(ProcessorCapability::TableDetection));
/// assert!(!caps.supports(ProcessorCapability::Link));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DetailedProcessorCapabilities {
    /// 表格检测能力等级。
    pub table_detection: CapabilityLevel,
    /// 图片提取能力等级。
    pub image_extraction: CapabilityLevel,
    /// OCR 能力等级。
    pub ocr: CapabilityLevel,
    /// 阅读顺序检测能力等级。
    pub reading_order_detection: CapabilityLevel,
    /// 数学公式识别能力等级。
    pub formula_recognition: CapabilityLevel,
    /// 超链接提取能力等级。
    pub link_extraction: CapabilityLevel,
}

impl DetailedProcessorCapabilities {
    /// 创建全 None 的空能力集合。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            table_detection: CapabilityLevel::None,
            image_extraction: CapabilityLevel::None,
            ocr: CapabilityLevel::None,
            reading_order_detection: CapabilityLevel::None,
            formula_recognition: CapabilityLevel::None,
            link_extraction: CapabilityLevel::None,
        }
    }

    /// 合并两组能力，每项取较高 [`CapabilityLevel`]。
    ///
    /// 用于聚合管道中所有处理器的综合能力。
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            table_detection: max_level(self.table_detection, other.table_detection),
            image_extraction: max_level(self.image_extraction, other.image_extraction),
            ocr: max_level(self.ocr, other.ocr),
            reading_order_detection: max_level(
                self.reading_order_detection,
                other.reading_order_detection,
            ),
            formula_recognition: max_level(
                self.formula_recognition,
                other.formula_recognition,
            ),
            link_extraction: max_level(self.link_extraction, other.link_extraction),
        }
    }

    /// 查询是否支持指定能力维度（level 非 `None`）。
    #[must_use]
    pub const fn supports(&self, capability: ProcessorCapability) -> bool {
        match capability {
            ProcessorCapability::TableDetection => self.table_detection.is_supported(),
            ProcessorCapability::ImageExtraction => self.image_extraction.is_supported(),
            ProcessorCapability::Ocr => self.ocr.is_supported(),
            ProcessorCapability::ReadingOrder => self.reading_order_detection.is_supported(),
            ProcessorCapability::Formula => self.formula_recognition.is_supported(),
            ProcessorCapability::Link => self.link_extraction.is_supported(),
        }
    }

    /// 返回指定能力维度的等级。
    #[must_use]
    pub const fn level_of(&self, capability: ProcessorCapability) -> CapabilityLevel {
        match capability {
            ProcessorCapability::TableDetection => self.table_detection,
            ProcessorCapability::ImageExtraction => self.image_extraction,
            ProcessorCapability::Ocr => self.ocr,
            ProcessorCapability::ReadingOrder => self.reading_order_detection,
            ProcessorCapability::Formula => self.formula_recognition,
            ProcessorCapability::Link => self.link_extraction,
        }
    }

    /// 判断本组能力是否满足目标要求（每项 >= 目标）。
    ///
    /// 用于降级策略：跳过无法满足目标等级的处理器。
    #[must_use]
    pub fn meets_target(&self, target: &Self) -> bool {
        self.table_detection >= target.table_detection
            && self.image_extraction >= target.image_extraction
            && self.ocr >= target.ocr
            && self.reading_order_detection >= target.reading_order_detection
            && self.formula_recognition >= target.formula_recognition
            && self.link_extraction >= target.link_extraction
    }
}

/// 从布尔值的 [`MarkdownProcessorCapabilities`] 转换。
///
/// `true` 映射为 [`CapabilityLevel::Heuristic`]，`false` 映射为 [`CapabilityLevel::None`]。
impl From<MarkdownProcessorCapabilities> for DetailedProcessorCapabilities {
    fn from(caps: MarkdownProcessorCapabilities) -> Self {
        Self {
            table_detection: bool_to_level(caps.table_detection()),
            image_extraction: bool_to_level(caps.image_extraction()),
            ocr: bool_to_level(caps.ocr()),
            reading_order_detection: CapabilityLevel::None,
            formula_recognition: CapabilityLevel::None,
            link_extraction: CapabilityLevel::None,
        }
    }
}

const fn bool_to_level(b: bool) -> CapabilityLevel {
    if b {
        CapabilityLevel::Heuristic
    } else {
        CapabilityLevel::None
    }
}

const fn max_level(a: CapabilityLevel, b: CapabilityLevel) -> CapabilityLevel {
    if a as u8 >= b as u8 {
        a
    } else {
        b
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn detailed_default_is_all_none() {
        let caps = DetailedProcessorCapabilities::default();
        assert_eq!(caps.table_detection, CapabilityLevel::None);
        assert_eq!(caps.image_extraction, CapabilityLevel::None);
        assert_eq!(caps.ocr, CapabilityLevel::None);
        assert_eq!(caps.reading_order_detection, CapabilityLevel::None);
        assert_eq!(caps.formula_recognition, CapabilityLevel::None);
        assert_eq!(caps.link_extraction, CapabilityLevel::None);
    }

    #[test]
    fn merge_takes_higher_level() {
        let a = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Heuristic,
            ocr: CapabilityLevel::Cloud,
            ..Default::default()
        };
        let b = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ocr: CapabilityLevel::Heuristic,
            link_extraction: CapabilityLevel::Structural,
            ..Default::default()
        };
        let merged = a.merge(&b);
        assert_eq!(merged.table_detection, CapabilityLevel::Structural);
        assert_eq!(merged.ocr, CapabilityLevel::Cloud);
        assert_eq!(merged.link_extraction, CapabilityLevel::Structural);
    }

    #[test]
    fn supports_queries_correctly() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ..Default::default()
        };
        assert!(caps.supports(ProcessorCapability::TableDetection));
        assert!(!caps.supports(ProcessorCapability::Ocr));
    }

    #[test]
    fn meets_target_allows_higher() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ocr: CapabilityLevel::Cloud,
            ..Default::default()
        };
        let target = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Heuristic,
            ocr: CapabilityLevel::Heuristic,
            ..Default::default()
        };
        assert!(caps.meets_target(&target));
    }

    #[test]
    fn meets_target_rejects_lower() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Heuristic,
            ..Default::default()
        };
        let target = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ..Default::default()
        };
        assert!(!caps.meets_target(&target));
    }

    #[test]
    fn from_bool_capabilities() {
        let bool_caps = MarkdownProcessorCapabilities::new()
            .with_table_detection()
            .with_ocr();
        let detailed = DetailedProcessorCapabilities::from(bool_caps);
        assert_eq!(detailed.table_detection, CapabilityLevel::Heuristic);
        assert_eq!(detailed.ocr, CapabilityLevel::Heuristic);
        assert_eq!(detailed.image_extraction, CapabilityLevel::None);
        assert_eq!(detailed.reading_order_detection, CapabilityLevel::None);
    }

    #[test]
    fn level_of_returns_correct_field() {
        let caps = DetailedProcessorCapabilities {
            formula_recognition: CapabilityLevel::Cloud,
            ..Default::default()
        };
        assert_eq!(
            caps.level_of(ProcessorCapability::Formula),
            CapabilityLevel::Cloud
        );
    }

    #[test]
    fn level_of_all_capabilities() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Heuristic,
            image_extraction: CapabilityLevel::Structural,
            ocr: CapabilityLevel::Cloud,
            reading_order_detection: CapabilityLevel::Heuristic,
            formula_recognition: CapabilityLevel::Structural,
            link_extraction: CapabilityLevel::Cloud,
        };
        assert_eq!(caps.level_of(ProcessorCapability::TableDetection), CapabilityLevel::Heuristic);
        assert_eq!(caps.level_of(ProcessorCapability::ImageExtraction), CapabilityLevel::Structural);
        assert_eq!(caps.level_of(ProcessorCapability::Ocr), CapabilityLevel::Cloud);
        assert_eq!(caps.level_of(ProcessorCapability::ReadingOrder), CapabilityLevel::Heuristic);
        assert_eq!(caps.level_of(ProcessorCapability::Formula), CapabilityLevel::Structural);
        assert_eq!(caps.level_of(ProcessorCapability::Link), CapabilityLevel::Cloud);
    }

    #[test]
    fn supports_all_capabilities() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Heuristic,
            image_extraction: CapabilityLevel::Structural,
            ocr: CapabilityLevel::Cloud,
            reading_order_detection: CapabilityLevel::Heuristic,
            formula_recognition: CapabilityLevel::Structural,
            link_extraction: CapabilityLevel::Cloud,
        };
        assert!(caps.supports(ProcessorCapability::TableDetection));
        assert!(caps.supports(ProcessorCapability::ImageExtraction));
        assert!(caps.supports(ProcessorCapability::Ocr));
        assert!(caps.supports(ProcessorCapability::ReadingOrder));
        assert!(caps.supports(ProcessorCapability::Formula));
        assert!(caps.supports(ProcessorCapability::Link));
    }

    #[test]
    fn supports_none_returns_false() {
        let caps = DetailedProcessorCapabilities::default();
        assert!(!caps.supports(ProcessorCapability::TableDetection));
        assert!(!caps.supports(ProcessorCapability::Ocr));
    }

    #[test]
    fn merge_with_default() {
        let a = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ..Default::default()
        };
        let b = DetailedProcessorCapabilities::default();
        let merged = a.merge(&b);
        assert_eq!(merged.table_detection, CapabilityLevel::Structural);
    }

    #[test]
    fn merge_default_with_non_default() {
        let a = DetailedProcessorCapabilities::default();
        let b = DetailedProcessorCapabilities {
            ocr: CapabilityLevel::Cloud,
            ..Default::default()
        };
        let merged = a.merge(&b);
        assert_eq!(merged.ocr, CapabilityLevel::Cloud);
    }

    #[test]
    fn meets_target_exact_match() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ..Default::default()
        };
        let target = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Structural,
            ..Default::default()
        };
        assert!(caps.meets_target(&target));
    }

    #[test]
    fn meets_target_with_none_target() {
        let caps = DetailedProcessorCapabilities::default();
        let target = DetailedProcessorCapabilities::default();
        // None meets None target
        assert!(caps.meets_target(&target));
    }

    #[test]
    fn debug_format() {
        let caps = DetailedProcessorCapabilities::default();
        let dbg = format!("{:?}", caps);
        assert!(dbg.contains("DetailedProcessorCapabilities"));
    }

    #[test]
    fn clone_preserves() {
        let caps = DetailedProcessorCapabilities {
            table_detection: CapabilityLevel::Cloud,
            ..Default::default()
        };
        let cloned = caps;
        assert_eq!(caps, cloned);
    }

    #[test]
    fn processor_capability_debug() {
        assert_eq!(format!("{:?}", ProcessorCapability::TableDetection), "TableDetection");
        assert_eq!(format!("{:?}", ProcessorCapability::Ocr), "Ocr");
    }
}
