# easypdf-rust  ·  [English](#easypdf-rust)  |  [中文](#easypdf-rust-中文)

> **An idiomatic Rust library for quick PDF operations.**  
> Inspired by [Alibaba EasyExcel](https://github.com/alibaba/easyexcel)'s builder-pattern API design.  
> **纯 Rust · 零 unsafe · Builder 模式 · 多引擎后端**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance)
[![tests](https://img.shields.io/badge/tests-136%20passed-green.svg)]()
[![coverage](https://img.shields.io/badge/coverage-89%25%20non--derive-blue.svg)]()

---

`easypdf-rust` provides a fluent, type-safe builder API for all common PDF tasks:  
**creation**, **reading & extraction**, **merge / split / rotate**, and **template / form filling**.

📖 **[Usage Guide 使用指南 →](docs/usage-guide.md)**  |  🏗️ **[Architecture 架构 →](docs/architecture.md)**  |  ✅ **[Compatibility 兼容性 →](docs/compatibility.md)**

---

`easypdf-rust` provides a fluent, type-safe builder API for all common PDF tasks:  
**creation**, **reading & extraction**, **merge / split / rotate**, and **template / form filling**.

---

## Table of Contents  |  目录

- [Features 功能](#features--功能)
- [Architecture 架构](#architecture--架构)
- [Quick Start 快速开始](#quick-start--快速开始)
  - [Create PDF 创建 PDF](#1-create-a-pdf--创建-pdf)
  - [Read PDF 读取 PDF](#2-read-a-pdf--读取-pdf)
  - [Merge PDFs 合并 PDF](#3-merge-pdfs--合并-pdf)
  - [Split PDF 拆分 PDF](#4-split-a-pdf--拆分-pdf)
  - [Manipulate Pages 页面操作](#5-manipulate-pages--页面操作)
  - [Fill Form 填充表单](#6-fill-a-form--填充表单)
- [API Reference API 参考](#api-reference--api-参考)
  - [EasyPdf Entry Points 入口方法](#easypdf--entry-points)
  - [PdfCreateBuilder](#pdfcreatebuilder)
  - [PdfReadBuilder](#pdfreadbuilder)
  - [PdfSplitBuilder](#pdfsplitbuilder)
  - [PdfManipulateBuilder](#pdfmanipulatebuilder)
  - [PdfFillBuilder](#pdffillbuilder)
  - [Low-level Types 底层类型](#low-level-types--底层类型)
- [Design Principles 设计原则](#design-principles--设计原则)
- [Roadmap 路线图](#roadmap--路线图)
- [License 许可证](#license--许可证)

---

## Features  |  功能

| Feature 功能 | Status 状态 | Backend 后端 | Description 描述 |
|:---|:---:|:---|:---|
| Create PDF (text, fonts, metadata) | ✅ v0.1 | printpdf | 创建含文本、字体的 PDF |
| Read / extract text + metadata | ✅ v0.1 | lopdf | 读取提取文本、元数据 |
| PDF → Markdown | ✅ v0.1 | lopdf | GFM / LLM / Plain 输出、页范围、转换报告 |
| Merge PDF files | ✅ v0.1 | lopdf | 合并多个 PDF |
| Split PDF into pages | ✅ v0.1 | lopdf | 拆分 PDF 为单页 |
| Rotate pages | ✅ v0.1 | lopdf | 旋转页面 |
| Reorder pages | ✅ v0.1 | lopdf | 重排页面顺序 |
| Fill AcroForm fields | ✅ v0.1 | lopdf | 填充表单字段 |
| `#[derive(PdfModel)]` macro | ✅ v0.1 | — | 编译期反射宏 |
| Page lifecycle handlers | ✅ v0.1 | — | 写入生命周期钩子 |
| Event-driven read listeners | ✅ v0.1 | — | 事件驱动读取回调 |
| Tables | 🚧 v0.2 | printpdf | 表格布局渲染 |
| Images & shapes | 🚧 v0.2 | printpdf | 图片、矢量图形 |
| Custom fonts (TTF/OTF) | 🚧 v0.2 | printpdf | 嵌入自定义字体 |
| Watermarks | 🚧 v0.3 | lopdf | 文本/图片水印 |
| Encryption | 🚧 v0.4 | lopdf | PDF 加密/解密 |
| Digital signatures | 🚧 v0.5 | — | 数字签名 |
| PDF/A compliance | 🚧 v0.5 | — | PDF/A 合规 |
| HTML → PDF | 🚧 v0.6 | — | HTML 转 PDF |

---

## Architecture  |  架构

```mermaid
flowchart TB
    facade["easypdf<br/>EasyPdf facade"]
    markdown["easypdf-markdown<br/>PDF to Markdown pipeline"]
    reader["easypdf-reader<br/>single-parse session"]
    writer["easypdf-writer<br/>printpdf backend"]
    manipulate["easypdf-manipulate<br/>merge/split/edit"]
    template["easypdf-template<br/>AcroForm filling"]
    layout["easypdf-layout<br/>backend-neutral layout"]
    model["easypdf-model<br/>semantic IR"]
    io["easypdf-io<br/>limits + atomic output"]
    core["easypdf-core<br/>types + errors"]

    facade --> markdown & reader & writer & manipulate & template
    markdown --> reader & model & io
    reader --> model & io & core
    writer --> layout & io & core
    manipulate --> io & core
    template --> io & core
    layout --> core
    model --> core
    io --> core
```

| Crate 子包 | Purpose 用途 | Dependencies 依赖 |
|:---|:---|---|
| **easypdf** | Facade + Builder entry points 外观入口 | All sub-crates |
| **easypdf-core** | Types, traits, enums, errors 核心抽象 | thiserror, chrono |
| **easypdf-model** | Engine-neutral semantic IR 引擎无关语义模型 | easypdf-core |
| **easypdf-io** | Resource limits + atomic output 资源限制与原子输出 | easypdf-core, tempfile |
| **easypdf-derive** | `#[derive(PdfModel)]` proc-macro 编译期反射 | syn, quote, proc-macro2 |
| **easypdf-reader** | PDF parsing & extraction PDF 读取提取 | lopdf |
| **easypdf-writer** | PDF creation & writing PDF 创建写入 | printpdf, image |
| **easypdf-layout** | Backend-neutral flow layout 后端无关布局 | easypdf-core |
| **easypdf-manipulate** | Merge / split / rotate / reorder 页面操作 | lopdf |
| **easypdf-template** | Form filling 表单填充 | lopdf |
| **easypdf-markdown** | Deterministic PDF → Markdown 确定性转换 | reader, model, io |

---

## Quick Start  |  快速开始

Add to your `Cargo.toml`:

```toml
[dependencies]
easypdf = "0.1"
```

### 1. Create a PDF  |  创建 PDF

```rust
use easypdf::prelude::*;

// Simple one-liner
EasyPdf::create("hello.pdf")
    .title("My Document")
    .page(PageSize::A4)
    .add_text("Hello, World!")
        .font(PdfFont::helvetica(16.0).bold())
        .position(100.0, 700.0)
    .do_write()?;

// Manual page-by-page construction
let mut writer = EasyPdf::create("multi-page.pdf")
    .metadata(PdfMetadata::new()
        .title("Report")
        .author("Alice"))
    .build()?;

writer.add_page(PageSize::A4, Orientation::Portrait)?;
writer.write_text(
    &PdfText::new("Chapter 1").font(PdfFont::helvetica(18.0).bold()),
    72.0, 750.0,
)?;
writer.finish("multi-page.pdf")?;
```

### 2. Read a PDF  |  读取 PDF

```rust
// Extract all text
let text = EasyPdf::read("input.pdf").extract_text()?;
println!("{text}");

// Extract metadata
let meta = EasyPdf::read("input.pdf").metadata()?;
println!("Title: {:?}, Author: {:?}", meta.title, meta.author);

// Get page count
let count = EasyPdf::read("input.pdf").page_count()?;

// Limit to specific pages (0-based)
let pages_1_to_3 = EasyPdf::read("input.pdf")
    .pages(0..3)
    .extract_text()?;

// Event-driven reading
struct MyListener { texts: Vec<String> }
impl PdfReadListener for MyListener {
    fn on_text(&mut self, _page: usize, text: &str) -> easypdf::Result<()> {
        self.texts.push(text.to_string());
        Ok(())
    }
}

let mut listener = MyListener { texts: vec![] };
PdfReader::open("input.pdf")?.read_with_listener(&mut listener)?;
```

### PDF to Markdown  |  PDF 转 Markdown

```rust
use easypdf::prelude::*;

let result = EasyPdf::export_markdown("input.pdf", "output.md")
    .pages(0..20) // zero-based, end-exclusive
    .profile(MarkdownProfile::Llm)
    .tables(TablePolicy::Detect)
    .images(ImagePolicy::Reference)
    .ocr(OcrPolicy::Auto)
    .do_export()?;

println!("converted {} pages", result.report().pages_read());
for warning in result.report().warnings() {
    eprintln!("warning: {warning:?}");
}
```

The current backend extracts native PDF text into semantic paragraph blocks.
Table detection and OCR requests are reported as structured warnings until their optional backends are enabled; no capability is silently simulated.

当前后端会把 PDF 原生文本提取为语义段落。表格检测与 OCR 在可选后端尚未启用时会返回结构化警告，不会伪造成功结果。

Reader performance can be measured with the checked-in single-parse regression benchmark:

```bash
cargo bench -p easypdf-reader --bench reader_session
```

### 3. Merge PDFs  |  合并 PDF

```rust
EasyPdf::merge(
    &["cover.pdf", "chapter1.pdf", "chapter2.pdf", "appendix.pdf"],
    "book.pdf",
)?;
```

### 4. Split a PDF  |  拆分 PDF

```rust
// Split into individual pages
let files = EasyPdf::split("big.pdf")
    .every_n_pages(1)
    .save_to_dir("./pages/")?;
// Produces: ./pages/page_001.pdf, ./pages/page_002.pdf, ...

// Split every 5 pages
EasyPdf::split("big.pdf")
    .every_n_pages(5)
    .save_to_dir("./chunks/")?;
```

### 5. Manipulate Pages  |  页面操作

```rust
// Rotate all pages + reorder
EasyPdf::manipulate("input.pdf")
    .rotate_all(Rotation::Clockwise90)
    .reorder_pages(&[2, 0, 1])  // 0-based: moves p3 to front
    .save("rotated.pdf")?;

// Rotate specific pages only
EasyPdf::manipulate("input.pdf")
    .rotate_page(1, Rotation::Clockwise90)   // page 1 only
    .rotate_page(3, Rotation::Clockwise180)  // page 3 only
    .save("output.pdf")?;
```

### 6. Fill a Form  |  填充表单

```rust
#[derive(PdfModel)]
struct MyForm {
    #[pdf(field = "customer_name")]
    customer_name: String,
    #[pdf(field = "total_amount")]
    total_amount: String,
}

EasyPdf::fill_form("template.pdf", &MyForm {
    customer_name: "Alice".into(),
    total_amount: "$1,234.00".into(),
})
    .save("invoice-filled.pdf")?;

// Programmatic field filling without derive
EasyPdf::fill_form("template.pdf", &MyForm::default())
    .field("customer_name", "Bob")
    .field("total_amount", "$567.00")
    .save("filled.pdf")?;
```

---

## API Reference  |  API 参考

### EasyPdf  ·  Entry Points

| Method 方法 | Signature 签名 | Returns 返回 |
|:---|---|:---|
| `create` | `(path: impl Into<PathBuf>)` | `PdfCreateBuilder` |
| `read` | `(path: impl Into<PathBuf>)` | `PdfReadBuilder` |
| `export_markdown` | `(input, output)` | `PdfMarkdownExportBuilder` |
| `merge` | `(inputs: &[impl AsRef<Path>], output: impl AsRef<Path>)` | `Result<()>` |
| `split` | `(path: impl Into<PathBuf>)` | `PdfSplitBuilder` |
| `manipulate` | `(path: impl Into<PathBuf>)` | `PdfManipulateBuilder` |
| `fill_form` | `(template: impl Into<PathBuf>, data: &dyn PdfModel)` | `PdfFillBuilder` |

---

### PdfCreateBuilder

| Method | Signature | Description |
|:---|---|:---|
| `title` | `(title: impl Into<String>) -> Self` | Set document title |
| `page_size` | `(size: PageSize) -> Self` | Set default page size |
| `orientation` | `(orientation: Orientation) -> Self` | Set page orientation |
| `metadata` | `(metadata: PdfMetadata) -> Self` | Set document metadata |
| `register_handler` | `(handler: Box<dyn PdfWriteHandler>) -> Self` | Register lifecycle handler |
| `add_text` | `(content: impl Into<String>) -> PdfTextBuilder<Self>` | Add text element |
| `build` | `() -> Result<PdfWriter>` | Build for manual use |
| `do_write` | `() -> Result<PathBuf>` | Build + add page + save |

**PdfTextBuilder<PdfCreateBuilder>** (returned by `add_text`):

| Method | Signature | Description |
|:---|---|:---|
| `font` | `(font: PdfFont) -> Self` | Set text font |
| `position` | `(x: f64, y: f64) -> PdfPositionedTextBuilder` | Set position in points |
| `do_write` | `() -> Result<PathBuf>` | Write at default (100, 700) |

**PdfPositionedTextBuilder** (returned by `position`):

| Method | Signature | Description |
|:---|---|:---|
| `do_write` | `() -> Result<PathBuf>` | Write at the set position |

---

### PdfReadBuilder

| Method | Signature | Returns |
|:---|---|:---|
| `pages` | `(range: Range<usize>) -> Self` | Limit to page range (0-based) |
| `extract_text` | `() -> Result<String>` | Extract all text, pages joined by `\n` |
| `metadata` | `() -> Result<PdfMetadata>` | Extract title, author |
| `page_count` | `() -> Result<usize>` | Total pages in document |

---

### PdfSplitBuilder

| Method | Signature | Returns |
|:---|---|:---|
| `every_n_pages` | `(n: usize) -> Self` | Pages per split file (default: 1) |
| `save_to_dir` | `(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>>` | Split and save, returns output paths |

---

### PdfManipulateBuilder

| Method | Signature | Returns |
|:---|---|:---|
| `rotate_page` | `(page: usize, rotation: Rotation) -> Self` | Rotate one page (1-based) |
| `rotate_all` | `(rotation: Rotation) -> Self` | Rotate every page |
| `rotate` | `(rotation: Rotation) -> Self` | Alias for `rotate_all` |
| `reorder_pages` | `(order: &[usize]) -> Self` | Permute pages (0-based) |
| `save` | `(output: impl AsRef<Path>) -> Result<()>` | Apply + save |

---

### PdfFillBuilder

| Method | Signature | Returns |
|:---|---|:---|
| `field` | `(name: impl Into<String>, value: impl Into<String>) -> Self` | Add one field |
| `fields` | `(fields: impl IntoIterator<Item = (K, V)>) -> Self` | Add many fields |
| `save` | `(output: impl AsRef<Path>) -> Result<()>` | Fill + save |

---

### Low-level Types  |  底层类型

<details>
<summary>Click to expand 点击展开</summary>

#### Page & Layout 页面与布局

| Type | Description |
|:---|:---|
| `PageSize` | `A0`–`A5`, `Letter`, `Legal`, `Custom(w, h)` in points |
| `Orientation` | `Portrait` (default), `Landscape` |
| `Rotation` | `None`, `Clockwise90`, `Clockwise180`, `Clockwise270` |

#### Colors 颜色

| Type / Method | Description |
|:---|:---|
| `PdfColor::Rgb(r, g, b)` | RGB, range 0.0–1.0 |
| `PdfColor::Gray(v)` | Grayscale |
| `PdfColor::Cmyk(c, m, y, k)` | CMYK |
| `PdfColor::rgb_u8(r, g, b)` | RGB from 0–255 ints |
| `PdfColor::black()` / `white()` / `red()` / `green()` / `blue()` | Predefined colors |

#### Fonts 字体

| Type | Description |
|:---|:---|
| `FontFamily::BuiltIn(BuiltInFont)` | One of 14 standard PDF fonts |
| `FontFamily::Custom(path)` | TTF/OTF file path |
| `PdfFont` | Family + size + style (bold, italic) |
| `PdfFont::helvetica(size)` / `times_roman(size)` / `courier(size)` | Convenience constructors |
| `BuiltInFont` | `Helvetica`, `TimesRoman`, `Courier`, `Symbol`, `ZapfDingbats` (+ bold/italic variants) |

#### Content 内容

| Type | Description |
|:---|:---|
| `PdfText` | Content string + font + color + alignment |
| `PdfTable` | Headers + rows + column widths |
| `PdfImage` | Raw bytes + format (JPEG/PNG) + dimensions |
| `PdfLine` | (x1,y1)→(x2,y2) + width + color |
| `PdfRect` | Position + dimensions + border + fill |

#### Traits 扩展点

| Trait | Role | Analogous to |
|:---|:---|:---|
| `PdfModel` | Map struct → PDF elements (derive) | `ExcelRow` in easyexcel-rs |
| `PdfReadListener` | Event-driven PDF reading | `ReadListener<T>` |
| `PdfWriteHandler` | Lifecycle hooks: before/after page | `WriteHandler` |
| `PdfConverter<T>` | Bidirectional Rust ⇄ PDF string | `Converter<T>` |

#### Errors 错误

| Variant | Description |
|:---|:---|
| `PdfError::Io(e)` | Wraps `std::io::Error` |
| `PdfError::Parse(msg)` | Malformed PDF or invalid content |
| `PdfError::InvalidPage(n)` | Page index out of bounds |
| `PdfError::UnsupportedFeature(msg)` | Feature not yet implemented |
| `PdfError::Encryption(msg)` | Encryption-related error |
| `PdfError::Other(msg)` | Catch-all |

```rust
pub type Result<T, E = PdfError> = std::result::Result<T, E>;
```

</details>

---

## Design Principles  |  设计原则

| Principle 原则 | Practice 实践 |
|:---|:---|
| **Pure Rust** | `#![forbid(unsafe_code)]` in every crate |
| **Type-safe builders** | `mut self → Self`, `#[must_use]` on all builders |
| **Multi-engine** | lopdf for parse/manipulate, printpdf for create — swappable backends |
| **Trait extensibility** | `PdfReadListener`, `PdfWriteHandler`, `PdfConverter<T>` for custom logic |
| **Compile-time reflection** | `#[derive(PdfModel)]` generates mapping code — no runtime reflection |
| **Error transparency** | Single `PdfError` enum with `thiserror`, single `Result<T>` alias |
| **Zero-cost abstractions** | Builder chains compile to direct calls, derive macros expand at compile time |
| **Inspired by Alibaba EasyExcel** | Same builder · listener · handler · converter patterns |

---

## Roadmap  |  路线图

| Phase | Focus | Key Deliverables |
|:---:|:---|:---|
| **v0.1** ✅ | Foundation | Workspace, all 7 crates, core types, read/write/manipulate/template, derive macro, builder API |
| **v0.2** 🚧 | Rich content | Tables, images, vector shapes, custom TTF/OTF fonts, page headers/footers, multi-page writer |
| **v0.3** | Watermarks & layers | Text/image watermarks, PDF layers (OCG), background/foreground overlay |
| **v0.4** | Security | AES-256 encryption/decryption, password protection, permission flags |
| **v0.5** | Compliance | PDF/A-1/2/3 validation, digital signatures, XMP metadata |
| **v0.6** | Converters | HTML → PDF, Markdown → PDF, SVG → PDF |
| **v1.0** | Stable | Stable API, full test coverage, performance benchmarks, documentation |

---

## License  |  许可证

Apache-2.0

---

## Documentation  |  文档

| Document | 说明 | Description |
|:---|:---|:---|
| [Usage Guide](docs/usage-guide.md) | 完整 API 使用指南，含 12 章节代码示例 | Complete usage guide with 12 chapters of examples |
| [Architecture](docs/architecture.md) | 架构设计文档，含数据流、类型层级、设计模式 | Architecture design with data flows, type hierarchy, patterns |
| [Compatibility](docs/compatibility.md) | 功能兼容性矩阵 + 覆盖率报告 | Feature matrix + coverage report |
| [Implementation Plan](docs/implementation-plan.md) | 未实现功能的实施规划 | Implementation plan for planned features |

## Related Projects  |  相关项目

- [easyexcel-rs](https://github.com/easypdf-rust/easyexcel-rs) — Rust port of Alibaba EasyExcel  
- [easyexcel](https://github.com/alibaba/easyexcel) — Original Java library by Alibaba  
- [lopdf](https://crates.io/crates/lopdf) — Pure Rust PDF manipulation library  
- [printpdf](https://crates.io/crates/printpdf) — Pure Rust PDF generation library

---

<p align="center">
  <sub>Built with Rust 🦀 · Follows <a href="https://github.com/easypdf-rust/easyexcel-rs">easyexcel-rs</a> conventions</sub>
</p>
