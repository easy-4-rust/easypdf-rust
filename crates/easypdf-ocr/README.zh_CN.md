# easypdf-ocr

> 云 OCR 引擎层：统一 HTTP 基类，集成 4 大引擎（GLM、混元 OCR、百度 OCR），支持 5 种认证方式。

## 角色

`easypdf-ocr` 为 easypdf-rust 工作区提供云端 OCR 引擎集成。它封装了三个主流中文 OCR 云服务（智谱 GLM、腾讯混元 OCR、百度 OCR），并通过通用的 `HttpOcrEngine` 基类统一处理认证、重试、限流和图片编码等基础设施，使各引擎只需关注请求构建与响应解析。

## 核心能力

- **GLM OCR**（智谱 AI）——通用文字识别，Bearer Token 认证（`crates/easypdf-ocr/src/glm/`）
- **混元 OCR**（腾讯）——通用/表格/手写等多种模式，HMAC-SHA256 签名认证（`crates/easypdf-ocr/src/hunyuan/`）
- **百度 OCR**——通用文字识别，AK/SK + Token 管理（`crates/easypdf-ocr/src/baidu/`）
- **HTTP 基类**（`HttpOcrEngine`）——通用 HTTP OCR 引擎框架，实现 `OcrEngine` trait（`crates/easypdf-ocr/src/http/client/`）
- **认证抽象**（`AuthMethod`）——Bearer / AK-SK / HMAC-SHA256 / NoAuth / 自定义 Header（`crates/easypdf-ocr/src/http/auth.rs`）
- **重试与限流**（`BackoffStrategy`、`RateLimitConfig`）——指数退避 + 令牌桶限流（`crates/easypdf-ocr/src/http/retry.rs`、`crates/easypdf-ocr/src/http/rate_limit.rs`）
- **图片编码**（`EncodedImage`、`ImageEncoding`）——Base64 内联或 multipart 上传（`crates/easypdf-ocr/src/http/image.rs`）
- **响应解析**（`OcrResponseParser`）——可插拔的响应解析器 trait（`crates/easypdf-ocr/src/http/response.rs`）

## 依赖

### 内部依赖

| Crate | 用途 |
|-------|------|
| `easypdf-core` | 核心类型（`CapabilityLevel`） |
| `easypdf-markdown` | OCR 抽象 trait（`OcrEngine`、`OcrImage`、`OcrResult`） |

### 外部依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `reqwest` | 0.12 | 同步 HTTP 客户端（features: json, rustls-tls, multipart, blocking） |
| `serde` / `serde_json` | 1.x | 序列化 |
| `hmac` / `sha2` | -- | API 签名算法 |
| `base64` | -- | 图片编码 |

## 主要 API

### 创建 OCR 引擎

```rust
use easypdf_ocr::{
    create_glm_ocr_engine, GlmConfig,
    create_hunyuan_ocr_engine, HunyuanConfig,
    BaiduOcrEngine, BaiduConfig,
};

// GLM
let glm = create_glm_ocr_engine(GlmConfig {
    api_key: "your-key".into(),
    ..Default::default()
})?;

// 混元 OCR
let hunyuan = create_hunyuan_ocr_engine(HunyuanConfig {
    secret_id: "id".into(),
    secret_key: "key".into(),
    ..Default::default()
})?;

// 百度 OCR
let baidu = BaiduOcrEngine::new(BaiduConfig {
    api_key: "ak".into(),
    secret_key: "sk".into(),
    ..Default::default()
})?;
```

### HttpOcrEngine（通用基类）

```rust
use easypdf_ocr::{build_http_engine, HttpClientConfig, build_http_engine_with_config};

// 简单构建
let engine = build_http_engine(request, parser)?;

// 自定义配置
let engine = build_http_engine_with_config(request, parser, HttpClientConfig {
    max_retries: 3,
    ..Default::default()
})?;
```

### 使用 OCR 引擎

```rust
use easypdf_markdown::{OcrEngine, OcrImage};

let image = OcrImage::from_path("scan.png")?;
let result = engine.recognize(&image)?;
println!("识别文本: {}", result.text);
println!("置信度: {:?}", result.confidence);
```

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-ocr
**docs.rs**：https://docs.rs/easypdf-ocr
