# easypdf-derive

> Proc-macro crate: `#[derive(PdfModel)]` with 12+ `#[pdf(...)]` attributes for declarative PDF content mapping.

## Role

`easypdf-derive` is the proc-macro crate for easypdf-rust. It auto-generates `PdfModel` trait implementations for Rust structs via `#[derive(PdfModel)]`. With rich `#[pdf(...)]` attributes, developers can declaratively map Rust structs to PDF content elements (text, tables, images, form fields) without writing rendering logic by hand.

## Core Capabilities

- **`#[derive(PdfModel)]`** -- auto-implements `PdfModel` trait, generating `render()`, `metadata()`, and `field_descriptors()` methods (`crates/easypdf-derive/src/lib.rs:54`)
- **Struct-level attributes** -- `#[pdf(page = A4, orientation = Portrait)]` for page configuration (`crates/easypdf-derive/src/lib.rs:44`)
- **Field-level attributes** -- `text`, `table`, `image`, `ignore`/`skip`, `field`, `order`, `default`, `required`, `format`, `nested`, `font`, `size` (`crates/easypdf-derive/src/lib.rs:46-55`)
- **Field descriptor generation** -- produces `PdfFieldDescriptor` for form filling and data mapping (`crates/easypdf-core/src/traits.rs:40`)
- **Compile-time validation** -- `trybuild` tests ensure invalid attributes produce clear errors (`crates/easypdf-derive/Cargo.toml:dev-dependencies`)

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `syn` | 3.0.3 | Rust source parsing (features = ["full"]) |
| `quote` | 1.0.47 | Code generation |
| `proc-macro2` | 1.0.107 | Proc-macro 2.0 primitives |
| `proc-macro-crate` | 3.5.0 | Crate name resolution |

## Main API

### `#[derive(PdfModel)]`

```rust
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
#[pdf(page = A4, orientation = Portrait)]
struct Invoice {
    #[pdf(text, position = (100, 700), font = "Helvetica", size = 14)]
    title: String,

    #[pdf(table, position = (50, 600))]
    items: Vec<Vec<String>>,

    #[pdf(image, position = (400, 700))]
    logo: Vec<u8>,

    #[pdf(field = "invoice_number", required)]
    number: String,

    #[pdf(order = 1)]
    date: String,

    #[pdf(nested)]
    address: Address,

    #[pdf(ignore)]
    internal_note: String,
}
```

### Attribute Reference

| Attribute | Description |
|-----------|-------------|
| `#[pdf(page = A4, orientation = Portrait)]` | Struct-level page configuration |
| `#[pdf(text, position = (x, y))]` | Render field as positioned text |
| `#[pdf(table, position = (x, y))]` | Render field as table |
| `#[pdf(image, position = (x, y))]` | Render field as image |
| `#[pdf(ignore)]` / `#[pdf(skip)]` | Skip this field |
| `#[pdf(field = "name")]` | Map to PDF form field name |
| `#[pdf(order = N)]` | Display/render order |
| `#[pdf(default = "value")]` | Default value when empty |
| `#[pdf(required)]` | Field must be non-empty |
| `#[pdf(format = "pattern")]` | Format pattern (e.g. `"YYYY-MM-DD"`) |
| `#[pdf(nested)]` | Recursively include inner model elements |
| `#[pdf(font = ...)]` | Set text rendering font |
| `#[pdf(size = N)]` | Set text rendering font size |

## Generated Code

The derive macro generates:

```rust
// For each struct with #[derive(PdfModel)]
impl PdfModel for MyStruct {
    fn render(&self) -> Result<Vec<RenderedElement>> {
        // Generated rendering logic based on field attributes
        // text -> positioned text, table -> table, image -> image
    }

    fn metadata(&self) -> PdfModelMetadata {
        // Page size, orientation from struct-level #[pdf(...)]
    }

    fn field_descriptors(&self) -> Vec<PdfFieldDescriptor> {
        // Generated from #[pdf(field = "...", required, default = "...")]
    }
}
```

## Compile-Time Validation

Invalid attributes produce clear compiler errors:

```rust
#[derive(PdfModel)]
struct Bad {
    #[pdf(text)]  // Error: missing position for text field
    name: String,

    #[pdf(field = "x", default = "val", required)]  // Error: default and required are mutually exclusive
    value: String,
}
```

## License

Apache-2.0

---

**Project**: https://github.com/easy-4-rust/easypdf-rust
**crates.io**: https://crates.io/crates/easypdf-derive
**docs.rs**: https://docs.rs/easypdf-derive
