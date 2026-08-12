# Cloud OCR Unified Design

**日期**: 2026-08-09
**作用范围**: `easypdf-ocr`（http / glm / hunyuan / baidu / deepseek）
**类型**: 平台抽象设计

---

## 1. 背景与问题

easypdf-rust 需要支持多种云 OCR 服务来实现 PDF 图片文字识别。每个云服务有不同的 API 协议、认证方式、请求格式和响应格式：

| 服务 | 认证 | 协议 | 特殊要求 |
|---|---|---|---|
| GLM-OCR（智谱 BigModel） | Bearer token | OpenAI 兼容 | base64 图片 |
| HunyuanOCR（腾讯云） | TC3-HMAC-SHA256 签名 | 腾讯云 API | 复杂签名流程 |
| Baidu Qianfan / PP-OCRv6 | OAuth 2.0 token | 百度 AI API | 14 个端点 + token 缓存 |
| DeepSeek-OCR-2 | Bearer token | OpenAI 兼容 | base64 图片 |

### 1.1 核心挑战

- **协议差异**: 4 种不同的认证和请求格式
- **错误处理**: 每种服务有不同的错误码和重试策略
- **限流**: 各服务有不同的速率限制策略
- **安全**: SSRF 防护（URL 校验）、API key 脱敏
- **Feature gate**: 用户应能按需启用特定引擎，避免引入不需要的依赖

---

## 2. 设计方案

### 2.1 统一抽象层 (`HttpOcrEngine`)

```rust
/// 统一 OCR 引擎 trait
pub trait OcrEngine: Send + Sync {
    /// 引擎名称
    fn name(&self) -> &str;
    
    /// 识别图片中的文字
    fn recognize(&self, image: &OcrImage) -> Result<OcrResult, OcrError>;
}

/// OCR 图片输入
pub struct OcrImage {
    pub data: Vec<u8>,
    pub format: ImageFormat,
}

/// OCR 识别结果
pub struct OcrResult {
    pub text: String,
    pub confidence: Option<f64>,
    pub regions: Vec<TextRegion>,
}

/// HTTP OCR 引擎统一实现
pub struct HttpOcrEngine {
    client: reqwest::blocking::Client,
    config: HttpOcrConfig,
    auth: AuthStrategy,
    parser: Box<dyn OcrResponseParser>,
}
```

### 2.2 认证策略 (`AuthStrategy`)

```rust
pub enum AuthStrategy {
    /// Bearer token（GLM / DeepSeek）
    Bearer { token: String },
    /// TC3-HMAC-SHA256 签名（腾讯云）
    TencentHmac {
        secret_id: String,
        secret_key: String,
        service: String,
    },
    /// OAuth 2.0 token exchange（百度）
    OAuth2 {
        client_id: String,
        client_secret: String,
        token_url: String,
        token_manager: Arc<Mutex<TokenManager>>,
    },
}
```

### 2.3 GLM-OCR 引擎

- **认证**: Bearer token（`Authorization: Bearer <api_key>`）
- **请求**: OpenAI 兼容格式（`messages` 数组，含 base64 图片）
- **Feature gate**: `ocr-glm`

```rust
pub fn create_glm_ocr_engine(config: GlmConfig) -> Box<dyn OcrEngine> {
    HttpOcrEngine::new(
        HttpOcrConfig { base_url: config.base_url, timeout: config.timeout },
        AuthStrategy::Bearer { token: config.api_key },
        Box::new(GlmOcrParser),
    )
}
```

### 2.4 HunyuanOCR 引擎

- **认证**: TC3-HMAC-SHA256 签名（腾讯云标准签名流程）
- **请求**: 腾讯云 API 格式（`Action` + `Version` + 参数）
- **签名**: `sign_tencent_cloud_request()` 函数（`http/auth.rs`）
- **Feature gate**: `ocr-hunyuan`

```rust
pub fn create_hunyuan_ocr_engine(config: HunyuanConfig) -> Box<dyn OcrEngine> {
    HttpOcrEngine::new(
        HttpOcrConfig { base_url: config.base_url, timeout: config.timeout },
        AuthStrategy::TencentHmac {
            secret_id: config.secret_id,
            secret_key: config.secret_key,
            service: "hunyuan".into(),
        },
        Box::new(HunyuanOcrParser),
    )
}
```

### 2.5 Baidu OCR 引擎

- **认证**: OAuth 2.0 token exchange（`client_id` + `client_secret` → access_token）
- **请求**: 百度 AI API 格式（form-urlencoded）
- **14 个端点**（通过 `BaiduApi` 枚举选择）:
  - GeneralBasic / GeneralAccurate / GeneralBasicWithLocation / GeneralAccurateWithLocation
  - TableRecognitionV2 / WebImage / WebImageWithLocation / OfficeDocument
  - Handwriting / Seal / Digit / Qrcode / Structured
- **Token 管理**: `TokenManager` 缓存 access_token，过期自动刷新
- **Feature gate**: `ocr-baidu`

```rust
pub struct BaiduOcrEngine {
    client: reqwest::blocking::Client,
    config: BaiduConfig,
    api: BaiduApi,
    token_manager: TokenManager,
    parser: BaiduOcrParser,
}

impl OcrEngine for BaiduOcrEngine {
    fn recognize(&self, image: &OcrImage) -> Result<OcrResult, OcrError> {
        let token = self.token_manager.get_or_refresh(&self.client, &self.config)?;
        let url = self.api.endpoint_url();
        // ... 发送请求，解析响应
    }
}
```

