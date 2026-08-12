# easypdf-core

> easypdf-rust 工作区的基础层：核心 trait、类型定义、IO 安全原语、PDF 加解密与签名、Markdown IR。

## 角色

`easypdf-core` 是整个 easypdf-rust 工作区的最底层 crate。它定义了所有上层 crate（reader、writer、markdown、ocr、runtime）共享的核心 trait、内容类型、枚举、错误类型、IO 安全守卫、PDF 加密/签名能力，以及语义文档模型（`PdfBlock` / `PdfDocumentModel`）。

## 核心能力

- **核心 trait**（`PdfModel`、`PdfReadListener`、`PdfWriteHandler`、`PdfConverter`、`PdfEngine`）——定义读写与转换的扩展点（`crates/easypdf-core/src/traits.rs`）
- **内容模型**（`PdfText`、`PdfTable`、`PdfImage`、`PdfLine`、`PdfRect`）——描述 PDF 页面上的语义内容（`crates/easypdf-core/src/content.rs`）
- **文档 IR**（`PdfDocumentModel`、`PdfPageModel`、`PdfBlock` 14 种变体）——结构化中间表示（`crates/easypdf-core/src/model/`）
- **IO 安全**（`PdfInput`、`ResourceLimits`、`AtomicFileOutput`、`guard_decompression_bomb`、`guard_element_explosion`）——资源限制、SSRF 防护、崩溃安全写入（`crates/easypdf-core/src/io/`）
- **PDF 加密**（AES-128/256，符合 ISO 32000）——`encrypt_pdf()` / `decrypt_pdf()`，8 种权限标志（`crates/easypdf-core/src/crypto/encrypt.rs`）
- **PDF 签名**（PKCS#7/CMS，RSA-PKCS#1v1.5 + SHA-256，via `ring`）——`sign_pdf()` / `verify_pdf_signature()`（`crates/easypdf-core/src/crypto/sign.rs`）
- **样式与元数据**（`PdfFont`、`PdfColor`、`BuiltInFont`、`TableStyle`、`PdfMetadata`、`PdfBookmark`）——（`crates/easypdf-core/src/style.rs`、`crates/easypdf-core/src/metadata.rs`）
- **布局引擎**（`FlowLayout`、`LayoutSink`、`Direction`）——流式布局基础设施（`crates/easypdf-core/src/layout/`）

## 依赖

### 内部依赖

无——这是基础 crate。

### 外部依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `lopdf` | 0.44.0 | PDF 对象模型（加密/解密通过 lopdf API） |
| `ring` | 0.17 | 常量时间 RSA 操作（签名模块） |
| `aes` / `cbc` / `cipher` | -- | AES 加密原语 |
| `x509-parser` | 0.16 | X.509 证书解析 |
| `bitflags` | 2 | `PdfPermissions` 位标志 |
| `thiserror` | 2.0.18 | 错误类型派生 |
| `chrono` | 0.4.45 | 日期/时间处理 |
| `serde` / `serde_json` | 1.x | 序列化 |

## 主要 API

### 枚举

```rust
// 页面与布局
pub enum PageSize { A0, A1, A2, A3, A4, A5, Letter, Legal, Custom(f64, f64) }
pub enum Orientation { Portrait, Landscape }
pub enum Rotation { None, Clockwise90, Clockwise180, Clockwise270 }
pub enum TextAlignment { Left, Center, Right, Justify }
pub enum VerticalAlignment { Top, Middle, Bottom }
pub enum ImageFormat { Jpeg, Png }

// 内容块（#[non_exhaustive]，14 种变体）
pub enum PdfBlock {
    Heading { level: u8, text: String, source: SourceLocation },
    Paragraph { text: String, source: SourceLocation },
    List { ordered: bool, items: Vec<ListItem>, source: SourceLocation },
    Table { headers: Vec<String>, rows: Vec<Vec<String>>, source: SourceLocation },
    Image { data: ImageData, source: SourceLocation },
    Code { language: Option<String>, text: String, source: SourceLocation },
    // ... 另有 8 种变体
}

// 加密
pub enum PdfEncryptionAlgorithm { Aes128, Aes256 }
```

### Trait

```rust
pub trait PdfModel {
    fn render(&self) -> Result<Vec<RenderedElement>>;
    fn metadata(&self) -> PdfModelMetadata;
    fn field_descriptors(&self) -> Vec<PdfFieldDescriptor>;
}

pub trait PdfReadListener: Send {
    fn on_page_start(&mut self, page: PageNumber) -> Result<()>;
    fn on_text(&mut self, page: PageNumber, text: &str) -> Result<()>;
    fn on_page_end(&mut self, page: PageNumber) -> Result<()>;
    fn on_document_end(&mut self) -> Result<()>;
}

pub trait PdfWriteHandler: Send {
    fn before_document(&mut self) -> Result<()> { Ok(()) }
    fn before_page(&mut self, page: PageNumber) -> Result<()> { Ok(()) }
    fn after_page(&mut self, page: PageNumber) -> Result<()> { Ok(()) }
    fn after_document(&mut self) -> Result<()> { Ok(()) }
}

pub trait PdfEngine: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> EngineCapabilities;
}
```

### 内容构建器

```rust
let text = PdfText::new("你好")
    .font(PdfFont::helvetica(12.0))
    .alignment(TextAlignment::Center)
    .color(PdfColor::red());

let table = PdfTable::new(vec!["姓名".into(), "年龄".into()])
    .row(vec!["张三".into(), "30".into()])
    .width(400.0);
```

### 加密与签名

```rust
use easypdf_core::crypto::{encrypt_pdf, decrypt_pdf};

let enc = PdfEncryption::new("user", "owner")
    .with_algorithm(PdfEncryptionAlgorithm::Aes256)
    .with_permissions(PdfPermissions::PRINT | PdfPermissions::COPY);
let encrypted = encrypt_pdf(&pdf_bytes, &enc)?;

// 签名
use easypdf_core::crypto::{sign_pdf, verify_pdf_signature};
sign_pdf(&pdf_bytes, &signer)?;
let info = verify_pdf_signature(&pdf_bytes)?;
```

### IO 安全

```rust
let input = PdfInput::from_path("doc.pdf");
let limits = ResourceLimits::default(); // 50 MB, 10000 页
let bytes = input.read(limits)?;

// 原子写入
let out = AtomicFileOutput::new("output.pdf");
out.write_all(&pdf_bytes)?; // 写入临时文件，成功后重命名
```

## 已知限制

- RFC 3161 时间戳服务器：字段已预留但**尚未实现**（`crates/easypdf-core/src/crypto/sign.rs:69`）
- `PdfEngine` trait 已定义但**无具体实现**——等待第二个成熟引擎（`crates/easypdf-core/src/traits.rs:260-263`）
- `unsafe_code = "forbid"` 全工作区禁止 unsafe

## License

Apache-2.0

---

**项目主页**：https://github.com/easy-4-rust/easypdf-rust
**crates.io**：https://crates.io/crates/easypdf-core
**docs.rs**：https://docs.rs/easypdf-core
