# easypdf-rust Roadmap

## 0.1 — Foundation ✅ Complete

- [x] 11-crate workspace with proper dependency hierarchy
- [x] `EasyPdf` static factory with fluent builder API
- [x] `#[derive(PdfModel)]` proc-macro for compile-time struct-to-PDF mapping
- [x] PDF creation: text, built-in fonts (14 standard), metadata, page sizes
- [x] PDF reading: text extraction, metadata extraction
- [x] Reader single-parse session reuse (~129x speedup vs re-open)
- [x] Merge multiple PDFs with valid `/Pages` tree
- [x] Split PDF into individual pages with valid `/Pages` tree
- [x] Rotate pages (per-page or all-page, 0°/90°/180°/270°)
- [x] Reorder pages in arbitrary order
- [x] AcroForm field filling (template)
- [x] PDF → Markdown with GFM/LLM/Plain profiles
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

## 0.2 — Rich Content

- [ ] Table layout rendering in Writer (headers, rows, column widths, borders)
- [ ] Image insertion (JPEG/PNG) with size and position control
- [ ] Vector shapes (lines, rectangles, circles, polygons)
- [ ] Custom TTF/OTF font registration and embedding
- [ ] Page headers and footers
- [ ] Multi-page writer with automatic page breaks
- [ ] PDF → Markdown: real table detection backend
- [ ] PDF → Markdown: real image extraction backend
- [ ] Configurable resource limits (user-supplied `ResourceLimits`)
- [ ] Writer: `FlowLayout` auto-positioning integration

## 0.3 — Watermarks & Layout

- [ ] Text watermarks (rotation, opacity, font, position)
- [ ] Image watermarks (PNG overlay)
- [ ] PDF layers (Optional Content Groups / OCG)
- [ ] Background/foreground overlay
- [ ] Layout engine: vertical flow with margin, spacing, alignment
- [ ] Layout engine: horizontal flow for multi-column
- [ ] Layout engine: table auto-layout

## 0.4 — Security

- [ ] AES-256 encryption/decryption
- [ ] Password protection (user password + owner password)
- [ ] Permission flags (print, copy, modify, annotate)
- [ ] PDF → Markdown: OCR backend integration (structured warning → real extraction)

## 0.5 — Compliance

- [ ] PDF/A-1b validation
- [ ] PDF/A-2b validation
- [ ] PDF/A-3b validation
- [ ] Digital signatures (PKCS#7 / CMS)
- [ ] XMP metadata
- [ ] Document info dictionary standardization

## 0.6 — Converters

- [ ] HTML → PDF (Chromium-based, feature-gated)
- [ ] Markdown → PDF (via HTML intermediate)
- [ ] SVG → PDF
- [ ] PDF → image (rasterize pages)

## 1.0 — Stable

- [ ] Stable public API with semver guarantees
- [ ] Full test coverage (unit, integration, property, fuzz)
- [ ] Performance benchmarks for all operations
- [ ] Complete documentation (API reference, examples, migration guide)
- [ ] crates.io publication
- [ ] CI matrix: Linux, macOS, Windows, MSRV, nightly

## Non-Goals

- Full PDF rendering/viewer engine (use external viewers)
- 1:1 Java PDFBox compatibility (API inspired-by, not clone-of)
- Real-time collaborative editing
- PDF to Word/Excel conversion
- OCR/LLM image description (via trait injection, not default dependency)
- Old-style `.pdf` to `.doc` conversion
