# easypdf-reader

> PDF reading layer: parsing, text extraction, page manipulation (merge/split/rotate/reorder/watermark), with three adaptive read strategies.

## Role

`easypdf-reader` handles all PDF input operations in the easypdf-rust workspace. Built on the `lopdf` backend, it provides text extraction, metadata reading, page manipulation (merge, split, rotate, reorder, watermark, layer), and PDF/A validation. It automatically selects the optimal read strategy based on file size to balance speed for small files against memory efficiency for large ones.

## Core Capabilities

- **Three read strategies** (`Full` / `Lazy` / `Streaming`) -- auto-selected by file size: 0-5 MB = Full, 5-100 MB = Lazy, >100 MB = Streaming (`crates/easypdf-reader/src/strategy.rs:56-68`)
- **Text extraction** (`extract_text()`) -- plain text from PDF, with CMap/ToUnicode support for CJK fonts (`crates/easypdf-reader/src/reader/extract.rs`)
- **Page manipulation** (`PdfManipulator`) -- merge, split, rotate, reorder, extract pages, add text watermark, add optional content groups (layers) (`crates/easypdf-reader/src/manipulate.rs`)
- **PDF repair** (`open_with_repair()`) -- auto-detect and fix corrupted PDF files (`crates/easypdf-core/src/io/repair.rs`)
- **Resource guards** -- decompression bomb and element explosion protection via `ResourceLimits` (`crates/easypdf-core/src/io/guards.rs`)
- **Streaming scanner** (`StreamScanner`) -- byte-stream scanning without building a full `Document` object for very large files (`crates/easypdf-reader/src/streaming/`)
- **PDF/A validation** (`validate_pdfa()`) -- check PDF/A-1b compliance (`crates/easypdf-reader/src/manipulate.rs:260`)
- **Benchmark suite** -- reader session benchmarks (`crates/easypdf-reader/benches/reader_session.rs`)

## Dependencies

### Internal

| Crate | Purpose |
|-------|---------|
| `easypdf-core` | Core types (`PdfInput`, `ResourceLimits`, `PageRange`, error types, IO guards) |

### External

| Crate | Version | Purpose |
|-------|---------|---------|
| `lopdf` | 0.44.0 | PDF parsing engine |
| `flate2` | 1.1.9 | Stream decompression (Streaming strategy) |

## Main API

### PdfReader

```rust
use easypdf_reader::{PdfReader, ReadStrategy};

// Auto strategy (selected by file size)
let reader = PdfReader::open("document.pdf")?;
let text = reader.extract_text()?;

// Specify strategy
let reader = PdfReader::open_with_strategy("large.pdf", ReadStrategy::Lazy)?;
let text = reader.pages(0..5).extract_text()?;

// From memory bytes
let reader = PdfReader::from_bytes(pdf_bytes)?;

// With auto-repair
let reader = PdfReader::open_with_repair("corrupted.pdf", true, ReadStrategy::Full)?;

// With custom resource limits
let reader = PdfReader::open_with_limits(input, ResourceLimits::default())?;
```

### ReadStrategy

```rust
use easypdf_reader::ReadStrategy;

// Auto-select by file size
let strategy = ReadStrategy::auto(50_000_000); // 50 MB -> Lazy

// Manual selection
let s = ReadStrategy::Full;     // <5 MB, full object tree
let s = ReadStrategy::Lazy;     // 5-100 MB, lazy page loading
let s = ReadStrategy::Streaming; // >100 MB, byte-stream scan
```

### PdfManipulator

```rust
use easypdf_reader::PdfManipulator;

// Merge multiple PDFs
PdfManipulator::merge_files(&["a.pdf", "b.pdf"], "merged.pdf")?;

// Open and manipulate
let mut m = PdfManipulator::open("input.pdf")?;
m.rotate_page(0, Rotation::Clockwise90)?;
m.reorder_pages(&[2, 0, 1])?;
m.extract_pages(&(0..5))?;
m.add_text_watermark("CONFIDENTIAL", 48.0, 0.3)?;
m.add_layer("Annotations")?;
m.validate_pdfa()?;
```

## Known Limitations

- `ReadStrategy::Streaming` does not build a complete object tree -- precision is lower than Full/Lazy, especially for CJK text boundaries (`crates/easypdf-reader/src/strategy.rs:47-51`)
- Streaming mode skips cross-reference parsing and font encoding (CMap/ToUnicode) for speed

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-reader
**docs.rs**: https://docs.rs/easypdf-reader
