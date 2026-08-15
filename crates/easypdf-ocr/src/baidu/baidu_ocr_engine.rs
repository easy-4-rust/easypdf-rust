//! 百度云 OCR 引擎实现。

use base64::Engine;
use easypdf_core::CapabilityLevel;
use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};

use super::config::{BaiduApi, BaiduConfig, BaiduError, BaiduResult};
use super::parser::BaiduOcrParser;
use super::token::TokenManager;

/// 百度云 OCR 引擎。
///
/// 通过 OAuth 令牌交换（带缓存）和表单编码请求实现 [`OcrEngine`]。
/// 支持通过 [`BaiduApi`] 选择多种 API 端点。
///
/// # 线程安全
///
/// `BaiduOcrEngine` 是 `Send + Sync` 的，可跨线程共享。
/// OAuth 令牌缓存使用 `parking_lot::Mutex` 实现无竞争访问。
pub struct BaiduOcrEngine {
    /// 引擎配置。
    config: BaiduConfig,
    /// OAuth 令牌管理器（用于标准 API）。
    token_manager: TokenManager,
    /// 响应解析器。
    parser: BaiduOcrParser,
    /// HTTP 客户端（复用连接池）。
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for BaiduOcrEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaiduOcrEngine")
            .field("api", &self.config.api)
            .field("endpoint", &self.config.endpoint)
            .field("api_key", &self.config.api_key)
            .field("secret_key", &"***")
            .field("token_manager", &self.token_manager)
            .field("parser", &self.parser)
            .finish_non_exhaustive()
    }
}

impl BaiduOcrEngine {
    /// 使用给定配置创建百度 OCR 引擎。
    ///
    /// # 参数
    ///
    /// * `config` - 百度云 OCR 配置，包含 API 密钥、密钥和端点等信息。
    ///
    /// # Panics
    ///
    /// 若 `reqwest` HTTP 客户端构建失败则 panic（正常情况下不会发生）。
    #[must_use]
    pub fn new(config: BaiduConfig) -> Self {
        let token_manager = TokenManager::new(
            config.token_url.clone(),
            config.api_key.clone(),
            config.secret_key.clone(),
        );
        let parser = BaiduOcrParser::new(config.api);
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build HTTP client");

        Self {
            config,
            token_manager,
            parser,
            client,
        }
    }

    /// 构建标准百度 OCR API 的完整请求 URL。
    ///
    /// 格式：`{endpoint}/{path}?access_token={token}`
    pub(crate) fn build_url(&self, token: &str) -> String {
        format!(
            "{}/{}?access_token={}",
            self.config.endpoint,
            self.config.api.path(),
            token
        )
    }

    /// 构建千帆 OCR 的请求 URL。
    fn build_qianfan_url(&self) -> &str {
        &self.config.qianfan_endpoint
    }

    /// 将图像编码为 base64 并进行 URL 编码以用于表单提交。
    pub(crate) fn encode_image_form(image: &OcrImage) -> BaiduResult<String> {
        let png_bytes = encode_to_png(image)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        Ok(urlencoding::encode(&b64).into_owned())
    }

    /// 执行标准百度 OCR 请求（表单编码，带 `access_token`）。
    fn execute_standard(&self, image: &OcrImage) -> BaiduResult<OcrResult> {
        if !self.config.api.is_supported() {
            return Err(BaiduError::UnsupportedApi(self.config.api));
        }

        let token = self.token_manager.get_token()?;
        let url = self.build_url(&token);
        let image_data = Self::encode_image_form(image)?;

        let mut params = vec![("image", image_data)];

        // 对带位置的变体添加 recognizeGranularity=char。
        if self.config.api.requests_boxes() {
            params.push(("recognizeGranularity", "char".to_owned()));
        }

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .map_err(BaiduError::Transport)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().unwrap_or_default();
            return Err(BaiduError::InvalidResponse(format!(
                "HTTP {status}: {body}"
            )));
        }

        let raw: serde_json::Value = resp.json().map_err(BaiduError::Transport)?;
        self.parser.parse(&raw)
    }

    /// 执行千帆 OCR 请求（JSON，带 Bearer 令牌）。
    fn execute_qianfan(&self, image: &OcrImage) -> BaiduResult<OcrResult> {
        let png_bytes = encode_to_png(image)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        // 千帆 OCR 直接使用 api_key 作为 Bearer 令牌。
        let url = self.build_qianfan_url();

        let body = serde_json::json!({
            "image": b64,
            "model": "Qianfan-OCR"
        });

        let resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&body)
            .send()
            .map_err(BaiduError::Transport)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().unwrap_or_default();
            return Err(BaiduError::InvalidResponse(format!(
                "HTTP {status}: {body}"
            )));
        }

        let raw: serde_json::Value = resp.json().map_err(BaiduError::Transport)?;
        self.parser.parse(&raw)
    }

    /// 获取引擎配置的引用。
    #[must_use]
    pub fn config(&self) -> &BaiduConfig {
        &self.config
    }
}

impl OcrEngine for BaiduOcrEngine {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        if self.config.api == BaiduApi::QianfanOcr {
            self.execute_qianfan(image).map_err(Into::into)
        } else {
            self.execute_standard(image).map_err(Into::into)
        }
    }

    fn name(&self) -> &'static str {
        self.config.api.engine_name()
    }

    fn languages(&self) -> &[&str] {
        &["zh", "en", "ja", "ko"]
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Cloud
    }
}

/// 将 RGBA 像素数据编码为 PNG 字节。
///
/// # Errors
///
/// 若像素数据无法编码，返回 [`BaiduError::ImageEncoding`]。
pub(crate) fn encode_to_png(image: &OcrImage) -> BaiduResult<Vec<u8>> {
    let rgba_img = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
        .ok_or_else(|| {
            BaiduError::ImageEncoding(format!(
                "pixel buffer length {} does not match {}x{}x4",
                image.pixels.len(),
                image.width,
                image.height,
            ))
        })?;

    let dynamic = image::DynamicImage::ImageRgba8(rgba_img);
    let mut buf = Vec::new();
    dynamic
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| BaiduError::ImageEncoding(format!("PNG encoding failed: {e}")))?;
    Ok(buf)
}
