use easypdf_markdown::ocr::OcrResult;

use super::parsers;
use crate::baidu::config::{BaiduApi, BaiduError, BaiduResult};

/// Parser for Baidu Cloud OCR JSON responses.
///
/// Configured with a [`BaiduApi`] variant to select the correct parsing logic.
#[derive(Debug, Clone)]
pub struct BaiduOcrParser {
    /// Which API variant is being used (determines parsing strategy).
    api: BaiduApi,
}

impl BaiduOcrParser {
    /// Create a new parser for the given API variant.
    #[must_use]
    pub const fn new(api: BaiduApi) -> Self {
        Self { api }
    }

    /// Parse a Baidu OCR JSON response into an [`OcrResult`].
    ///
    /// # Errors
    ///
    /// Returns [`BaiduError::Api`] if the response contains a Baidu error code,
    /// or [`BaiduError::InvalidResponse`] if the JSON structure is unexpected.
    pub fn parse(&self, raw: &serde_json::Value) -> BaiduResult<OcrResult> {
        // Check for Baidu-level errors first.
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
            // GeneralBasic, GeneralAccurate, WithLocation variants,
            // WebImage, WebImageWithLocation, Handwriting, Digit
            // all use the standard words_result format.
            _ => parsers::parse_words_response(raw),
        }
    }
}

/// Extract a `u32` coordinate from a JSON value that may be `u64`.
pub(crate) fn json_u32(val: Option<&serde_json::Value>) -> u32 {
    val.and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}