### 2.6 DeepSeek-OCR-2 引擎

- **认证**: Bearer token（OpenAI 兼容）
- **请求**: OpenAI 兼容格式（与 GLM 类似）
- **Feature gate**: `ocr-deepseek`
- **实现**: 通过 `HttpOcrEngine` 的 OpenAI 兼容模式，复用 `AuthStrategy::Bearer`

> **注意**: DeepSeek OCR 没有独立模块目录，而是通过 `HttpOcrEngine` 的通用 OpenAI 兼容协议实现。在 `http/auth.rs` 和 `http/image.rs` 中以注释形式引用。

### 2.7 限流与重试

```rust
/// 限流配置
pub struct RateLimitConfig {
    pub max_requests_per_second: u32,
    pub burst_size: u32,
}

/// 重试策略
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed(Duration),
    /// 指数退避
    Exponential { base: Duration, max: Duration, max_attempts: u32 },
}
```

- `RateLimitConfig` 控制每秒最大请求数
- `BackoffStrategy` 控制失败重试策略
- 限流器使用令牌桶算法

### 2.8 错误处理

```rust
pub enum OcrHttpError {
    /// HTTP 请求失败
    Http { status: u16, body: String },
    /// 速率限制
    RateLimit { retry_after_secs: u64 },
    /// 认证失败
    Auth(String),
    /// 响应解析失败
    Parse(String),
    /// 最大重试次数用尽
    MaxRetriesExceeded,
    /// URL 被 SSRF 防护拦截
    SsrfBlocked(String),
}
```

### 2.9 SSRF 防护

OCR 引擎在发送 HTTP 请求前，通过 `easypdf_core::io::ssrf_guard::validate_url()` 校验 URL：

- 禁止 `file://` / `ftp://` 等非 HTTP 协议
- 禁止私有 IP（10.x / 172.16-31.x / 192.168.x）
- 禁止 loopback（127.x / ::1）
- 禁止 IPv6 link-local（fe80::）
- 禁止 IPv6 ULA（fc00:: / fd00::）
- 禁止 IPv4-mapped IPv6（::ffff:127.0.0.1）

### 2.10 API Key Debug 脱敏

所有包含 `api_key` / `secret_key` 的结构体（`GlmConfig` / `BaiduConfig` / `HunyuanConfig` 等）在 `Debug` 输出中脱敏：

```rust
impl fmt::Debug for GlmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlmConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}
```

---

## 3. 测试改动范围

- `easypdf-ocr/src/http/client/tests.rs` -- HTTP 客户端 mock 测试
- `easypdf-ocr/src/glm/` -- GLM 引擎测试
- `easypdf-ocr/src/hunyuan/` -- Hunyuan 引擎测试
- `easypdf-ocr/src/baidu/` -- Baidu 引擎测试（14 端点覆盖）
- `fuzz/fuzz_targets/ssrf_url.rs` -- SSRF 防护 fuzz 测试
- `easypdf-markdown/` -- OCR 处理器集成测试

---

## 4. 不在范围内（YAGNI）

- 不实现 OCR 引擎抽象为通用 trait（`OcrEngine` trait 在 `easypdf-markdown::ocr` 中定义）
- 不实现异步 HTTP 客户端（使用 reqwest blocking，后续可升级）
- 不实现 OCR 结果缓存（由上层 Markdown 转换流程负责）
- 不实现本地 OCR（Tesseract 等）-- 仅云端 OCR
- 不实现 OCR 图片预处理（旋转/裁剪/增强）

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 云服务 API 变更 | OCR 功能失效 | 版本锁定 + 监控 changelog |
| API key 泄露 | 安全风险 | Debug 脱敏 + 环境变量配置 |
| SSRF 绕过 | 安全风险 | 全面的 IPv4/IPv6 校验 + fuzz 测试 |
| 速率限制 | 功能降级 | 限流器 + 指数退避重试 |
| 网络超时 | 用户体验差 | 可配置超时 + 重试策略 |
| Baidu token 过期 | 请求失败 | TokenManager 自动刷新 |

---

## 6. 实施顺序

1. 实现 `HttpOcrEngine` 统一抽象（`http/mod.rs`）
2. 实现认证策略（`http/auth.rs`）-- Bearer / TencentHmac / OAuth2
3. 实现 HTTP 客户端（`http/client/`）-- reqwest blocking + 超时 + 重试
4. 实现限流器（`http/rate_limit.rs`）-- 令牌桶算法
5. 实现图片编码（`http/image.rs`）-- base64 编码
6. 实现错误类型（`http/error.rs`）-- `OcrHttpError` 枚举
7. 实现 GLM 引擎（`glm/`）-- GlmConfig + GlmOcrParser
8. 实现 Hunyuan 引擎（`hunyuan/`）-- HunyuanConfig + TC3 签名
9. 实现 Baidu 引擎（`baidu/`）-- BaiduConfig + 14 端点 + TokenManager
10. SSRF 防护集成 + fuzz 测试
11. API key Debug 脱敏
12. Feature gate 配置（Cargo.toml）
