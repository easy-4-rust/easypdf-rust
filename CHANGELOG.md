# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

_No unreleased changes._

## [0.1.1] - 2026-08-16

Quality and compliance patch release. No public API changes; all existing
paths remain valid.

### Fixed

- **CI restored to green**: the global `bin/` and `*.json` gitignore rules
  silently excluded 4 test-utility binaries and 6 golden baseline files that
  were declared in Cargo.toml/tests -- fresh checkouts failed to build.
  Added negation rules and committed the files.
- **`pdfium` feature compiles again**: pdfium-render 0.8.37 API drift
  (u16 page indexes, `set_maximum_height` builder, non-Send/Sync `Pdfium`
  handle) plus a broken doc example.
- **`ocrs` feature clippy-clean**: pedantic cast warnings on pixel coords.
- **Flaky resident port-file test**: TOCTOU race between parallel tests on
  Linux CI eliminated via a module mutex.
- **432 rustfmt violations** across 104 files formatted.

### Changed

- **mod.rs purity**: 12 `mod.rs` files no longer define types/functions;
  definitions moved to dedicated per-type files with `pub use` re-exports
  (all public paths unchanged).
- **Chinese documentation**: every pub type and pub method across ~150
  production files now carries Chinese doc comments (code examples
  byte-preserved; test modules unchanged).
- **Code spec compliance**: zero production wildcard imports, zero
  todo!/unimplemented!(), zero files above 800 lines.
- **deny.toml added**: explicit license allowlist matching the dependency
  tree, advisory ignores documented with reasons; `cargo deny check`
  passes all four categories.

### Verification

1535 tests + 71 doctests (all features); clippy/fmt/rustdoc zero warnings;
MSRV 1.88; coverage 90.86% lines (excluding dev utility bins).

## [0.1.0] - 2026-08-12

First public release of easypdf-rust -- a pure-Rust PDF library with builder API,
OCR, Markdown conversion, MCP server, and resident daemon.

### Added

- **22-crate consolidation to 9 crates**: Workspace refactored from ~22 fine-grained
  crates into 9 focused crates, reducing compile times and dependency graph complexity.
  (See [docs/easypdf-rust-Architecture.md](docs/easypdf-rust-Architecture.md) for mapping.)
- **Streaming ReadStrategy**: `ReadStrategy::Streaming` performs byte-stream scanning
  without building a full `Document` object. `ReadStrategy::auto` selects the optimal
  strategy by file size (Full < 5 MB, Lazy 5-100 MB, Streaming > 100 MB).
- **CMap / ToUnicode support**: Correct handling of CMap-encoded fonts in
  `easypdf-reader`, fixing garbled CJK text extraction.
- **WriteBackend selection**: `easypdf-writer` supports `InMemory` (default),
  `Spill` (page-level temp files, constant memory), and `Auto` (threshold-based).
- **PdfWriterBuilder + WriteHandlerChain**: Composable write-handler pipeline
  with priority-sorted stable execution.
- **ConverterRegistry**: Type-erased bidirectional converter registry in
  `easypdf-core::converter_registry`.
- **ProcessorPipeline**: Capability-negotiating, priority-sorted markdown processor chain.
- **4 cloud OCR engines** (new `easypdf-ocr` crate):
  - GLM-OCR (Zhipu BigModel, feature-gated `ocr-glm`)
  - HunyuanOCR (Tencent Cloud, TC3-HMAC-SHA256 signature, feature-gated `ocr-hunyuan`)
  - Baidu Qianfan / PP-OCRv6 (14 API endpoints + OAuth token management, feature-gated `ocr-baidu`)
  - DeepSeek-OCR-2 (OpenAI-compatible protocol, feature-gated `ocr-deepseek`)
  - Unified `HttpOcrEngine` trait with reqwest blocking HTTP, base64 image encoding,
    structured `OcrHttpError`.
- **Resident daemon** (new `easypdf-runtime` crate): Unix socket + Windows TCP
  fallback via `Transport` trait abstraction, adaptive autosave (EMA smoothing),
  idle-timeout watchdog.
- **MCP server** (new `easypdf-runtime` crate): 7 tools (`pdf_read_text`,
  `pdf_to_markdown`, `pdf_create_text`, `pdf_merge`, `pdf_split`, `pdf_metadata`,
  `pdf_page_count`) over stdio JSON-RPC for LLM agent integration.
- **PdfBlock IR expanded**: From 5 to 14 variants (added Code, Formula, PageBreak,
  Footnote, TableCell, BlockQuote, HorizontalRule, Link, Unknown).
- **easypdf-derive attributes**: 8 new attributes -- `field`, `order`, `skip`,
  `default`, `required`, `format`, `nested`, `font`/`size`.
