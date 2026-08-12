# easypdf-rust Roadmap

> **TL;DR**: v0.2 Architecture Consolidation **Done** (19 items). v0.3 Rich Content **In Progress** (1/6 done). v0.4 Security **mostly Done** (3/4). Working toward v0.3 completion.

## Progress Metrics

| Metric | Current |
|--------|---------|
| Tests passing | 1522 |
| Test coverage | 91.61% |
| Cargo audit CVEs | 0 |
| Clippy warnings | 0 |
| Rustdoc warnings | 0 |
| Fuzz targets | 6 |
| crates.io published | v0.1.0 (8 crates) |
| Workspace crates | 9 (consolidated from 22) |

---

## 0.1 — Foundation ✅ Done

- [x] 22-crate-to-9-crate workspace consolidation with proper dependency hierarchy
- [x] `EasyPdf` static factory with fluent builder API
- [x] `#[derive(PdfModel)]` proc-macro for compile-time struct-to-PDF mapping
- [x] PDF creation: text, built-in fonts (14 standard), metadata, page sizes
- [x] PDF reading: text extraction, metadata extraction
- [x] Reader single-parse session reuse (~129x speedup vs re-open)
- [x] Merge multiple PDFs with valid `/Pages` tree
- [x] Split PDF into individual pages with valid `/Pages` tree
- [x] Rotate pages (per-page or all-page, 0/90/180/270)
- [x] Reorder pages in arbitrary order
- [x] AcroForm field filling (template)
- [x] PDF to Markdown with GFM/LLM/Plain profiles
- [x] Zero-based page ranges with correct PDF 1-based mapping
- [x] Engine-neutral semantic IR (`PdfDocumentModel`, `PdfPageModel`, `PdfBlock`)
- [x] Backend-neutral layout abstraction (`LayoutSink` trait, `FlowLayout`)
- [x] Atomic output (temp file + atomic rename) for all save operations
- [x] Resource limits: file size, page count, text length
- [x] Writer lifecycle hooks (`PdfWriteHandler`)
- [x] Event-driven read listeners (`PdfReadListener`)
- [x] Structured warnings for unimplemented capabilities
- [x] `#![forbid(unsafe_code)]` in every crate
- [x] 136 tests, bilingual README, architecture docs, usage guide
- [x] Reader session benchmark

## 0.2 — Architecture Consolidation ✅ Done

- [x] 22-to-9 crate consolidation (easypdf-core/reader/writer/markdown/ocr/runtime/derive/)
- [x] Streaming ReadStrategy (Full/Lazy/Streaming + auto-select by file size)
- [x] CMap/ToUnicode support (CJK text extraction)
- [x] WriteBackend selection (InMemory / Spill / Auto for constant-memory writes)
- [x] PdfWriterBuilder + WriteHandlerChain (priority-based composable pipeline)
- [x] ConverterRegistry (type-erased bidirectional converter registration)
- [x] 4 cloud OCR engines (GLM / Hunyuan / Baidu / DeepSeek via unified HttpOcrEngine)
- [x] Resident daemon (Unix socket + Windows TCP fallback, adaptive autosave)
- [x] MCP server (7 tools, stdio JSON-RPC for LLM agent integration)
- [x] PdfBlock IR expanded to 14 variants (Code/Formula/PageBreak/Footnote/TableCell/BlockQuote/HorizontalRule/Link/Unknown)
- [x] ProcessorPipeline with capability negotiation and priority sorting
- [x] ISO 32000 PDF spec encryption (AES-128/256, permission flags)
- [x] ISO 32000 PDF spec signature (PKCS#7/CMS detached, X.509 via ring + x509-parser)
- [x] tracing observability (structured spans across reader/writer/markdown/IPC)
- [x] Security fixes (rsa-to-ring migration, SSRF IPv6 coverage, API key Debug redact)
- [x] cargo-fuzz (6 targets: pdf_parse, streaming_scan, pdf_encrypt_decrypt, pdf_sign_verify, markdown_convert, ssrf_url)
- [x] Test coverage 91.61%
- [x] v0.1.0 published to crates.io (8 crates)
- [x] Bilingual documentation (Chinese + English README, architecture docs, usage guide)

## 0.3 — Rich Content 🔄 In Progress

- [x] `add_table` Builder API (table layout rendering derived from PDF model fields)
- [ ] Table border style enhancements (zebra striping, custom borders)
- [ ] Image insertion (JPEG/PNG) with size and position control
- [ ] Vector shapes (lines, rectangles, circles)
- [ ] Custom TTF/OTF font registration and embedding
- [ ] Multi-page writer with automatic page breaks

## 0.4 — Security 🔄 Mostly Done

- [x] AES-256 encryption/decryption (ISO 32000 compliant)
- [x] Password protection (user password + owner password)
- [x] Permission flags (print, copy, modify, annotate)
- [ ] PDF to Markdown: OCR real integration (currently mock + cloud API ready, needs end-to-end wiring)

## 0.5 — Compliance ⏳ Planned

- [ ] PDF/A-1b validation
- [ ] PDF/A-2b validation
- [ ] PDF/A-3b validation
- [ ] XMP metadata
- [ ] Document info dictionary standardization

## 0.6 — Converters 🔄 Partial

- [x] HTML to PDF (Chromium-based via printpdf html feature, feature-gated)
- [ ] Markdown to PDF optimization
- [ ] SVG to PDF (svg2pdf available via printpdf transitive dependency)
- [ ] PDF to image rasterize (partially via easypdf-render trait)

## 1.0 — Stable ⏳ Planned

- [x] Public API published to crates.io
- [ ] semver guarantees (0.2.x to 0.3.x to 1.0 progression)
- [x] CI matrix (Linux + macOS)
- [ ] Windows MSRV testing
- [ ] Property-based testing
- [ ] Complete migration guide

## Non-Goals

- Full PDF rendering/viewer engine (use external viewers)
- 1:1 Java PDFBox compatibility (API inspired-by, not clone-of)
- Real-time collaborative editing
- PDF to Word/Excel conversion
- OCR/LLM image description (via trait injection, not default dependency)
- Old-style `.pdf` to `.doc` conversion
