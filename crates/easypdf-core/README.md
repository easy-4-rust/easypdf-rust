# easypdf-core

> easypdf-rust 工作区的基础层：核心 trait、类型定义、IO 安全原语、PDF 加解密与签名。

## 角色

`easypdf-core` 是整个 easypdf-rust 工作区的最底层 crate。它定义了所有上层 crate（reader、writer、markdown 等）共享的核心 trait（`PdfModel`、`PdfReadListener`、`PdfWriteHandler`）、内容类型（文本、表格、图片）、枚举、错误类型，以及 IO 安全守卫和 PDF 加密/签名能力。

## 核心能力

- **核心 trait**（`PdfModel`、`PdfReadListener`、`PdfWriteHandler`、`PdfConverter`、`PdfEngine`）——定义读写与转换的扩展点
- **内容模型**（`PdfText`、`PdfTable`、`PdfImage`、`PdfBlock`）——描述 PDF 页面上的语义内容
- **文档模型**（`PdfDocumentModel`、`PdfPageModel`）——结构化的 PDF 文档表示
- **IO 安全**（`PdfInput`、`ResourceLimits`、`AtomicFileOutput`）——资源限制、SSRF 防护、崩溃安全写入
- **PDF 加密**（AES-128/256）——密码加密与解密，符合 ISO 32000
- **PDF 签名**（PKCS#7 + RSA）——数字签名与验证
- **样式与元数据**（`PdfFont`、`PdfColor`、`PdfMetadata`、`PdfBookmark`）——字体、颜色、书签
- **布局引擎**（`FlowLayout`、`LayoutSink`、`Direction`）——流式布局基础设施

## 依赖

- `lopdf`: PDF 底层对象模型（用于加密/签名操作）
- `thiserror`: 错误类型派生
- `chrono`: 时间处理
- `tempfile`: 安全临时文件
- `ring` / `aes` / `cbc`: 加密原语
- `x509-parser` / `pkcs7` / `der`: 证书与签名格式解析

## 主要 API

### `PdfModel`
```rust
pub trait PdfModel {
    fn render(&self) -> Result<Vec<RenderedElement>>;
    fn metadata(&self) -> PdfModelMetadata;
    fn field_descriptors(&self) -> Vec<PdfFieldDescriptor>;
}
```

### `PdfText`
```rust
let text = PdfText::new("Hello")
    .font(PdfFont::helvetica(12.0))
    .alignment(TextAlignment::Center)
    .color(PdfColor::red());
```

### `PdfTable`
```rust
let table = PdfTable::new(vec!["Name".into(), "Age".into()])
    .row(vec!["Alice".into(), "30".into()])
    .width(400.0);
```

### `PdfInput` / `ResourceLimits`
```rust
let input = PdfInput::from_path("doc.pdf");
let limits = ResourceLimits::default(); // 50 MB, 10000 pages
let bytes = input.read(limits)?;
```

### 加密与签名
```rust
use easypdf_core::crypto::{encrypt_pdf, decrypt_pdf, sign_pdf, verify_pdf_signature};

encrypt_pdf(&input_path, &output_path, "password")?;
decrypt_pdf(&encrypted_path, &output_path, "password")?;
sign_pdf(&input_path, &output_path, &pem_key, &pem_cert)?;
```

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-core
