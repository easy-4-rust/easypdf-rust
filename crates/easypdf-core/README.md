# easypdf-core

> Foundation layer for easypdf-rust: core traits, types, IO safety primitives, PDF crypto (encrypt/sign), and Markdown IR.

## Role

`easypdf-core` is the lowest-level crate in the easypdf-rust workspace. It defines the shared abstractions that every upper-layer crate (reader, writer, markdown, ocr, runtime) depends on: core traits, content types, enums, error types, IO guards, encryption, digital signatures, and the semantic document model (`PdfBlock` / `PdfDocumentModel`).

## Core Capabilities

- **Core traits** (`PdfModel`, `PdfReadListener`, `PdfWriteHandler`, `PdfConverter`, `PdfEngine`) -- extension points for reading, writing, and conversion (`crates/easypdf-core/src/traits.rs`)
- **Content model** (`PdfText`, `PdfTable`, `PdfImage`, `PdfLine`, `PdfRect`) -- semantic content elements (`crates/easypdf-core/src/content.rs`)
- **Document IR** (`PdfDocumentModel`, `PdfPageModel`, `PdfBlock` with 14 variants) -- structured intermediate representation (`crates/easypdf-core/src/model/`)
- **IO safety** (`PdfInput`, `ResourceLimits`, `AtomicFileOutput`, `guard_decompression_bomb`, `guard_element_explosion`) -- resource limits, SSRF protection, crash-safe writes (`crates/easypdf-core/src/io/`)
- **PDF encryption** (AES-128/256, ISO 32000) -- `encrypt_pdf()` / `decrypt_pdf()` with 8 permission flags (`crates/easypdf-core/src/crypto/encrypt.rs`)
- **PDF signing** (PKCS#7/CMS, RSA-PKCS#1v1.5 + SHA-256 via `ring`) -- `sign_pdf()` / `verify_pdf_signature()` (`crates/easypdf-core/src/crypto/sign.rs`)
- **Style & metadata** (`PdfFont`, `PdfColor`, `BuiltInFont`, `TableStyle`, `PdfMetadata`, `PdfBookmark`) -- (`crates/easypdf-core/src/style.rs`, `crates/easypdf-core/src/metadata.rs`)
- **Layout engine** (`FlowLayout`, `LayoutSink`, `Direction`) -- flow-based layout infrastructure (`crates/easypdf-core/src/layout/`)

## Dependencies

### Internal

None -- this is the foundation crate.

### External

| Crate | Version | Purpose |
|-------|---------|---------|
| `lopdf` | 0.44.0 | PDF object model (encrypt/decrypt via lopdf API) |
| `ring` | 0.17 | Constant-time RSA operations for signing |
| `aes` / `cbc` / `cipher` | -- | AES encryption primitives |
| `x509-parser` | 0.16 | X.509 certificate parsing |
| `bitflags` | 2 | `PdfPermissions` bitflags |
| `thiserror` | 2.0.18 | Error type derivation |
| `chrono` | 0.4.45 | Date/time handling |
| `serde` / `serde_json` | 1.x | Serialization |

## Main API

### Enums

```rust
// Page & layout
pub enum PageSize { A0, A1, A2, A3, A4, A5, Letter, Legal, Custom(f64, f64) }
pub enum Orientation { Portrait, Landscape }
pub enum Rotation { None, Clockwise90, Clockwise180, Clockwise270 }
pub enum TextAlignment { Left, Center, Right, Justify }
pub enum VerticalAlignment { Top, Middle, Bottom }
pub enum ImageFormat { Jpeg, Png }

// Content block (#[non_exhaustive], 14 variants)
pub enum PdfBlock {
    Heading { level: u8, text: String, source: SourceLocation },
    Paragraph { text: String, source: SourceLocation },
    List { ordered: bool, items: Vec<ListItem>, source: SourceLocation },
    Table { headers: Vec<String>, rows: Vec<Vec<String>>, source: SourceLocation },
    Image { data: ImageData, source: SourceLocation },
    Code { language: Option<String>, text: String, source: SourceLocation },
    // ... 8 more variants
}

// Crypto
pub enum PdfEncryptionAlgorithm { Aes128, Aes256 }
```

### Traits

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

### Content Builders

```rust
let text = PdfText::new("Hello")
    .font(PdfFont::helvetica(12.0))
    .alignment(TextAlignment::Center)
    .color(PdfColor::red());

let table = PdfTable::new(vec!["Name".into(), "Age".into()])
    .row(vec!["Alice".into(), "30".into()])
    .width(400.0);
```

### Encryption & Signing

```rust
use easypdf_core::crypto::{encrypt_pdf, decrypt_pdf};

let enc = PdfEncryption::new("user", "owner")
    .with_algorithm(PdfEncryptionAlgorithm::Aes256)
    .with_permissions(PdfPermissions::PRINT | PdfPermissions::COPY);
let encrypted = encrypt_pdf(&pdf_bytes, &enc)?;

// Signing
use easypdf_core::crypto::{sign_pdf, verify_pdf_signature};
sign_pdf(&pdf_bytes, &signer)?;
let info = verify_pdf_signature(&pdf_bytes)?;
```

### IO Safety

```rust
let input = PdfInput::from_path("doc.pdf");
let limits = ResourceLimits::default(); // 50 MB, 10000 pages
let bytes = input.read(limits)?;

// Atomic writes
let out = AtomicFileOutput::new("output.pdf");
out.write_all(&pdf_bytes)?; // writes to temp, renames on success
```

## Known Limitations

- RFC 3161 timestamp server support: fields reserved but **not yet implemented** (`crates/easypdf-core/src/crypto/sign.rs:69`)
- `PdfEngine` trait defined but **no concrete implementation** yet (`crates/easypdf-core/src/traits.rs:260-263`)
- `unsafe_code = "forbid"` enforced workspace-wide

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-core
**docs.rs**: https://docs.rs/easypdf-core
