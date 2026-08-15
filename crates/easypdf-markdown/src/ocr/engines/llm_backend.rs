//! 使用 `rig-core` 的 LLM Vision OCR 后端。
//!
//! 需要 `llm` feature 标志。使用 `OpenAI` 兼容的 Vision API
//! 从图像中提取文本。
//!
//! LLM 客户端从环境变量读取 API 密钥：
//! - `OpenAI`：`OPENAI_API_KEY`
//! - `Gemini`：`GEMINI_API_KEY`

use base64::Engine;
use easypdf_core::CapabilityLevel;
use rig::OneOrMany;
use rig::completion::Prompt;
use rig::message::{ContentFormat, ImageMediaType, Message, UserContent};

use crate::ocr::engine::{OcrEngine, OcrImage, OcrResult};

/// 发送给 LLM Vision 模型的默认 OCR 提示词。
const DEFAULT_OCR_PROMPT: &str = "Extract all text from this image. \
Return ONLY the extracted text, maintaining the original layout and order. \
Do not add any commentary or description.";

/// 基于 `rig-core` 的 LLM Vision OCR 引擎。
///
/// 支持任何 `OpenAI` 兼容的 Vision API（`OpenAI`、`Gemini`、`DeepSeek` 等）。
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::ocr::{OcrEngine, engines::LlmOcrEngine};
///
/// // 需要 OPENAI_API_KEY 环境变量
/// let engine = LlmOcrEngine::openai("gpt-4o");
/// println!("engine: {}", engine.name());
/// ```
pub struct LlmOcrEngine {
    provider: LlmProvider,
    model: String,
    prompt: String,
}

/// 支持的 LLM 提供商。
#[derive(Debug, Clone)]
enum LlmProvider {
    /// `OpenAI`（或兼容）API。
    OpenAI,
    /// Google Gemini API。
    Gemini,
    /// `DeepSeek` API。
    DeepSeek,
}

impl std::fmt::Debug for LlmOcrEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmOcrEngine")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl LlmOcrEngine {
    /// 创建 `OpenAI` Vision OCR 引擎。
    ///
    /// 需要 `OPENAI_API_KEY` 环境变量。
    #[must_use]
    pub fn openai(model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            model: model.into(),
            prompt: DEFAULT_OCR_PROMPT.to_owned(),
        }
    }

    /// 创建 Gemini Vision OCR 引擎。
    ///
    /// 需要 `GEMINI_API_KEY` 环境变量。
    #[must_use]
    pub fn gemini(model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::Gemini,
            model: model.into(),
            prompt: DEFAULT_OCR_PROMPT.to_owned(),
        }
    }

    /// 创建 `DeepSeek` Vision OCR 引擎。
    ///
    /// 需要 `DEEPSEEK_API_KEY` 环境变量。
    #[must_use]
    pub fn deepseek(model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::DeepSeek,
            model: model.into(),
            prompt: DEFAULT_OCR_PROMPT.to_owned(),
        }
    }

    /// 设置自定义 OCR 提示词。
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// 同步运行 LLM Vision 请求。
    fn run_llm(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 将 RGBA 转换为 PNG 字节供 LLM 使用。
        let img = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
            .ok_or("invalid pixel buffer dimensions")?;
        let dynamic = image::DynamicImage::ImageRgba8(img);
        let mut png_buf = Vec::new();
        dynamic.write_to(
            &mut std::io::Cursor::new(&mut png_buf),
            image::ImageFormat::Png,
        )?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_buf);

        let mut content_items = OneOrMany::one(UserContent::image(
            b64,
            Some(ContentFormat::Base64),
            Some(ImageMediaType::PNG),
            None,
        ));
        content_items.push(UserContent::text(&self.prompt));

        let message = Message::User {
            content: content_items,
        };

        let rt = tokio::runtime::Handle::current();
        let response = rt.block_on(async {
            match self.provider {
                LlmProvider::OpenAI => {
                    let client = rig::providers::openai::Client::from_env();
                    let agent = client
                        .agent(&self.model)
                        .preamble("You are an OCR text extractor.")
                        .build();
                    agent.prompt(message).await
                }
                LlmProvider::Gemini => {
                    let client = rig::providers::gemini::Client::from_env();
                    let agent = client
                        .agent(&self.model)
                        .preamble("You are an OCR text extractor.")
                        .build();
                    agent.prompt(message).await
                }
                LlmProvider::DeepSeek => {
                    let client = rig::providers::deepseek::Client::from_env();
                    let agent = client
                        .agent(&self.model)
                        .preamble("You are an OCR text extractor.")
                        .build();
                    agent.prompt(message).await
                }
            }
        })?;

        Ok(response)
    }
}

impl OcrEngine for LlmOcrEngine {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        let text = self.run_llm(image)?;
        Ok(OcrResult {
            text: text.trim().to_owned(),
            confidence: None,
            word_boxes: vec![],
        })
    }

    fn name(&self) -> &'static str {
        match self.provider {
            LlmProvider::OpenAI => "llm-openai",
            LlmProvider::Gemini => "llm-gemini",
            LlmProvider::DeepSeek => "llm-deepseek",
        }
    }

    fn languages(&self) -> &[&str] {
        &["en"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Cloud
    }
}

// ================================================================
// DeepSeek-OCR-2 引擎
// ================================================================

