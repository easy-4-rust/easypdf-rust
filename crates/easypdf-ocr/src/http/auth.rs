//! Authentication methods for OCR HTTP APIs.
//!
//! Each cloud OCR provider uses a different authentication scheme.
//! [`AuthMethod`] abstracts over these, applying the correct headers
//! (or signature) to outgoing requests.

use std::collections::HashMap;
use std::fmt;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use super::error::{OcrHttpError, Result};

/// Authentication method for an OCR HTTP endpoint.
///
/// Variants cover the five supported engines:
/// - **Bearer**: simple `Authorization: Bearer <token>` (GLM-OCR, DeepSeek-OCR)
/// - **`ApiKeyHeader`**: custom header key (GLM-OCR `BigModel` `x-api-key`)
/// - **`BearerFromOAuth`**: exchange API key + secret for an `access_token` via
///   OAuth, then use Bearer (Qianfan, PP-OCRv6)
/// - **`TencentCloud`**: TC3-HMAC-SHA256 request signing (`HunyuanOCR`)
/// - **None**: no authentication (self-hosted endpoints)
///
/// # Security
///
/// The `Debug` implementation redacts all secret material.
#[derive(Clone)]
pub enum AuthMethod {
    /// `Authorization: Bearer <token>`.
    Bearer(String),

    /// Custom header authentication (e.g., `x-api-key: <key>`).
    ApiKeyHeader {
        /// Header name (e.g., `"x-api-key"`).
        header: &'static str,
        /// The API key value.
        key: String,
    },

    /// `OAuth2` client-credentials flow: exchange API key + secret for a Bearer
    /// token, then use `Authorization: Bearer <access_token>`.
    ///
    /// The `token_url` is called once to obtain the token; the token is cached
    /// for subsequent requests.
    BearerFromOAuth {
        /// Token endpoint URL.
        token_url: String,
        /// OAuth API key (client ID).
        api_key: String,
        /// OAuth secret key (client secret).
        secret_key: String,
    },

    /// Tencent Cloud TC3-HMAC-SHA256 signature authentication.
    ///
    /// Constructs the `Authorization` header per the [TC3 signature spec][tc3].
    ///
    /// [tc3]: https://cloud.tencent.com/document/api/1729/101840
    TencentCloud {
        /// Tencent Cloud secret ID.
        secret_id: String,
        /// Tencent Cloud secret key.
        secret_key: String,
        /// Service name (e.g., `"hunyuan"`).
        service: String,
        /// API host (e.g., `"hunyuan.tencentcloudapi.com"`).
        host: String,
        /// Region (e.g., `"ap-guangzhou"`).
        region: String,
        /// API version (e.g., `"2023-09-01"`).
        version: String,
    },

    /// No authentication (self-hosted or public endpoints).
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

/// Redact a string, showing only the first 4 and last 4 characters.
fn redact(s: &str) -> String {
    if s.len() <= 8 {
        "***".to_owned()
    } else {
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    }
}

/// Apply authentication to a set of HTTP headers.
///
/// Returns the headers that should be added to the request.
///
/// # Errors
///
/// Returns `OcrHttpError::Auth` if token exchange fails (for `BearerFromOAuth`).
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
            // TC3 signature is request-specific; we only set the static headers here.
            // The actual signing happens in `sign_tencent_cloud_request`.
            headers.insert("Host".to_owned(), host.clone());
            headers.insert("X-TC-Action".to_owned(), "RecognizeGeneralOCR".to_owned());
            headers.insert("X-TC-Version".to_owned(), version.clone());
            headers.insert("X-TC-Region".to_owned(), region.clone());
            headers.insert("X-TC-Service".to_owned(), service.clone());
            // Store secret_id for the signing step.
            headers.insert("X-TC-SecretId".to_owned(), secret_id.clone());
            // Store secret_key as a pseudo-header for signing (removed before sending).
            headers.insert("X-TC-SecretKey-Pending".to_owned(), secret_key.clone());
        }
        AuthMethod::None => {}
    }
    Ok(headers)
}

/// Sign a Tencent Cloud request with TC3-HMAC-SHA256.
///
/// This must be called after all other headers are set and the request body
/// is known. It returns the `Authorization` header value and timestamp.
///
/// # Arguments
///
/// * `secret_id` - Tencent Cloud secret ID
/// * `secret_key` - Tencent Cloud secret key
/// * `service` - Service name (e.g., `"hunyuan"`)
/// * `host` - API host
/// * `action` - API action name
/// * `version` - API version
/// * `region` - Region
/// * `payload` - JSON request body
///
/// # Returns
///
/// A tuple of `(authorization_header, timestamp_seconds)`.
///
/// # Panics
///
/// Panics if the current system time cannot be converted to a valid timestamp
/// (should not happen in practice).
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

    // Step 1: Canonical request.
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

    // Step 2: String to sign.
    let algorithm = "TC3-HMAC-SHA256";
    let credential_scope = format!("{date}/{service}/tc3_request");
    let hashed_canonical_request = {
        let mut hasher = Sha256::new();
        hasher.update(canonical_request.as_bytes());
        hex::encode(hasher.finalize())
    };
    let string_to_sign = format!("{algorithm}\n{timestamp}\n{credential_scope}\n{hashed_canonical_request}");

    // Step 3: Signature.
    let secret_date = {
        let mut mac = HmacSha256::new_from_slice(format!("TC3{secret_key}").as_bytes())
            .expect("HMAC accepts any key length");
        mac.update(date.as_bytes());
        mac.finalize().into_bytes()
    };
    let secret_service = {
        let mut mac = HmacSha256::new_from_slice(&secret_date)
            .expect("HMAC accepts any key length");
        mac.update(service.as_bytes());
        mac.finalize().into_bytes()
    };
    let secret_signing = {
        let mut mac = HmacSha256::new_from_slice(&secret_service)
            .expect("HMAC accepts any key length");
        mac.update(b"tc3_request");
        mac.finalize().into_bytes()
    };
    let signature = {
        let mut mac = HmacSha256::new_from_slice(&secret_signing)
            .expect("HMAC accepts any key length");
        mac.update(string_to_sign.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    // Step 4: Authorization header.
    let authorization = format!(
        "{algorithm} Credential={secret_id}/{credential_scope}, \
         SignedHeaders={signed_headers}, Signature={signature}"
    );

    (authorization, timestamp.to_string())
}

/// Exchange API key + secret for an `OAuth2` Bearer token (client credentials flow).
///
/// Used by Baidu Cloud (Qianfan, PP-OCRv6) to obtain `access_token`.
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
        assert_eq!(
            headers.get("Host").unwrap(),
            "hunyuan.tencentcloudapi.com"
        );
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
            "id", "key", "hunyuan", "host.example.com", "Action", "2023-09-01", "ap-guangzhou", "{}",
        );
        let (auth2, ts2) = sign_tencent_cloud_request(
            "id", "key", "hunyuan", "host.example.com", "Action", "2023-09-01", "ap-guangzhou", "{}",
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
