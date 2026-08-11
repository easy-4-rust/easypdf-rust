//! 开箱即用的内置语义处理器。
//!
//! 提供三个 Heuristic 级别的基础处理器：
//!
//! - `ReadingOrderProcessor`：阅读顺序检测
//! - `LinkExtractorProcessor`：超链接提取
//! - `HeadingDetectorProcessor`：标题检测
//!
//! 这些处理器可在 [`ProcessorPipeline`](crate::ProcessorPipeline) 中
//! 与自定义处理器组合使用。

mod heading_detector;
mod link_extractor;
mod reading_order;

pub use heading_detector::HeadingDetectorProcessor;
pub use link_extractor::LinkExtractorProcessor;
pub use reading_order::ReadingOrderProcessor;