/// DeepSeek-OCR-2 的默认 OCR 提示词。
const DEEPSEEK_OCR_PROMPT: &str = "Extract all text from this image. \
Return ONLY the extracted text, maintaining the original layout and order. \
Do not add any commentary or description.";

/// 使用 `DeepSeek` Vision API 的 DeepSeek-OCR-2 OCR 引擎。
///
/// 该引擎针对 [DeepSeek-OCR-2][model] 进行了优化，这是一个 30 亿参数的
/// 视觉语言模型，专门设计用于 OCR 任务（包括文档转 Markdown）。
///
/// # 配置
///
/// | 环境变量 | 说明 | 默认值 |
/// |---------|------|--------|
/// | `DEEPSEEK_API_KEY` | `DeepSeek` API 密钥（api.deepseek.com 必需） | — |
/// | `DEEPSEEK_BASE_URL` | 自定义端点 URL | `https://api.deepseek.com` |
///
/// 设置 `DEEPSEEK_BASE_URL` 时，请求将路由到该 URL 而非官方 `DeepSeek` API。
/// 支持：
/// - **自托管** vLLM/TGI 服务器
/// - **`HuggingFace` Inference Endpoints**（设置 URL 为你的端点）
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::ocr::{OcrEngine, engines::DeepSeekOcrEngine};
///
/// // 使用环境变量中的 DEEPSEEK_API_KEY 和可选的 DEEPSEEK_BASE_URL
/// let engine = DeepSeekOcrEngine::from_env();
/// println!("engine: {}", engine.name());
/// ```
///
/// ```no_run
/// use easypdf_markdown::ocr::engines::DeepSeekOcrEngine;
///
/// // 显式配置
/// let engine = DeepSeekOcrEngine::new("my-api-key")
///     .with_base_url("https://my-vllm.example.com/v1")
///     .with_model("deepseek-ocr-2")
///     .with_prompt("Extract text from this document image.");
/// ```
///
/// [model]: https://huggingface.co/deepseek-ai/DeepSeek-OCR-2
pub struct DeepSeekOcrEngine {
    api_key: String,
    base_url: Option<String>,
    model: String,
    prompt: String,
}

impl std::fmt::Debug for DeepSeekOcrEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepSeekOcrEngine")
            .field("api_key", &"***")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl DeepSeekOcrEngine {
    /// 使用给定 API 密钥创建新的 DeepSeek-OCR-2 引擎。
    ///
    /// 使用官方 `DeepSeek` API（`https://api.deepseek.com`）。
    /// 使用 [`with_base_url`](Self::with_base_url) 指定其他端点。
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            model: "deepseek-ocr-2".to_owned(),
            prompt: DEEPSEEK_OCR_PROMPT.to_owned(),
        }
    }

    /// 从环境变量创建。
    ///
    /// 读取 `DEEPSEEK_API_KEY`（必需）和 `DEEPSEEK_BASE_URL`（可选）。
    ///
    /// # Panics
    ///
    /// 当 `DEEPSEEK_API_KEY` 未设置时 panic。
    #[must_use]
    pub fn from_env() -> Self {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .expect("DEEPSEEK_API_KEY environment variable not set");
        let base_url = std::env::var("DEEPSEEK_BASE_URL").ok();
        Self {
            api_key,
            base_url,
            model: "deepseek-ocr-2".to_owned(),
            prompt: DEEPSEEK_OCR_PROMPT.to_owned(),
        }
    }

    /// 设置自定义 API 端点基础 URL。
    ///
    /// 用于指向自托管 vLLM/TGI 服务器或 `HuggingFace`
    /// Inference Endpoints。
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// 覆盖模型名称。
    ///
    /// 默认值为 `"deepseek-ocr-2"`。
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 设置自定义 OCR 提示词。
    ///
    /// 覆盖指示模型提取文本的默认提示词。
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// 同步运行 Vision OCR 请求。
    fn run_ocr(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // 将 RGBA 转换为 PNG 字节。
        let img = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
            .ok_or("invalid pixel buffer dimensions")?;
        let dynamic = image::DynamicImage::ImageRgba8(img);
        let mut png_buf = Vec::new();
        dynamic.write_to(
            &mut std::io::Cursor::new(&mut png_buf),
            image::ImageFormat::Png,
        )?;

        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_buf);

        let mut content_items = OneOrMany::one(UserContent::image(
            b64,
            Some(ContentFormat::Base64),
            Some(ImageMediaType::PNG),
            None,
        ));
        content_items.push(UserContent::text(&self.prompt));

        let message = Message::User {
            content: content_items,
        };

        let client = match &self.base_url {
            Some(url) => rig::providers::deepseek::Client::from_url(&self.api_key, url),
            None => rig::providers::deepseek::Client::new(&self.api_key),
        };

        let model = self.model.clone();
        let rt = tokio::runtime::Handle::current();
        let response = rt.block_on(async {
            let agent = client
                .agent(&model)
                .preamble("You are an OCR text extractor.")
                .build();
            agent.prompt(message).await
        })?;

        Ok(response)
    }
}

impl OcrEngine for DeepSeekOcrEngine {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        let text = self.run_ocr(image)?;
        Ok(OcrResult {
            text: text.trim().to_owned(),
            confidence: None,
            word_boxes: vec![],
        })
    }

    fn name(&self) -> &'static str {
        "deepseek-ocr-2"
    }

    fn languages(&self) -> &[&str] {
        &["zh", "en", "auto"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Cloud
    }
}