- **tracing observability**: Workspace-level `tracing` + `tracing-subscriber`
  (`env-filter` + JSON output), structured spans across reader/writer/markdown/IPC.
- **Transport trait**: `easypdf-runtime::transport` provides unified Unix socket
  (default) and Windows TCP fallback interface.
- **ISO 32000 encryption**: AES-128 (V4/R4) and AES-256 (V5/R6) encryption with
  full permission control (PRINT, MODIFY, COPY, FILL_FORMS, etc.).
- **ISO 32000 digital signatures**: PKCS#7/CMS detached SignedData with RSA-PKCS#1v1.5
  + SHA-256 via `ring`, X.509 certificate parsing via `x509-parser`.
- **cargo-fuzz**: 6 fuzz targets -- `pdf_parse`, `streaming_scan`,
  `pdf_encrypt_decrypt`, `pdf_sign_verify`, `markdown_convert`, `ssrf_url`.

### Security

- **RUSTSEC-2023-0071 fix (Marvin Attack)**: Migrated from `rsa` to `ring` 0.17.14
  constant-time RSA in production code paths. `rsa` retained only as dev-dependency
  for test certificate generation (ring has no keygen API). Advisory ignored via
  `.cargo/audit.toml`.
- **RUSTSEC-2025-0055 fix**: Upgraded `tracing-subscriber` to >=0.3.20, fixing
  ANSI escape sequence injection vulnerability.
- **Decompression bomb guard fix**: Removed 64 KB exemption; guard now checks
  absolute decompressed size regardless of input size.
- **SSRF IPv6 guard**: Full IPv6 coverage -- loopback, ULA, link-local, and
  IPv4-mapped addresses are all blocked.
- **API key Debug redact**: Structs containing secrets (`GlmConfig`, `BaiduConfig`,
  etc.) no longer leak `api_key` / `secret_key` in `Debug` output.
- **Double-hash signature fix**: Signatures no longer hash before signing
  (conforming to CMS spec), fixing signature verification failures.

### Changed

- **Architecture consolidation**: 22 crates merged into 9 crates (see mapping table).
- **`EasyPdf::encrypt()` full implementation**: Replaces previous `UnsupportedFeature`
  stub; supports `PdfEncryption` builder-pattern configuration.
- **`EasyPdf::sign()` full implementation**: Replaces previous `UnsupportedFeature`
  stub; supports `SignatureInfo` builder-pattern configuration.
- **`PdfEncryption` new fields**: `permissions` (PDF permission bits) + `algorithm`
  (encryption algorithm selection) + builder methods.
- **`SignatureInfo` new fields**: X.509 metadata -- `signer_name`, `issuer`,
  `cert_not_before`, `cert_not_after`.
- **Markdown processor chain refactored**: Now based on `ProcessorPipeline` with
  capability negotiation.
- **Feature system rebuilt**: Fixed latent bugs -- `ocr` feature no longer silently
  activates `http-base`; `markdown-table`, `render`, `ocr` now enable submodules
  within `easypdf-markdown` instead of separate crates; `resident` and `mcp` features
  moved to `easypdf-runtime`.
- **File size split**: `streaming`, `lib.rs`, and similar files kept under 800 lines;
  compliance improved from 80% to 95%.
- **Clippy configuration**: Workspace-level `similar_names = "allow"` added
  (PDF object names `page_dict` / `pages_dict` cause false positives).

### Fixed

- **Writer metadata UTF-16BE encoding**: Writer now correctly writes UTF-16BE BOM
  + encoding into PDF metadata; reader detects BOM for decoding.
- **Baidu OCR Digit path**: `digit` -> `numbers` (corrected API endpoint path).
- **Baidu OCR Structured path**: `structured` -> `smart_struct` (corrected API
  endpoint path). (Note: these were endpoint path corrections, not logic bugs.)
- **Parity roundtrip_metadata tests**: All roundtrip tests pass after writer
  metadata persistence fix.
- **Test isolation**: Each parity test uses an independent tempdir, preventing
  parallel test race conditions.
- **byte_finder OOB panic**: Out-of-bounds panic discovered by fuzz testing fixed.
- **rustdoc warnings**: Fixed broken intra-doc links in `easypdf-markdown`
  (redundant explicit link targets, unresolved module paths).

### Documentation

- 14 bilingual (English + Chinese) documentation files in `docs/`.
- 11 examples + `crates/easypdf/examples/README.md`.
- `docs/security/AUDIT.md` + `docs/security/AUDIT-IGNORED.md`.
- `docs/performance/BENCHMARK.md`.
- 0 rustdoc warnings across workspace.
- Roadmap synchronized with actual v0.2 completion status.

[Unreleased]: https://github.com/easy-4-rust/easypdf-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/easy-4-rust/easypdf-rust/releases/tag/v0.1.0
