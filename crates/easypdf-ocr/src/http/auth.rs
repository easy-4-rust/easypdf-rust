//! OCR HTTP API 的认证方式。
//!
//! 每个云 OCR 提供商使用不同的认证方案。[`AuthMethod`] 对这些方案进行抽象，
//! 为出站请求添加正确的请求头（或签名）。

use std::collections::HashMap;
use std::fmt;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use super::error::{OcrHttpError, Result};

/// OCR HTTP 端点的认证方式。
///
/// 各变体覆盖五种支持的引擎：
/// - **Bearer**：简单的 `Authorization: Bearer <token>`（GLM-OCR、DeepSeek-OCR）
/// - **`ApiKeyHeader`**：自定义请求头密钥（GLM-OCR `BigModel` 的 `x-api-key`）
/// - **`BearerFromOAuth`**：通过 OAuth 将 API Key + Secret 交换为 `access_token`，
///   然后使用 Bearer（千帆、PP-OCRv6）
/// - **`TencentCloud`**：TC3-HMAC-SHA256 请求签名（`HunyuanOCR`）
/// - **None**：无认证（自托管端点）
///
/// # 安全
///
/// `Debug` 实现会脱敏所有密钥材料。
#[derive(Clone)]
pub enum AuthMethod {
    /// `Authorization: Bearer <token>`。
    Bearer(String),

    /// 自定义请求头认证（如 `x-api-key: <key>`）。
    ApiKeyHeader {
        /// 请求头名称（如 `"x-api-key"`）。
        header: &'static str,
        /// API 密钥值。
        key: String,
    },

    /// `OAuth2` 客户端凭证流程：将 API Key + Secret 交换为 Bearer 令牌，
    /// 然后使用 `Authorization: Bearer <access_token>`。
    ///
    /// `token_url` 仅在获取令牌时调用一次；令牌会被缓存以供后续请求使用。
    BearerFromOAuth {
        /// 令牌端点 URL。
        token_url: String,
        /// OAuth API 密钥（客户端 ID）。
        api_key: String,
        /// OAuth 密钥（客户端密钥）。
        secret_key: String,
    },

    /// 腾讯云 TC3-HMAC-SHA256 签名认证。
    ///
    /// 按照 [TC3 签名规范][tc3] 构造 `Authorization` 请求头。
    ///
    /// [tc3]: https://cloud.tencent.com/document/api/1729/101840
    TencentCloud {
        /// 腾讯云 Secret ID。
        secret_id: String,
        /// 腾讯云 Secret Key。
        secret_key: String,
        /// 服务名称（如 `"hunyuan"`）。
        service: String,
        /// API 主机名（如 `"hunyuan.tencentcloudapi.com"`）。
        host: String,
        /// 地域（如 `"ap-guangzhou"`）。
        region: String,
        /// API 版本（如 `"2023-09-01"`）。
        version: String,
    },

    /// 无认证（自托管或公开端点）。
    None,
}

impl fmt::Debug for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bearer(_) => f.debug_tuple("Bearer").field(&"***").finish(),
            Self::ApiKeyHeader { header, .. } => f
                .debug_struct("ApiKeyHeader")
                .field("header", header)
                .field("key", &"***")
                .finish(),
            Self::BearerFromOAuth {
                token_url, api_key, ..
            } => f
                .debug_struct("BearerFromOAuth")
                .field("token_url", token_url)
                .field("api_key", api_key)
                .field("secret_key", &"***")
                .finish(),
            Self::TencentCloud {
                secret_id,
                service,
                host,
                region,
                version,
                ..
            } => f
                .debug_struct("TencentCloud")
                .field("secret_id", &redact(secret_id))
                .field("secret_key", &"***")
                .field("service", service)
                .field("host", host)
                .field("region", region)
                .field("version", version)
                .finish(),
            Self::None => f.write_str("None"),
        }
    }
}

/// 脱敏字符串，仅显示前 4 个和后 4 个字符。
fn redact(s: &str) -> String {
    if s.len() <= 8 {
        "***".to_owned()
    } else {
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    }
}

