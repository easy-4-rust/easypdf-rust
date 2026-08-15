use easypdf_markdown::ocr::OcrResult;

use super::parsers;
use crate::baidu::config::{BaiduApi, BaiduError, BaiduResult};

/// 百度云 OCR JSON 响应解析器。
///
/// 配置 [`BaiduApi`] 变体以选择正确的解析逻辑。
#[derive(Debug, Clone)]
pub struct BaiduOcrParser {
    /// 使用的 API 变体（决定解析策略）。
    api: BaiduApi,
}

impl BaiduOcrParser {
    /// 为给定 API 变体创建解析器。
    #[must_use]
    pub const fn new(api: BaiduApi) -> Self {
        Self { api }
    }

    /// 将百度 OCR JSON 响应解析为 [`OcrResult`]。
    ///
    /// # Errors
    ///
    /// 若响应包含百度错误码，返回 [`BaiduError::Api`]；
    /// 若 JSON 结构不符合预期，返回 [`BaiduError::InvalidResponse`]。
    pub fn parse(&self, raw: &serde_json::Value) -> BaiduResult<OcrResult> {
        // 首先检查百度级错误。
        if let Some(error_code) = raw.get("error_code").and_then(serde_json::Value::as_i64) {
            let message = raw
                .get("error_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_owned();
            return Err(BaiduError::Api {
                code: error_code,
                message,
            });
        }

        match self.api {
            BaiduApi::TableRecognitionV2 => parsers::parse_table_response(raw),
            BaiduApi::OfficeDocument => parsers::parse_office_doc_response(raw),
            BaiduApi::Seal => parsers::parse_seal_response(raw),
            BaiduApi::Qrcode => parsers::parse_qrcode_response(raw),
            BaiduApi::Structured => parsers::parse_structured_response(raw),
            BaiduApi::QianfanOcr => parsers::parse_qianfan_response(raw),
            // GeneralBasic、GeneralAccurate、WithLocation 变体、
            // WebImage、WebImageWithLocation、Handwriting、Digit
            // 均使用标准 words_result 格式。
            _ => parsers::parse_words_response(raw),
        }
    }
}

/// 从可能为 `u64` 的 JSON 值中提取 `u32` 坐标。
pub(crate) fn json_u32(val: Option<&serde_json::Value>) -> u32 {
    val.and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}
