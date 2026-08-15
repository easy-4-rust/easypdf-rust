//! OCR 引擎抽象与结果类型。

use crate::render::RenderedImage;
use easypdf_core::CapabilityLevel;

/// OCR 识别的输入图像。
///
/// 保存原始 RGBA 像素数据及尺寸。可从 [`RenderedImage`]
/// 或 `image::DynamicImage` 构造。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::ocr::OcrImage;
/// use image::DynamicImage;
///
/// let img = DynamicImage::new_rgba8(100, 50);
/// let ocr_img = OcrImage::from_dynamic_image(&img);
/// assert_eq!(ocr_img.width, 100);
/// assert_eq!(ocr_img.height, 50);
/// ```
#[derive(Debug, Clone)]
pub struct OcrImage {
    /// 图像宽度（像素）。
    pub width: u32,
    /// 图像高度（像素）。
    pub height: u32,
    /// 原始 RGBA 像素数据（每像素 4 字节，行优先，从上到下）。
    pub pixels: Vec<u8>,
}

impl OcrImage {
    /// 从已渲染的 PDF 页面图像创建 `OcrImage`。
    ///
    /// # Panics
    ///
    /// 当像素缓冲区长度不等于 `width * height * 4` 时 panic。
    #[must_use]
    pub fn from_rendered(rendered: &RenderedImage) -> Self {
        Self {
            width: rendered.width,
            height: rendered.height,
            pixels: rendered.pixels.clone(),
        }
    }

    /// 从 `image::DynamicImage` 创建 `OcrImage`。
    ///
    /// 内部转换为 RGBA8 格式。
    #[must_use]
    pub fn from_dynamic_image(image: &image::DynamicImage) -> Self {
        let rgba = image.to_rgba8();
        Self {
            width: rgba.width(),
            height: rgba.height(),
            pixels: rgba.into_raw(),
        }
    }

    /// 从原始 RGBA 像素数据创建 `OcrImage`。
    #[must_use]
    pub const fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }
}

/// OCR 文本识别结果。
///
/// 包含提取的文本、可选的置信度分数以及可选的词级边界框（用于空间布局保留）。
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// 从图像中提取的文本。
    pub text: String,
    /// 总体置信度分数（0.0 到 1.0），由引擎提供时存在。
    pub confidence: Option<f32>,
    /// 逐词边界框，由引擎提供时存在。
    pub word_boxes: Vec<WordBox>,
}

/// 单个识别词的边界框。
///
/// 坐标为相对于输入图像左上角的像素值。
#[derive(Debug, Clone)]
pub struct WordBox {
    /// 识别出的词文本。
    pub text: String,
    /// 左上角 X 坐标（像素）。
    pub x: u32,
    /// 左上角 Y 坐标（像素）。
    pub y: u32,
    /// 边界框宽度（像素）。
    pub width: u32,
    /// 边界框高度（像素）。
    pub height: u32,
    /// 逐词置信度分数（0.0 到 1.0），可用时存在。
    pub confidence: Option<f32>,
}

/// OCR 引擎抽象。
///
/// 实现者使用不同后端（本地 ML 模型、云端 API、mock）从图像中提供文本识别。
/// 该 trait 是对象安全的，且要求 `Send + Sync` 以便跨线程使用。
///
/// # 实现自定义引擎
///
/// ```
/// use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};
/// use easypdf_core::CapabilityLevel;
///
/// struct MyEngine;
///
/// impl OcrEngine for MyEngine {
///     fn recognize(&self, image: &OcrImage) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
///         Ok(OcrResult {
///             text: format!("OCR of {}x{} image", image.width, image.height),
///             confidence: Some(0.95),
///             word_boxes: vec![],
///         })
///     }
///
///     fn name(&self) -> &'static str { "my-engine" }
///     fn languages(&self) -> &[&str] { &["en"] }
///     fn level(&self) -> CapabilityLevel { CapabilityLevel::Heuristic }
/// }
/// ```
pub trait OcrEngine: Send + Sync {
    /// 对给定图像执行 OCR 并返回识别文本。
    ///
    /// # Errors
    ///
    /// 当引擎处理图像失败时（模型加载失败、网络超时、格式不支持等）返回错误。
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>>;

    /// 此 OCR 引擎的可读名称（例如 `"ocrs"`、`"llm-gpt-4o"`）。
    fn name(&self) -> &'static str;

    /// 支持的语言代码（例如 `["en", "zh"]`）。
    fn languages(&self) -> &[&str];

    /// 此引擎的能力等级。
    ///
    /// - [`CapabilityLevel::Heuristic`]：本地 ML 模型（例如 ocrs）
    /// - [`CapabilityLevel::Cloud`]：云端 API（例如 LLM Vision）
    fn level(&self) -> CapabilityLevel;
}