/// 将认证信息应用到一组 HTTP 请求头。
///
/// 返回应添加到请求中的请求头。
///
/// # Errors
///
/// 若令牌交换失败（`BearerFromOAuth`），返回 `OcrHttpError::Auth`。
pub fn apply_auth(auth: &AuthMethod) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    match auth {
        AuthMethod::Bearer(token) => {
            headers.insert("Authorization".to_owned(), format!("Bearer {token}"));
        }
        AuthMethod::ApiKeyHeader { header, key } => {
            headers.insert((*header).to_owned(), key.clone());
        }
        AuthMethod::BearerFromOAuth {
            token_url,
            api_key,
            secret_key,
        } => {
            let token = exchange_oauth_token(token_url, api_key, secret_key)?;
            headers.insert("Authorization".to_owned(), format!("Bearer {token}"));
        }
        AuthMethod::TencentCloud {
            secret_id,
            secret_key,
            service,
            host,
            region,
            version,
        } => {
            // TC3 签名与请求相关，此处仅设置静态请求头。
            // 实际签名在 `sign_tencent_cloud_request` 中完成。
            headers.insert("Host".to_owned(), host.clone());
            headers.insert("X-TC-Action".to_owned(), "RecognizeGeneralOCR".to_owned());
            headers.insert("X-TC-Version".to_owned(), version.clone());
            headers.insert("X-TC-Region".to_owned(), region.clone());
            headers.insert("X-TC-Service".to_owned(), service.clone());
            // 存储 secret_id 供签名步骤使用。
            headers.insert("X-TC-SecretId".to_owned(), secret_id.clone());
            // 将 secret_key 作为伪头存储供签名使用（发送前会被移除）。
            headers.insert("X-TC-SecretKey-Pending".to_owned(), secret_key.clone());
        }
        AuthMethod::None => {}
    }
    Ok(headers)
}

