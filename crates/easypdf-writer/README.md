# easypdf-writer

> PDF writing layer: create new PDF documents (text, tables, images, SVG, shapes, custom fonts), with constant-memory spill mode for large documents.

## Role

`easypdf-writer` handles all PDF output operations in the easypdf-rust workspace. Built on the `printpdf` backend, it supports text (14 built-in fonts + custom TTF/TTC), tables with styling, PNG/JPEG images, SVG vector graphics, and shape primitives. It offers two write backends: `InMemory` (default, fast for small docs) and `Spill` (page-level temp files, constant memory for large docs), plus an `Auto` mode that selects based on page count threshold.

## Core Capabilities

- **Text writing** -- 14 built-in fonts (`BuiltInFont`) + custom TTF/TTC font registration (`crates/easypdf-writer/src/writer.rs`)
- **Table writing** -- headers, rows, cell styles, borders via `PdfTable` + `TableStyle` (`crates/easypdf-core/src/content.rs`, `crates/easypdf-core/src/style.rs`)
- **Image writing** -- PNG/JPEG embedding from path or bytes (`crates/easypdf-writer/src/writer.rs`)
- **SVG writing** -- vector graphics embedding (`crates/easypdf-writer/src/writer.rs`)
- **Shape drawing** -- lines, rectangles, circles (`crates/easypdf-writer/src/writer.rs`)
- **Handler lifecycle** -- `PdfWriteHandler` hooks: `before_document` / `before_page` / `after_page` / `after_document` (`crates/easypdf-core/src/traits.rs:183`)
- **AcroForm template filling** -- `PdfTemplateFiller` for filling existing PDF forms (`crates/easypdf-writer/src/template.rs`)
- **Two write backends** -- `InMemory` (default) and `Spill` (constant memory via page-level temp files) with `Auto` threshold selection (`crates/easypdf-writer/src/backend.rs`)

## Dependencies

### Internal

| Crate | Purpose |
|-------|---------|
| `easypdf-core` | Core types (`PdfText`, `PdfTable`, `PdfImage`, `PdfFont`, `PdfWriteHandler`, `AtomicFileOutput`) |

### External

| Crate | Version | Purpose |
|-------|---------|---------|
| `printpdf` | 0.12.4 | PDF creation engine (features: png, html, svg) |
| `lopdf` | 0.44.0 | PDF object model (for post-processing & template filling) |
| `image` | 0.25.9 | Image decoding |
| `serde` / `serde_json` | 1.x | Template data serialization |
| `chrono` | 0.4.45 | Timestamps |

## Main API

### PdfWriter

```rust
use easypdf_writer::PdfWriter;
use easypdf_core::*;

let mut w = PdfWriter::new("My Document");
w.add_page(PageSize::A4, Orientation::Portrait)?;
w.write_text("Hello, world!", 100.0, 700.0)?;
w.add_text(&PdfFont::times_roman(12.0), "Auto-positioned text")?;
w.write_image_from_path("logo.png", 50.0, 50.0, 200.0, 100.0)?;
w.draw_line(50.0, 680.0, 545.0, 680.0, 1.0)?;
w.register_font_from_path("custom.ttf")?;
w.finish("output.pdf")?;
```

### PdfWriterBuilder

```rust
use easypdf_writer::{PdfWriterBuilder, WriteBackend};

let w = PdfWriterBuilder::new("Big Report")
    .metadata(PdfMetadata::new().title("Q4 Report"))
    .backend(WriteBackend::auto(500)) // auto-select at 500 pages
    .constant_memory(true)
    .build()?;
```

### PdfTemplateFiller

```rust
use easypdf_writer::PdfTemplateFiller;

let filler = PdfTemplateFiller::new("template.pdf")?;
let output = filler.fill(&model)?.save_to("filled.pdf")?;
```

### WriteBackend

```rust
use easypdf_writer::WriteBackend;

// Default: in-memory (fast, small docs)
let backend = WriteBackend::InMemory;

// Spill: page-level temp files, constant memory
let backend = WriteBackend::Spill;

// Auto: select at page threshold
let backend = WriteBackend::auto(500);
```

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-writer
**docs.rs**: https://docs.rs/easypdf-writer
