<a id="readme-top"></a>

<div align="center">

# easypdf-rust

**An idiomatic Rust library for quick PDF operations.**
Inspired by [Alibaba EasyExcel](https://github.com/alibaba/easyexcel)'s builder-pattern API design.

[![Crates.io](https://img.shields.io/crates/v/easypdf)](https://crates.io/crates/easypdf)
[![docs.rs](https://img.shields.io/docsrs/easypdf)](https://docs.rs/easypdf)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#3-rust-baseline--platform-support)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance)
[![tests](https://img.shields.io/badge/tests-136%20passed-green.svg)]()

[English](./README.md) · [简体中文](./README.zh-CN.md)

[定位](#1-project-positioning--status) · [功能](#2-features--maturity) · [架构](#5-workspace--crate-architecture) ·
[快速开始](#7-quick-start) · [Markdown](#8-pdf--markdown-export) · [Features](#6-cargo-features) ·
[质量](#14-build-test--quality-gates) · [路线图](#12-roadmap) · [贡献](#16-contributing--license)

</div>

---

> **Current Version**: `0.1.0`
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Workspace Resolver**: `3`
> **Maturity**: Experimental (API and behavior may change)
> **License**: Apache-2.0
> **Last Verified**: 2026-08-09

## 1. Project Positioning & Status

### 1.1 What it is

**easypdf-rust is an idiomatic Rust workspace for quick PDF operations — creation, reading, manipulation, template filling, and PDF → Markdown conversion.**

| Dimension | Detail |
|---|---|
| Root crate | `easypdf` |
| Current version | `0.1.0` |
| MSRV / Edition | `1.88` / `2024` |
| Default features | `markdown` |
| unsafe policy | `#![forbid(unsafe_code)]` in every crate |
| Release status | Workspace-only (not yet on crates.io) |
| License | `Apache-2.0` |

### 1.2 What it is NOT

- Not a 1:1 port of Java EasyExcel — PDF and Excel are fundamentally different paradigms.
- Does not claim implemented features that only return `UnsupportedFeature` or produce stub output.
- Does not enable encryption or signing — these return explicit errors, not simulated success.
- Does not perform real OCR, table detection, or image extraction — the markdown pipeline emits structured warnings for these gaps.

### 1.3 Status Evidence

| Claim | Value | Evidence |
|---|---|---|
| Workspace builds | ✅ | `cargo check -p easypdf --all-features` |
| Tests pass | 136 passed, 1 ignored (legacy trybuild) | `cargo test --workspace --quiet` |
| New crates pass clippy | ✅ | `clippy -D warnings` on easypdf-model, easypdf-io, easypdf-markdown |
| No-default-features builds | ✅ | `cargo check -p easypdf --no-default-features` |
| Docs build | ✅ | `cargo doc --workspace --no-deps` |
| Reader session reuse | ~129x faster than re-open | `cargo bench -p easypdf-reader --bench reader_session` |
| crates.io | Not published | Workspace-only manifest |

## 2. Features & Maturity

### 2.1 Feature Matrix

| Feature | Status | Crate / Feature | Limitations | Verification |
|---|:---:|---|---|---|
| Create PDF (text, fonts, metadata) | ✅ Stable | `easypdf-writer` | Built-in fonts only by default | Tests + examples |
| Read / extract text + metadata | ✅ Stable | `easypdf-reader` | Text extraction depends on font encoding | Tests + benchmark |
| PDF → Markdown | ✅ Preview | `easypdf-markdown` / `markdown` | Native text MVP; tables/images/OCR emit warnings | 6 profile tests |
| Merge PDFs | ✅ Stable | `easypdf-manipulate` | Valid `/Pages` tree on output | Merge tests |
| Split PDF | ✅ Stable | `easypdf-manipulate` | Valid `/Pages` tree per output | Split tests |
| Rotate / reorder pages | ✅ Stable | `easypdf-manipulate` | Per-page or all-pages | Manipulate tests |
| Fill AcroForm fields | ✅ Stable | `easypdf-template` | Field name matching | Template tests |
| `#[derive(PdfModel)]` | ✅ Stable | `easypdf-derive` | Compile-time only | trybuild tests |
| Writer lifecycle hooks | ✅ Stable | `easypdf-writer` | `PdfWriteHandler` trait | Handler lifecycle test |
| Event-driven read listeners | ✅ Stable | `easypdf-reader` | `PdfReadListener` trait | Listener test |
| Atomic output | ✅ Stable | `easypdf-io` | Temp file + atomic rename | All save operations |
| Resource limits | ✅ Stable | `easypdf-io` | File size, page count, text length | Limit exceeded tests |
| Engine-neutral semantic model | ✅ Preview | `easypdf-model` | `PdfBlock` / `PdfPageModel` / `PdfDocumentModel` | Markdown pipeline |
| Tables, images, shapes | 🚧 Planned | — | Not yet implemented | v0.2 roadmap |
| Custom TTF/OTF fonts | 🚧 Planned | — | Partial: `register_font_from_path` exists | v0.2 roadmap |
| Encryption | ⛔ Not implemented | — | Returns `UnsupportedFeature` | Explicit error test |
| Digital signatures | ⛔ Not implemented | — | Returns `UnsupportedFeature` | Explicit error test |

### 2.2 Status Definitions

| Status | Definition |
|---|---|
| ✅ Stable | Public API, tests, and documentation complete; behavior verified |
| 🧪 Preview | Usable but API or behavior may change |
| 🚧 Partial | Only explicitly listed subsets are available |
| 🗓️ Planned | No callable implementation yet |
| ⛔ Not implemented | Returns explicit error; will not silently simulate success |

## 3. Rust Baseline & Platform Support

### 3.1 Toolchain

| Item | Value | Source |
|---|---|---|
| MSRV | `1.88` | `workspace.package.rust-version` |
| Edition | `2024` | `workspace.package.edition` |
| Resolver | `3` | `workspace.resolver` |
| unsafe | `forbid` | `workspace.lints.rust.unsafe_code` |

### 3.2 Platform

| Platform | Status | Notes |
|---|---|---|
| Linux (x86_64) | ✅ | Primary CI target |
| macOS (ARM64 / x86_64) | ✅ | Development platform |
| Windows | Expected | No blocking platform-specific code |
| WASM | Not tested | lopdf/printpdf may have constraints |

## 4. Document Processing Pipeline

```text
Input PDF / bytes / path
        │
        ▼
Resource limit check (file size, page count)
        │
        ▼
lopdf parse → PdfReader session (single-parse, reusable)
        │
        ├──► extract_text / extract_metadata
        ├──► PdfDocumentModel (engine-neutral IR)
        │         │
        │         ▼
        │    MarkdownRenderer (GFM / LLM / Plain profiles)
        │         │
        │         ├──► Output .md file (atomic write)
        │         └──► MarkdownExportReport + structured warnings
        │
        ├──► Merge / Split / Rotate / Reorder
        │         │
        │         └──► Atomic output (temp + rename)
        │
        └──► Template fill (AcroForm fields)
                  │
                  └──► Atomic output
```

## 5. Workspace & Crate Architecture

### 5.1 Crate Map

```mermaid
flowchart TB
    facade["easypdf\nEasyPdf facade"]
    markdown["easypdf-markdown\nPDF → Markdown pipeline"]
    reader["easypdf-reader\nsingle-parse session"]
    writer["easypdf-writer\nprintpdf backend"]
    manipulate["easypdf-manipulate\nmerge/split/edit"]
    template["easypdf-template\nAcroForm filling"]
    layout["easypdf-layout\nbackend-neutral layout"]
    model["easypdf-model\nsemantic IR"]
    io["easypdf-io\nlimits + atomic output"]
    core["easypdf-core\ntypes + errors"]
    derive["easypdf-derive\nproc-macro"]

    facade --> markdown & reader & writer & manipulate & template
    markdown --> reader & model & io
    reader --> model & io & core
    writer --> layout & io & core
    manipulate --> io & core
    template --> io & core
    layout --> core
    model --> core
    io --> core
    derive --> core
```

### 5.2 Crate Responsibilities

| Crate | Purpose | Backend |
|---|---|---|
| **easypdf** | Facade + `EasyPdf` entry point + all Builder types | Depends on all sub-crates |
| **easypdf-core** | Types, enums, traits, `PdfError` | thiserror, chrono (no engine) |
| **easypdf-model** | Engine-neutral semantic IR (`PdfBlock`, `PdfPageModel`, `PdfDocumentModel`) | No engine dependency |
| **easypdf-io** | `ResourceLimits`, `PdfInput`, `AtomicFileOutput` | std only |
| **easypdf-derive** | `#[derive(PdfModel)]` proc-macro | syn, quote |
| **easypdf-layout** | Backend-neutral layout abstractions (`LayoutSink` trait, `FlowLayout`) | No engine dependency |
| **easypdf-reader** | PDF parsing, text extraction, metadata, session reuse | lopdf |
| **easypdf-writer** | PDF creation with text, images, shapes, fonts | printpdf |
| **easypdf-manipulate** | Merge, split, rotate, reorder pages | lopdf |
| **easypdf-template** | AcroForm field filling | lopdf |
| **easypdf-markdown** | PDF → Markdown conversion with profiles and structured warnings | lopdf + easypdf-model |

### 5.3 Dependency Rules

- `easypdf-core` has zero engine dependencies — it is the shared vocabulary.
- `easypdf-model` and `easypdf-io` have zero engine dependencies — they are engine-neutral infrastructure.
- `easypdf-layout` does NOT depend on `easypdf-writer` — it exposes `LayoutSink` which Writer implements.
- Domain crates (reader, writer, manipulate, template, markdown) do NOT depend on each other.
- Only the `easypdf` facade depends on all sub-crates.

## 6. Cargo Features

| Feature | Crates enabled | Impact | Default |
|---|---|---|:---:|
| `markdown` | `easypdf-markdown` | PDF → Markdown pipeline | ✅ |
| `html` | `printpdf/html` | HTML → PDF (requires Chromium) | ❌ |

```toml
# Default: markdown enabled
easypdf = "0.1.0"

# Disable markdown (smaller build)
easypdf = { version = "0.1.0", default-features = false }

# Enable HTML → PDF
easypdf = { version = "0.1.0", features = ["html"] }
```

## 7. Quick Start

### 7.1 Create a PDF

```rust
use easypdf::prelude::*;

EasyPdf::create("output.pdf")
    .page(PageSize::A4)
    .add_text("Hello, world!")
        .font(PdfFont::helvetica(12.0))
        .position(72.0, 700.0)
    .do_write()?;
# Ok::<(), easypdf::PdfError>(())
```

### 7.2 Read a PDF

```rust
use easypdf::prelude::*;

let text = EasyPdf::read("input.pdf")
    .pages(0..10)
    .extract_text()?;

let meta = EasyPdf::read("input.pdf")
    .extract_metadata()?;
# Ok::<(), easypdf::PdfError>(())
```

### 7.3 Merge PDFs

```rust
use easypdf::prelude::*;

EasyPdf::merge(&["a.pdf", "b.pdf", "c.pdf"], "merged.pdf")?;
# Ok::<(), easypdf::PdfError>(())
```

### 7.4 Split PDF

```rust
use easypdf::prelude::*;

EasyPdf::split("input.pdf")
    .output_dir("/tmp/pages")
    .do_split()?;
# Ok::<(), easypdf::PdfError>(())
```

### 7.5 Manipulate Pages

```rust
use easypdf::prelude::*;

EasyPdf::manipulate("input.pdf")
    .rotate_all(Rotation::Clockwise90)
    .reorder_pages(&[2, 0, 1])
    .save("reordered.pdf")?;
# Ok::<(), easypdf::PdfError>(())
```

### 7.6 Fill a Form

```rust
use easypdf::prelude::*;

#[derive(PdfModel)]
struct MyData {
    #[pdf(field = "name")]
    name: String,
}

EasyPdf::fill_form("template.pdf", &MyData { name: "Alice".into() })
    .save("filled.pdf")?;
# Ok::<(), easypdf::PdfError>(())
```

## 8. PDF → Markdown Export

The `easypdf-markdown` crate provides deterministic PDF → Markdown conversion with profile-based rendering, zero-based page ranges, export reports, and structured warnings.

### 8.1 Export API

```rust
use easypdf::prelude::*;

EasyPdf::export_markdown("input.pdf", "output.md")
    .pages(0..20)
    .profile(MarkdownProfile::Llm)
    .tables(TablePolicy::Detect)
    .ocr(OcrPolicy::Auto)
    .do_export()?;
# Ok::<(), easypdf::PdfError>(())
```

### 8.2 Markdown Profiles

| Profile | Target use case | Output style |
|---|---|---|
| `MarkdownProfile::Gfm` | GitHub / GitLab rendering | Standard GFM with tables and fenced blocks |
| `MarkdownProfile::Llm` | LLM context injection | Clean, minimal markup optimized for token efficiency |
| `MarkdownProfile::Plain` | Human reading / plain text | Minimal formatting, maximum readability |

### 8.3 Structured Warnings

When a capability is not yet implemented, the markdown pipeline emits structured warnings rather than simulating success:

```rust
use easypdf::prelude::*;

let report = EasyPdf::export_markdown("input.pdf", "output.md")
    .do_export()?;

for warning in report.warnings() {
    match warning {
        MarkdownWarning::TableDetectionUnavailable { page } => { /* ... */ }
        MarkdownWarning::ImageExtractionUnavailable { page } => { /* ... */ }
        MarkdownWarning::OcrUnavailable { page } => { /* ... */ }
    }
}
# Ok::<(), easypdf::PdfError>(())
```

## 9. Core API Overview

### 9.1 Entry Points

| Method | Returns | Purpose |
|---|---|---|
| `EasyPdf::create(path)` | `PdfCreateBuilder` | Build and write a new PDF |
| `EasyPdf::read(path)` | `PdfReadBuilder` | Extract text and metadata |
| `EasyPdf::export_markdown(input, output)` | `PdfMarkdownExportBuilder` | PDF → Markdown |
| `EasyPdf::merge(&[paths], output)` | `Result<()>` | Merge multiple PDFs |
| `EasyPdf::split(path)` | `PdfSplitBuilder` | Split PDF into pages |
| `EasyPdf::manipulate(path)` | `PdfManipulateBuilder` | Rotate, reorder pages |
| `EasyPdf::fill_form(path, data)` | `PdfFillBuilder` | Fill AcroForm fields |
| `EasyPdf::encrypt(input, output, pwd)` | `Result<()>` | ⛔ Returns `UnsupportedFeature` |
| `EasyPdf::sign(input, output, reason)` | `Result<()>` | ⛔ Returns `UnsupportedFeature` |

### 9.2 Reader Session Reuse

The `PdfReader` parses the document exactly once and retains it in memory. Subsequent operations on the same reader reuse the parsed session without re-opening the file.

```text
Reader::open(path)     → parse PDF once, retain Document
  .pages(0..5)         → filter page range (0-based)
  .extract_text()      → iterate selected pages
  .extract_metadata()  → read /Info dictionary
```

Benchmark (local, 3-page PDF):

| Operation | Latency | Ratio |
|---|---:|---:|
| Reuse parsed session | ~1,047 ns/iter | 1x |
| Re-open + re-parse | ~135,011 ns/iter | ~129x |

### 9.3 Traits

| Trait | Role | Analogy from EasyExcel |
|---|---|---|
| `PdfModel` | Map struct fields to PDF elements (derive) | `ExcelRow` |
| `PdfReadListener` | Event-driven text extraction callbacks | `ReadListener<T>` |
| `PdfWriteHandler` | Page lifecycle hooks (before/after page) | `WriteHandler` |
| `PdfConverter<T>` | Bidirectional Rust ⇄ PDF string | `Converter<T>` |
| `LayoutSink` | Backend-neutral layout consumption | — |

## 10. Error Handling & Resource Limits

### 10.1 Error Type

```rust
pub enum PdfError {
    Io(std::io::Error),
    Parse(String),
    InvalidPage(usize),
    UnsupportedFeature(String),
    ResourceLimitExceeded { resource: &'static str, limit: u64, actual: u64 },
    Encryption(String),
    Other(String),
}

pub type Result<T, E = PdfError> = std::result::Result<T, E>;
```

### 10.2 Resource Limits

All file and memory operations are bounded by `ResourceLimits`:

| Resource | Default | Exceeded behavior |
|---|---|---|
| Max file size | 100 MB | `ResourceLimitExceeded` error |
| Max pages | 10,000 | `ResourceLimitExceeded` error |
| Max text length | 10 MB | `ResourceLimitExceeded` error |

### 10.3 Atomic Output

All save operations (`Writer`, `Manipulate`, `Template`, `Markdown`) use atomic output: write to a temporary file, then rename on success. If the operation fails, the original file is not corrupted.

## 11. Safety & Non-Goals

| Non-Goal | Rationale |
|---|---|
| Encryption / signing | Not implemented; returns `UnsupportedFeature` — no fake security |
| OCR | Not implemented; markdown export emits `OcrUnavailable` warning |
| Table detection | Not implemented; markdown export emits `TableDetectionUnavailable` warning |
| Image extraction | Not implemented; markdown export emits `ImageExtractionUnavailable` warning |
| 1:1 Java EasyExcel compatibility | PDF and Excel are different paradigms; API is inspired-by, not clone-of |

## 12. Roadmap

| Phase | Focus | Key Deliverables | Status |
|:---:|---|---|:---:|
| **v0.1** | Foundation | 11 crates, core types, read/write/manipulate/template/markdown, derive macro, builder API, atomic output, resource limits | ✅ |
| **v0.2** | Rich content | Tables, images, vector shapes, custom TTF/OTF fonts, page headers/footers | 🚧 |
| **v0.3** | Watermarks & layout | Text/image watermarks, layout engine, PDF layers (OCG) | 🗓️ |
| **v0.4** | Security | AES-256 encryption/decryption, password protection | 🗓️ |
| **v0.5** | Compliance | PDF/A validation, digital signatures, XMP metadata | 🗓️ |
| **v0.6** | Converters | HTML → PDF, Markdown → PDF, SVG → PDF | 🗓️ |
| **v1.0** | Stable | Stable API, full test coverage, performance benchmarks | 🗓️ |

## 13. Performance & Benchmarks

```bash
cargo bench -p easypdf-reader --bench reader_session
```

| Scenario | Data size | Latency | Notes |
|---|---:|---:|---|
| Session reuse (parsed in memory) | 3-page PDF | ~1,047 ns/iter | Single parse, repeated access |
| Re-open + re-parse | 3-page PDF | ~135,011 ns/iter | Full I/O + parse each time |
| Speedup | — | ~129x | Session reuse vs re-open |

Hardware: local development machine. Benchmark does not equal production SLA.

## 14. Build, Test & Quality Gates

### 14.1 Basic Gates

```bash
cargo check -p easypdf --no-default-features
cargo check -p easypdf --all-features
cargo test --workspace --quiet
cargo doc --workspace --no-deps
```

### 14.2 Extended Gates

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy -p easypdf-model -p easypdf-io -p easypdf-markdown -- -D warnings
```

### 14.3 Test Matrix

| Type | Scope | Command |
|---|---|---|
| Unit tests | All crates | `cargo test --workspace` |
| Compile tests | derive macro | trybuild (1 ignored legacy test) |
| Doc tests | API examples | `cargo test --doc` |
| Feature combinations | default, no-default, all | `cargo check` variants |

## 15. Documents & Examples

| Document | Description |
|---|---|
| [Architecture (EN)](docs/easypdf-rust-Architecture.md) | Architecture design document (English) |
| [Architecture (中文)](docs/easypdf-rust-Architecture.zh_CN.md) | 架构设计文档（中文） |
| [Usage Guide](docs/usage-guide.md) | Complete API guide with 12 chapters of examples |
| [Compatibility](docs/compatibility.md) | Feature matrix + coverage report |
| [Roadmap](docs/roadmap.md) | Detailed roadmap with current/target/non-goal separation |
| [Changelog](CHANGELOG.md) | Version history and release notes |
| [Contributing](CONTRIBUTING.md) | Development setup, quality gates, commit conventions |

## 16. Contributing & License

Before submitting, run all basic gates. New public API must include docs, examples, tests, and SemVer impact notes.

This project is licensed under [Apache-2.0](LICENSE).

## 17. Related Projects

- [easyexcel-rs](https://github.com/easy-4-rust/easyexcel-rs) — Rust port of Alibaba EasyExcel
- [easyexcel](https://github.com/alibaba/easyexcel) — Original Java library
- [lopdf](https://crates.io/crates/lopdf) — Pure Rust PDF manipulation
- [printpdf](https://crates.io/crates/printpdf) — Pure Rust PDF generation

---

<div align="center">

[Back to top](#readme-top) · [docs.rs](https://docs.rs/easypdf) · [crates.io](https://crates.io/crates/easypdf) · [Issues](https://github.com/easy-4-rust/easypdf-rust/issues)

</div>
