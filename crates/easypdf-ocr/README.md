# easypdf-ocr

> Cloud OCR engine layer: unified HTTP base class with 4 engine integrations (GLM, HunyuanOCR, Baidu OCR) and 5 authentication methods.

## Role

`easypdf-ocr` provides cloud OCR engine integration for the easypdf-rust workspace. It wraps three major Chinese OCR cloud services (Zhipu GLM, Tencent HunyuanOCR, Baidu OCR) behind a shared `HttpOcrEngine` base class that handles authentication, retries, rate limiting, and image encoding. Each engine only needs to implement request building and response parsing.

## Core Capabilities

- **GLM OCR** (Zhipu AI) -- general text recognition with Bearer Token authentication (`crates/easypdf-ocr/src/glm/`)
- **HunyuanOCR** (Tencent) -- general/table/handwriting modes with HMAC-SHA256 signature authentication (`crates/easypdf-ocr/src/hunyuan/`)
- **Baidu OCR** -- general text recognition with AK/SK + token management (`crates/easypdf-ocr/src/baidu/`)
- **HTTP base class** (`HttpOcrEngine`) -- generic HTTP OCR engine framework implementing `OcrEngine` trait (`crates/easypdf-ocr/src/http/client/`)
- **Authentication abstraction** (`AuthMethod`) -- Bearer / AK-SK / HMAC-SHA256 / NoAuth / Custom header (`crates/easypdf-ocr/src/http/auth.rs`)
- **Retry & rate limiting** (`BackoffStrategy`, `RateLimitConfig`) -- exponential backoff + token bucket (`crates/easypdf-ocr/src/http/retry.rs`, `crates/easypdf-ocr/src/http/rate_limit.rs`)
- **Image encoding** (`EncodedImage`, `ImageEncoding`) -- Base64 inline or multipart upload (`crates/easypdf-ocr/src/http/image.rs`)
- **Response parsing** (`OcrResponseParser`) -- pluggable response parser trait (`crates/easypdf-ocr/src/http/response.rs`)

## Dependencies

### Internal

| Crate | Purpose |
|-------|---------|
| `easypdf-core` | Core types (`CapabilityLevel`) |
| `easypdf-markdown` | OCR abstraction traits (`OcrEngine`, `OcrImage`, `OcrResult`) |

### External

| Crate | Version | Purpose |
|-------|---------|---------|
| `reqwest` | 0.12 | Synchronous HTTP client (features: json, rustls-tls, multipart, blocking) |
| `serde` / `serde_json` | 1.x | Serialization |
| `hmac` / `sha2` | -- | API signature algorithms |
| `base64` | -- | Image encoding |

## Main API

### Creating OCR Engines

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

// HunyuanOCR
let hunyuan = create_hunyuan_ocr_engine(HunyuanConfig {
    secret_id: "id".into(),
    secret_key: "key".into(),
    ..Default::default()
})?;

// Baidu OCR
let baidu = BaiduOcrEngine::new(BaiduConfig {
    api_key: "ak".into(),
    secret_key: "sk".into(),
    ..Default::default()
})?;
```

### HttpOcrEngine (Generic Base Class)

```rust
use easypdf_ocr::{build_http_engine, HttpClientConfig, build_http_engine_with_config};

// Simple construction
let engine = build_http_engine(request, parser)?;

// With custom config
let engine = build_http_engine_with_config(request, parser, HttpClientConfig {
    max_retries: 3,
    ..Default::default()
})?;
```

### Using an OCR Engine

```rust
use easypdf_markdown::{OcrEngine, OcrImage};

let image = OcrImage::from_path("scan.png")?;
let result = engine.recognize(&image)?;
println!("Text: {}", result.text);
println!("Confidence: {:?}", result.confidence);
```

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-ocr
**docs.rs**: https://docs.rs/easypdf-ocr
