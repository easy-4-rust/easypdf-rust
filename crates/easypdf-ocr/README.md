# easypdf-ocr

> 云 OCR 引擎层：集成 GLM、混元 OCR、百度 OCR 三大云端引擎，统一 HTTP 基类。

## 角色

`easypdf-ocr` 为 easypdf-rust 提供云端 OCR 引擎集成。它封装了三个主流中文 OCR 云服务（智谱 GLM、腾讯混元 OCR、百度 OCR），并提供通用的 HTTP 基类 `HttpOcrEngine`，统一处理认证、重试、限流、图片编码等基础设施，使各引擎只需关注请求构建与响应解析。

## 核心能力

- **GLM OCR**（智谱 AI）——支持通用文字识别，Bearer Token 认证
- **混元 OCR**（腾讯）——支持通用/表格/手写等多种模式，HMAC-SHA256 签名认证
- **百度 OCR**——支持通用文字识别，AK/SK 认证 + Token 管理
- **HTTP 基类**（`HttpOcrEngine`）——通用的 HTTP OCR 引擎框架，实现 `OcrEngine` trait
- **认证抽象**（`AuthMethod`）——Bearer / AK-SK / HMAC / 无认证
- **重试与限流**（`BackoffStrategy`、`RateLimitConfig`）——指数退避 + 令牌桶限流
- **图片编码**（`EncodedImage`）——Base64 内联 / multipart 上传

## 依赖

- `easypdf-core`: 核心类型（`CapabilityLevel`）
- `easypdf-markdown`: OCR 抽象 trait（`OcrEngine`、`OcrImage`、`OcrResult`）
- `reqwest`（同步，rustls）: HTTP 客户端
- `serde` / `serde_json`: 序列化
- `hmac` / `sha2`: 签名算法
- `base64`: 图片编码

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

### `HttpOcrEngine`（通用基类）
```rust
use easypdf_ocr::{build_http_engine, HttpClientConfig};

let engine = build_http_engine(request, parser)?;
// 或带自定义配置
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
