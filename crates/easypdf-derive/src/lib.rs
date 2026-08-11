//! Proc-macro derive for the `PdfModel` trait.
//!
//! Provides `#[derive(PdfModel)]` which generates compile-time
//! reflection code mapping Rust struct fields to PDF content elements.
//!
//! ## Usage
//!
//! ```ignore
//! use easypdf_derive::PdfModel;
//!
//! #[derive(PdfModel)]
//! #[pdf(page = A4, orientation = Portrait)]
//! struct Invoice {
//!     #[pdf(text, position = (100, 700))]
//!     title: String,
//! }
//! ```
//!
//! ## Field Attributes
//!
//! | Attribute | Description |
//! |---|---|
//! | `#[pdf(text, position = (x, y))]` | Render field as positioned text |
//! | `#[pdf(table, position = (x, y))]` | Render field as a table |
//! | `#[pdf(image, position = (x, y))]` | Render field as an image |
//! | `#[pdf(ignore)]` / `#[pdf(skip)]` | Skip field entirely |
//! | `#[pdf(field = "name")]` | Map to PDF form field name |
//! | `#[pdf(order = N)]` | Display/render order |
//! | `#[pdf(default = "value")]` | Default value if empty |
//! | `#[pdf(required)]` | Field must be non-empty |
//! | `#[pdf(format = "pattern")]` | Format pattern (e.g. `"YYYY-MM-DD"`) |
//! | `#[pdf(nested)]` | Recursively include inner model's elements |
//! | `#[pdf(font = ...)]` | Set font for text rendering |
//! | `#[pdf(size = N)]` | Set font size for text rendering |

use proc_macro::TokenStream;

mod implementation;

/// Derive macro that generates a [`PdfModel`] trait implementation.
///
/// # Attributes
///
/// - `#[pdf(page = ..., orientation = ..., margins = ...)]` on the struct
/// - `#[pdf(text, position = (x, y), font = ...)]` on text fields
/// - `#[pdf(table, position = (x, y), headers = [...])]` on collection fields
/// - `#[pdf(field = "field_name")]` on form/template fields
/// - `#[pdf(order = N)]` display order
/// - `#[pdf(ignore)]` / `#[pdf(skip)]` to skip a field
/// - `#[pdf(default = "value")]` default value
/// - `#[pdf(required)]` mark field as required
/// - `#[pdf(format = "pattern")]` format pattern
/// - `#[pdf(nested)]` recursively render inner model
#[proc_macro_derive(PdfModel, attributes(pdf))]
pub fn derive_pdf_model(input: TokenStream) -> TokenStream {
    implementation::expand_pdf_model(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
