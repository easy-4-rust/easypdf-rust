//! 使用 `ocrs` crate 的纯 Rust OCR 后端。
//!
//! 需要 `ocrs` feature 标志。使用 `ocrs` crate 内置的
//! 检测和识别 ONNX 模型进行文本提取。

use easypdf_core::CapabilityLevel;

use crate::ocr::engine::{OcrEngine, OcrImage, OcrResult, WordBox};

/// 基于 `ocrs` crate 的纯 Rust OCR 引擎。
///
/// 使用基于 ONNX 的文本检测和识别模型，无需外部系统依赖。
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::ocr::{OcrEngine, engines::OcrsEngine};
///
/// let engine = OcrsEngine::new().expect("failed to initialize ocrs");
/// println!("engine: {}", engine.name());
/// ```
pub struct OcrsEngine {
    engine: ocrs::OcrEngine,
}

impl std::fmt::Debug for OcrsEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrsEngine")
            .field("engine", &"<ocrs::OcrEngine>")
            .finish()
    }
}

impl OcrsEngine {
    /// 使用默认模型参数创建新的 ocrs 引擎。
    ///
    /// # Errors
    ///
    /// 当 ONNX 模型无法加载或初始化时返回错误。
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let params = ocrs::OcrEngineParams::default();
        let engine = ocrs::OcrEngine::new(params)?;
        Ok(Self { engine })
    }

    /// 使用自定义参数创建新的 ocrs 引擎。
    ///
    /// # Errors
    ///
    /// 当 ONNX 模型无法加载或初始化时返回错误。
    pub fn with_params(
        params: ocrs::OcrEngineParams,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let engine = ocrs::OcrEngine::new(params)?;
        Ok(Self { engine })
    }
}

impl OcrEngine for OcrsEngine {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        // 将 RGBA 像素转换为 RGB（HWC 排列）供 ocrs 使用。
        let rgb_pixels: Vec<u8> = image
            .pixels
            .chunks_exact(4)
            .flat_map(|rgba| &rgba[..3])
            .copied()
            .collect();

        let img_source = ocrs::ImageSource::from_bytes(&rgb_pixels, (image.width, image.height))?;
        let ocr_input = self.engine.prepare_input(img_source)?;

        // 使用便捷 API：检测 + 识别 + 收集为字符串。
        let text = self.engine.get_text(&ocr_input)?;

        // 获取词级详情，检测词并构建边界框。
        let word_boxes = match self.engine.detect_words(&ocr_input) {
            Ok(rects) => rects
                .iter()
                .map(|r| {
                    // ocrs 的坐标是 f32 像素值，转 u32 时截断/去符号是可接受的精度损失。
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let (cx, cy, rw, rh) = (
                        r.center().x as u32,
                        r.center().y as u32,
                        r.width() as u32,
                        r.height() as u32,
                    );
                    WordBox {
                        text: String::new(), // ocrs 不提供逐词文本
                        x: cx,
                        y: cy,
                        width: rw,
                        height: rh,
                        confidence: None,
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        Ok(OcrResult {
            text,
            confidence: None,
            word_boxes,
        })
    }

    fn name(&self) -> &'static str {
        "ocrs"
    }

    fn languages(&self) -> &[&str] {
        &["en"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Heuristic
    }
}
