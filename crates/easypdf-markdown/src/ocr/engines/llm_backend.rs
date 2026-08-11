//! LLM Vision OCR backend using `rig-core`.
//!
//! Requires the `llm` feature flag. Uses OpenAI-compatible vision APIs
//! for text extraction from images.
//!
//! The LLM client reads its API key from the environment:
//! - `OpenAI`: `OPENAI_API_KEY`
//! - `Gemini`: `GEMINI_API_KEY`

use base64::Engine;
use easypdf_core::CapabilityLevel;
use rig::completion::Prompt;
use rig::message::{ContentFormat, ImageMediaType, Message, UserContent};
use rig::OneOrMany;

use crate::ocr::engine::{OcrEngine, OcrImage, OcrResult};

/// Default OCR prompt sent to the LLM vision model.
const DEFAULT_OCR_PROMPT: &str = "Extract all text from this image. \
Return ONLY the extracted text, maintaining the original layout and order. \
Do not add any commentary or description.";

/// LLM Vision OCR engine backed by `rig-core`.
///
/// Supports any OpenAI-compatible vision API (`OpenAI`, Gemini, `DeepSeek`, etc.).
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::ocr::{OcrEngine, engines::LlmOcrEngine};
///
/// // Requires OPENAI_API_KEY environment variable
/// let engine = LlmOcrEngine::openai("gpt-4o");
/// println!("engine: {}", engine.name());
/// ```
pub struct LlmOcrEngine {
    provider: LlmProvider,
    model: String,
    prompt: String,
}

/// Supported LLM providers.
#[derive(Debug, Clone)]
enum LlmProvider {
    /// `OpenAI` (or compatible) API.
    OpenAI,
    /// Google Gemini API.
    Gemini,
    /// `DeepSeek` API.
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
    /// Create an `OpenAI` vision OCR engine.
    ///
    /// Requires `OPENAI_API_KEY` environment variable.
    #[must_use]
    pub fn openai(model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            model: model.into(),
            prompt: DEFAULT_OCR_PROMPT.to_owned(),
        }
    }

    /// Create a Gemini vision OCR engine.
    ///
    /// Requires `GEMINI_API_KEY` environment variable.
    #[must_use]
    pub fn gemini(model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::Gemini,
            model: model.into(),
            prompt: DEFAULT_OCR_PROMPT.to_owned(),
        }
    }

    /// Create a `DeepSeek` vision OCR engine.
    ///
    /// Requires `DEEPSEEK_API_KEY` environment variable.
    #[must_use]
    pub fn deepseek(model: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::DeepSeek,
            model: model.into(),
            prompt: DEFAULT_OCR_PROMPT.to_owned(),
        }
    }

    /// Set a custom OCR prompt.
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Run the LLM vision request synchronously.
    fn run_llm(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Convert RGBA to PNG bytes for the LLM.
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
// DeepSeek-OCR-2 Engine
// ================================================================

/// Default OCR prompt for DeepSeek-OCR-2.
const DEEPSEEK_OCR_PROMPT: &str = "Extract all text from this image. \
Return ONLY the extracted text, maintaining the original layout and order. \
Do not add any commentary or description.";

/// DeepSeek-OCR-2 OCR engine using the `DeepSeek` Vision API.
///
/// This engine is optimized for [DeepSeek-OCR-2][model], a 3B-parameter
/// vision-language model specifically designed for OCR tasks including
/// document-to-markdown conversion.
///
/// # Configuration
///
/// | Environment Variable | Description | Default |
/// |---------------------|-------------|---------|
/// | `DEEPSEEK_API_KEY` | `DeepSeek` API key (required for api.deepseek.com) | — |
/// | `DEEPSEEK_BASE_URL` | Custom endpoint URL | `https://api.deepseek.com` |
///
/// When `DEEPSEEK_BASE_URL` is set, requests are routed to that URL instead
/// of the official `DeepSeek` API. This supports:
/// - **Self-hosted** vLLM/TGI servers
/// - **`HuggingFace` Inference Endpoints** (set URL to your endpoint)
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::ocr::{OcrEngine, engines::DeepSeekOcrEngine};
///
/// // Uses DEEPSEEK_API_KEY and optional DEEPSEEK_BASE_URL from environment
/// let engine = DeepSeekOcrEngine::from_env();
/// println!("engine: {}", engine.name());
/// ```
///
/// ```no_run
/// use easypdf_markdown::ocr::engines::DeepSeekOcrEngine;
///
/// // Explicit configuration
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
    /// Create a new DeepSeek-OCR-2 engine with the given API key.
    ///
    /// Uses the official `DeepSeek` API (`https://api.deepseek.com`).
    /// Use [`with_base_url`](Self::with_base_url) to target a different endpoint.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            model: "deepseek-ocr-2".to_owned(),
            prompt: DEEPSEEK_OCR_PROMPT.to_owned(),
        }
    }

    /// Create from environment variables.
    ///
    /// Reads `DEEPSEEK_API_KEY` (required) and `DEEPSEEK_BASE_URL` (optional).
    ///
    /// # Panics
    ///
    /// Panics if `DEEPSEEK_API_KEY` is not set.
    #[must_use]
    pub fn from_env() -> Self {
        let api_key =
            std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY environment variable not set");
        let base_url = std::env::var("DEEPSEEK_BASE_URL").ok();
        Self {
            api_key,
            base_url,
            model: "deepseek-ocr-2".to_owned(),
            prompt: DEEPSEEK_OCR_PROMPT.to_owned(),
        }
    }

    /// Set a custom base URL for the API endpoint.
    ///
    /// Use this to target self-hosted vLLM/TGI servers or `HuggingFace`
    /// Inference Endpoints.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Override the model name.
    ///
    /// Defaults to `"deepseek-ocr-2"`.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set a custom OCR prompt.
    ///
    /// Overrides the default prompt that instructs the model to extract text.
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Run the vision OCR request synchronously.
    fn run_ocr(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Convert RGBA to PNG bytes.
        let img = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
            .ok_or("invalid pixel buffer dimensions")?;
        let dynamic = image::DynamicImage::ImageRgba8(img);
        let mut png_buf = Vec::new();
        dynamic.write_to(
            &mut std::io::Cursor::new(&mut png_buf),
            image::ImageFormat::Png,
        )?;

        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &png_buf,
        );

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