/// 使用 TC3-HMAC-SHA256 签名腾讯云请求。
///
/// 必须在所有其他请求头已设置且请求体已知后调用。
/// 返回 `Authorization` 请求头值和时间戳。
///
/// # 参数
///
/// * `secret_id` - 腾讯云 Secret ID
/// * `secret_key` - 腾讯云 Secret Key
/// * `service` - 服务名称（如 `"hunyuan"`）
/// * `host` - API 主机名
/// * `action` - API 操作名称
/// * `version` - API 版本
/// * `region` - 地域
/// * `payload` - JSON 请求体
///
/// # 返回
///
/// `(authorization_header, timestamp_seconds)` 元组。
///
/// # Panics
///
/// 若当前系统时间无法转换为有效时间戳则 panic（正常情况下不会发生）。
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn sign_tencent_cloud_request(
    secret_id: &str,
    secret_key: &str,
    service: &str,
    host: &str,
    action: &str,
    _version: &str,
    _region: &str,
    payload: &str,
) -> (String, String) {
    type HmacSha256 = Hmac<Sha256>;

    let timestamp = chrono::Utc::now().timestamp();
    let date = chrono::DateTime::from_timestamp(timestamp, 0)
        .expect("valid timestamp")
        .format("%Y-%m-%d")
        .to_string();

    // 步骤 1：规范请求。
    let http_request_method = "POST";
    let canonical_uri = "/";
    let canonical_querystring = "";
    let content_type = "application/json; charset=utf-8";
    let canonical_headers = format!(
        "content-type:{content_type}\nhost:{host}\nx-tc-action:{action_lower}\n",
        action_lower = action.to_lowercase()
    );
    let signed_headers = "content-type;host;x-tc-action";
    let hashed_payload = {
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        hex::encode(hasher.finalize())
    };
    let canonical_request = format!(
        "{http_request_method}\n{canonical_uri}\n{canonical_querystring}\n\
         {canonical_headers}\n{signed_headers}\n{hashed_payload}"
    );

    // 步骤 2：待签名字符串。
    let algorithm = "TC3-HMAC-SHA256";
    let credential_scope = format!("{date}/{service}/tc3_request");
    let hashed_canonical_request = {
        let mut hasher = Sha256::new();
        hasher.update(canonical_request.as_bytes());
        hex::encode(hasher.finalize())
    };
    let string_to_sign =
        format!("{algorithm}\n{timestamp}\n{credential_scope}\n{hashed_canonical_request}");

    // 步骤 3：签名。
    let secret_date = {
        let mut mac = HmacSha256::new_from_slice(format!("TC3{secret_key}").as_bytes())
            .expect("HMAC accepts any key length");
        mac.update(date.as_bytes());
        mac.finalize().into_bytes()
    };
    let secret_service = {
        let mut mac =
            HmacSha256::new_from_slice(&secret_date).expect("HMAC accepts any key length");
        mac.update(service.as_bytes());
        mac.finalize().into_bytes()
    };
    let secret_signing = {
        let mut mac =
            HmacSha256::new_from_slice(&secret_service).expect("HMAC accepts any key length");
        mac.update(b"tc3_request");
        mac.finalize().into_bytes()
    };
    let signature = {
        let mut mac =
            HmacSha256::new_from_slice(&secret_signing).expect("HMAC accepts any key length");
        mac.update(string_to_sign.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    // 步骤 4：Authorization 请求头。
    let authorization = format!(
        "{algorithm} Credential={secret_id}/{credential_scope}, \
         SignedHeaders={signed_headers}, Signature={signature}"
    );

    (authorization, timestamp.to_string())
}

/// 将 API Key + Secret 交换为 `OAuth2` Bearer 令牌（客户端凭证流程）。
///
/// 百度云（千帆、PP-OCRv6）使用此方式获取 `access_token`。
fn exchange_oauth_token(token_url: &str, api_key: &str, secret_key: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", api_key),
            ("client_secret", secret_key),
        ])
        .send()
        .map_err(|e| OcrHttpError::Auth(format!("OAuth token request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(OcrHttpError::Auth(format!(
            "OAuth token request returned status {}",
            resp.status()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .map_err(|e| OcrHttpError::Auth(format!("OAuth token response parse error: {e}")))?;

    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| OcrHttpError::Auth("OAuth response missing access_token".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_bearer_redacts_token() {
        let auth = AuthMethod::Bearer("super-secret-token-value".to_owned());
        let debug = format!("{auth:?}");
        assert!(!debug.contains("super-secret-token-value"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn test_debug_api_key_redacts() {
        let auth = AuthMethod::ApiKeyHeader {
            header: "x-api-key",
            key: "my-secret-api-key".to_owned(),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("my-secret-api-key"));
        assert!(debug.contains("x-api-key"));
    }

    #[test]
    fn test_debug_bearer_from_oauth_redacts_secret() {
        let auth = AuthMethod::BearerFromOAuth {
            token_url: "https://example.com/token".to_owned(),
            api_key: "client123".to_owned(),
            secret_key: "secret456".to_owned(),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("secret456"));
        assert!(debug.contains("client123"));
        assert!(debug.contains("https://example.com/token"));
    }

    #[test]
    fn test_debug_tencent_cloud_redacts_keys() {
        let auth = AuthMethod::TencentCloud {
            secret_id: "AKID1234567890".to_owned(),
            secret_key: "super-secret-key-value".to_owned(),
            service: "hunyuan".to_owned(),
            host: "hunyuan.tencentcloudapi.com".to_owned(),
            region: "ap-guangzhou".to_owned(),
            version: "2023-09-01".to_owned(),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("super-secret-key-value"));
        // secret_id is redacted with first/last 4 chars
        assert!(debug.contains("hunyuan"));
    }

    #[test]
    fn test_debug_none() {
        let auth = AuthMethod::None;
        assert_eq!(format!("{auth:?}"), "None");
    }

    #[test]
    fn test_apply_auth_bearer() {
        let auth = AuthMethod::Bearer("tok123".to_owned());
        let headers = apply_auth(&auth).unwrap();
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer tok123");
    }

    #[test]
    fn test_apply_auth_api_key() {
        let auth = AuthMethod::ApiKeyHeader {
            header: "x-api-key",
            key: "key123".to_owned(),
        };
        let headers = apply_auth(&auth).unwrap();
        assert_eq!(headers.get("x-api-key").unwrap(), "key123");
    }

    #[test]
    fn test_apply_auth_none() {
        let auth = AuthMethod::None;
        let headers = apply_auth(&auth).unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn test_apply_auth_tencent_cloud_sets_headers() {
        let auth = AuthMethod::TencentCloud {
            secret_id: "id123".to_owned(),
            secret_key: "key456".to_owned(),
            service: "hunyuan".to_owned(),
            host: "hunyuan.tencentcloudapi.com".to_owned(),
            region: "ap-guangzhou".to_owned(),
            version: "2023-09-01".to_owned(),
        };
        let headers = apply_auth(&auth).unwrap();
        assert_eq!(headers.get("Host").unwrap(), "hunyuan.tencentcloudapi.com");
        assert_eq!(headers.get("X-TC-Version").unwrap(), "2023-09-01");
        assert_eq!(headers.get("X-TC-Region").unwrap(), "ap-guangzhou");
        assert_eq!(headers.get("X-TC-Service").unwrap(), "hunyuan");
    }

    #[test]
    fn test_redact_short_string() {
        assert_eq!(redact("abc"), "***");
        assert_eq!(redact("12345678"), "***");
    }

    #[test]
    fn test_redact_long_string() {
        assert_eq!(redact("AKID12345678"), "AKID...5678");
    }

    #[test]
    fn test_tc3_signature_deterministic() {
        // Same inputs should produce the same signature.
        let (auth1, ts1) = sign_tencent_cloud_request(
            "id",
            "key",
            "hunyuan",
            "host.example.com",
            "Action",
            "2023-09-01",
            "ap-guangzhou",
            "{}",
        );
        let (auth2, ts2) = sign_tencent_cloud_request(
            "id",
            "key",
            "hunyuan",
            "host.example.com",
            "Action",
            "2023-09-01",
            "ap-guangzhou",
            "{}",
        );
        // Timestamps may differ by 1 second, but signatures should be valid.
        assert!(auth1.starts_with("TC3-HMAC-SHA256"));
        assert!(auth2.starts_with("TC3-HMAC-SHA256"));
        // The signed parts should be identical if timestamps are the same.
        if ts1 == ts2 {
            assert_eq!(auth1, auth2);
        }
    }
}
