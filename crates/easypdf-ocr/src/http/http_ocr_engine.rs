//! 通用 HTTP OCR 引擎实现。

use easypdf_core::CapabilityLevel;
use easypdf_markdown::ocr::{OcrEngine, OcrImage, OcrResult};

use super::client::{HttpClientConfig, OcrHttpClient};
use super::error::Result;
use super::request::{OcrRequest, RequestConfig};
use super::response::OcrResponseParser;

/// 通用 HTTP OCR 引擎。
///
/// 将 [`OcrRequest`]（构建引擎专用请求体）与 [`OcrResponseParser`]（解析引擎专用响应）
/// 和 [`OcrHttpClient`]（处理传输、认证、重试、限流）组合在一起。
///
/// 该结构体实现了 `easypdf-markdown-ocr` 的 [`OcrEngine`] trait，可直接用于 OCR 处理流水线。
///
/// # 类型参数
///
/// * `R` - 请求构建器，负责构造引擎专用的 JSON 请求体。
/// * `P` - 响应解析器，负责从引擎专用 JSON 响应中提取识别结果。
pub struct HttpOcrEngine<R: OcrRequest, P: OcrResponseParser> {
    /// 请求构建器。
    request: R,
    /// 响应解析器。
    parser: P,
    /// HTTP 客户端（处理传输、认证、重试、限流）。
    client: OcrHttpClient,
    /// 请求配置（图像编码、语言提示等）。
    request_config: RequestConfig,
}

impl<R: OcrRequest, P: OcrResponseParser> std::fmt::Debug for HttpOcrEngine<R, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpOcrEngine")
            .field("engine_name", &self.request.engine_name())
            .field("endpoint", &self.request.endpoint())
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl<R: OcrRequest, P: OcrResponseParser> HttpOcrEngine<R, P> {
    /// 使用默认配置创建 HTTP OCR 引擎。
    ///
    /// # 参数
    ///
    /// * `request` - 请求构建器，负责构造引擎专用的 JSON 请求体。
    /// * `parser` - 响应解析器，负责从引擎专用 JSON 响应中提取识别结果。
    ///
    /// # Errors
    ///
    /// 若底层 HTTP 客户端构建失败，返回 `OcrHttpError::Transport`。
    pub fn new(request: R, parser: P) -> Result<Self> {
        let client = OcrHttpClient::new(request.endpoint(), request.auth().clone())?;
        Ok(Self {
            request,
            parser,
            client,
            request_config: RequestConfig::default(),
        })
    }

    /// 使用自定义配置创建 HTTP OCR 引擎。
    ///
    /// # 参数
    ///
    /// * `request` - 请求构建器。
    /// * `parser` - 响应解析器。
    /// * `config` - HTTP 客户端配置（超时、重试、限流等）。
    /// * `request_config` - 请求配置（图像编码、语言提示等）。
    ///
    /// # Errors
    ///
    /// 若底层 HTTP 客户端构建失败，返回 `OcrHttpError::Transport`。
    pub fn with_config(
        request: R,
        parser: P,
        config: HttpClientConfig,
        request_config: RequestConfig,
    ) -> Result<Self> {
        let client =
            OcrHttpClient::with_config(request.endpoint(), request.auth().clone(), config)?;
        Ok(Self {
            request,
            parser,
            client,
            request_config,
        })
    }

    /// 获取底层 HTTP 客户端的引用。
    #[must_use]
    pub fn client(&self) -> &OcrHttpClient {
        &self.client
    }
}

impl<R: OcrRequest, P: OcrResponseParser> OcrEngine for HttpOcrEngine<R, P> {
    fn recognize(
        &self,
        image: &OcrImage,
    ) -> std::result::Result<OcrResult, Box<dyn std::error::Error + Send + Sync>> {
        let body = self
            .request
            .build_request_body(image, &self.request_config)?;
        let extra = self.request.extra_headers();
        let extra_ref = if extra.is_empty() { None } else { Some(&extra) };

        let raw: serde_json::Value = self.client.post_json(&body, extra_ref)?;

        // 检查响应中的引擎级错误。
        self.parser.parse_response(&raw).map_err(Into::into)
    }

    fn name(&self) -> &'static str {
        self.request.engine_name()
    }

    fn languages(&self) -> &[&str] {
        self.request.languages()
    }

    fn level(&self) -> CapabilityLevel {
        CapabilityLevel::Cloud
    }
}
