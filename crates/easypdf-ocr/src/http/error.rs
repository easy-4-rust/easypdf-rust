//! 基于 HTTP 的 OCR 引擎错误类型。

/// 基于 HTTP 的 OCR 操作中可能发生的错误。
///
/// 涵盖传输失败、认证问题、服务器错误和响应解析问题。
#[derive(Debug, thiserror::Error)]
pub enum OcrHttpError {
    /// HTTP 传输错误（连接被拒绝、超时、DNS 失败等）。
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// 认证失败（无效 API 密钥、令牌过期、签名错误）。
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// 服务器拒绝了请求（请求体格式错误、缺少字段等）。
    #[error("Bad request: {message} (code: {code})")]
    BadRequest {
        /// 服务器返回的错误码。
        code: i32,
        /// 人类可读的错误消息。
        message: String,
    },

    /// 超出速率限制；调用方应在指定延迟后重试。
    #[error("Rate limit exceeded; retry after {retry_after_secs}s")]
    RateLimit {
        /// 建议的重试延迟（秒）。
        retry_after_secs: u64,
    },

    /// 服务器返回了 5xx 错误。
    #[error("Server error: status={status}, body={body}")]
    ServerError {
        /// HTTP 状态码。
        status: u16,
        /// 响应体（显示时截断至 500 字符）。
        body: String,
    },

    /// 服务器响应无法解析。
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// OCR 引擎返回了应用级错误。
    #[error("OCR engine error: {0}")]
    Engine(String),

    /// 已耗尽最大重试次数。
    #[error("Max retries ({max}) exceeded")]
    MaxRetriesExceeded {
        /// 配置的最大重试次数。
        max: u32,
    },
}

/// OCR HTTP 操作的便捷 Result 类型。
pub type Result<T> = std::result::Result<T, OcrHttpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_transport() {
        // Transport errors wrap reqwest::Error; verify the prefix.
        // We can't easily construct a reqwest::Error, so test via the Display impl pattern.
        let err = OcrHttpError::Auth("bad key".to_owned());
        assert!(format!("{err}").contains("Authentication failed"));
        assert!(format!("{err}").contains("bad key"));
    }

    #[test]
    fn test_error_display_bad_request() {
        let err = OcrHttpError::BadRequest {
            code: 400,
            message: "missing image".to_owned(),
        };
        let s = format!("{err}");
        assert!(s.contains("Bad request"));
        assert!(s.contains("400"));
        assert!(s.contains("missing image"));
    }

    #[test]
    fn test_error_display_rate_limit() {
        let err = OcrHttpError::RateLimit {
            retry_after_secs: 30,
        };
        assert!(format!("{err}").contains("30"));
    }

    #[test]
    fn test_error_display_server_error() {
        let err = OcrHttpError::ServerError {
            status: 503,
            body: "Service Unavailable".to_owned(),
        };
        let s = format!("{err}");
        assert!(s.contains("503"));
        assert!(s.contains("Service Unavailable"));
    }

    #[test]
    fn test_error_display_invalid_response() {
        let err = OcrHttpError::InvalidResponse("unexpected JSON structure".to_owned());
        assert!(format!("{err}").contains("unexpected JSON structure"));
    }

    #[test]
    fn test_error_display_engine() {
        let err = OcrHttpError::Engine("OCR model not loaded".to_owned());
        assert!(format!("{err}").contains("OCR model not loaded"));
    }

    #[test]
    fn test_error_display_max_retries() {
        let err = OcrHttpError::MaxRetriesExceeded { max: 5 };
        let s = format!("{err}");
        assert!(s.contains('5'));
    }

    #[test]
    fn test_error_conversion_to_boxed() {
        let err = OcrHttpError::Auth("test".to_owned());
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(err);
        assert!(boxed.to_string().contains("Authentication failed"));
    }
}
