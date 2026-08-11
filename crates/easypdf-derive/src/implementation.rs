//! Implementation of the `#[derive(PdfModel)]` proc-macro.
//!
//! Supports the following field-level attributes:
//!
//! - `#[pdf(text, position = (x, y), font = ..., size = ...)]` -- render as text
//! - `#[pdf(table, position = (x, y))]` -- render as table
//! - `#[pdf(image, position = (x, y))]` -- render as image
//! - `#[pdf(ignore)]` -- skip field entirely (no render, no descriptor)
//! - `#[pdf(skip)]` -- alias for `ignore`
//! - `#[pdf(field = "pdf_field_name")]` -- PDF form field mapping
//! - `#[pdf(order = N)]` -- display/render order
//! - `#[pdf(default = "value")]` -- default value if field is empty
//! - `#[pdf(required)]` -- field must be non-empty
//! - `#[pdf(format = "pattern")]` -- format pattern (e.g. "YYYY-MM-DD")
//! - `#[pdf(nested)]` -- recursively include inner model's elements

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Result};

/// Resolve the `easypdf_core` crate name at compile time.
fn core_crate() -> TokenStream {
    let name = match proc_macro_crate::crate_name("easypdf-core")
        .or_else(|_| proc_macro_crate::crate_name("easypdf_core"))
    {
        Ok(found) => match found {
            proc_macro_crate::FoundCrate::Name(n) => n,
            proc_macro_crate::FoundCrate::Itself => "easypdf_core".to_string(),
        },
        Err(_) => "easypdf_core".to_string(),
    };
    let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    quote! { #ident }
}

/// Entry point: expands `#[derive(PdfModel)]` into the trait implementation.
pub(crate) fn expand_pdf_model(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    let core = core_crate();

    // Parse struct-level #[pdf(...)] attributes
    let PdfStructAttrs {
        page_size,
        orientation,
        margins,
    } = parse_struct_attrs(&input.attrs)?;

    // Parse all fields and their attributes
    let fields = get_named_fields(&input)?;
    let parsed_fields: Vec<ParsedField> = fields
        .iter()
        .map(parse_field_attrs)
        .collect::<Result<Vec<_>>>()?;

    // Generate field rendering code
    let render_arms = generate_render_arms(&parsed_fields, &core)?;

    // Generate field_descriptors code
    let descriptors = generate_field_descriptors(&parsed_fields, &core);

    let expanded = quote! {
        impl #core::PdfModel for #name {
            fn render(&self) -> #core::Result<Vec<#core::RenderedElement>> {
                let mut elements = Vec::new();
                #render_arms
                Ok(elements)
            }

            fn metadata(&self) -> #core::PdfModelMetadata {
                #core::PdfModelMetadata {
                    page_size: #page_size,
                    orientation: #orientation,
                    margins: #margins,
                }
            }

            fn field_descriptors(&self) -> Vec<#core::PdfFieldDescriptor> {
                let mut descriptors = Vec::new();
                #descriptors
                descriptors
            }
        }
    };

    Ok(expanded)
}

// ============================================================================
// Struct-level attributes
// ============================================================================

struct PdfStructAttrs {
    page_size: TokenStream,
    orientation: TokenStream,
    margins: TokenStream,
}

fn parse_struct_attrs(attrs: &[syn::Attribute]) -> Result<PdfStructAttrs> {
    let core = core_crate();
    let mut page_size = quote! { #core::PageSize::A4 };
    let mut orientation = quote! { #core::Orientation::Portrait };
    let mut margins = quote! { 72.0_f64 };

    for attr in attrs {
        if !attr.path().is_ident("pdf") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("page") {
                let value: syn::Expr = meta.value()?.parse()?;
                page_size = quote! { #value };
            } else if meta.path.is_ident("orientation") {
                let value: syn::Expr = meta.value()?.parse()?;
                orientation = quote! { #value };
            } else if meta.path.is_ident("margins") {
                let value: syn::Expr = meta.value()?.parse()?;
                margins = quote! { #value };
            }
            Ok(())
        })?;
    }

    Ok(PdfStructAttrs {
        page_size,
        orientation,
        margins,
    })
}

// ============================================================================
// Field-level attributes
// ============================================================================

/// All parsed attributes for a single field.
struct ParsedField {
    /// The Rust field identifier.
    ident: syn::Ident,
    /// The type of the field (reserved for future validation).
    #[allow(dead_code)]
    ty: syn::Type,
    /// Rendering kind: text, table, image, or none.
    render_kind: RenderKind,
    /// Position (x, y) for rendering.
    position: Option<(TokenStream, TokenStream)>,
    /// Text-specific attributes (font, size).
    text_attrs: TokenStream,
    /// PDF form field name (`#[pdf(field = "...")]`).
    field_name: Option<String>,
    /// Display order (`#[pdf(order = N)]`).
    order: Option<u32>,
    /// Whether this field is skipped (`#[pdf(ignore)]` or `#[pdf(skip)]`).
    skip: bool,
    /// Default value expression (`#[pdf(default = "...")]`).
    default_value: Option<String>,
    /// Whether this field is required (`#[pdf(required)]`).
    required: bool,
    /// Format pattern (`#[pdf(format = "...")]`).
    format: Option<String>,
    /// Whether this field is a nested model (`#[pdf(nested)]`).
    nested: bool,
}

enum RenderKind {
    None,
    Text,
    Table,
    Image,
}

fn get_named_fields(input: &DeriveInput) -> Result<&syn::punctuated::Punctuated<syn::Field, syn::Token![,]>> {
    match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => Ok(&named.named),
            _ => Err(Error::new_spanned(input, "PdfModel requires named fields")),
        },
        _ => Err(Error::new_spanned(input, "PdfModel only supports structs")),
    }
}

fn parse_field_attrs(field: &syn::Field) -> Result<ParsedField> {
    let field_ident = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "unnamed fields not supported"))?
        .clone();

    let mut render_kind = RenderKind::None;
    let mut position: Option<(TokenStream, TokenStream)> = None;
    let mut text_attrs = TokenStream::new();
    let mut field_name: Option<String> = None;
    let mut order: Option<u32> = None;
    let mut skip = false;
    let mut default_value: Option<String> = None;
    let mut required = false;
    let mut format: Option<String> = None;
    let mut nested = false;

    for attr in &field.attrs {
        if !attr.path().is_ident("pdf") {
            continue;
        }

        // First, do a quick scan for bare identifiers (ignore, skip, text, table, image, required, nested)
        if let Ok(list) = attr.meta.require_list() {
            for token in list.tokens.clone() {
                if let proc_macro2::TokenTree::Ident(ref ident) = token {
                    match ident.to_string().as_str() {
                        "ignore" | "skip" => skip = true,
                        "text" => render_kind = RenderKind::Text,
                        "table" => render_kind = RenderKind::Table,
                        "image" => render_kind = RenderKind::Image,
                        "required" => required = true,
                        "nested" => nested = true,
                        _ => {}
                    }
                }
            }
        }

        // Then parse key = value pairs
        attr.parse_nested_meta(|meta| {
            let ident_str = meta.path.get_ident().map(syn::Ident::to_string);

            if meta.path.is_ident("position") {
                let content: syn::ExprTuple = meta.value()?.parse()?;
                if content.elems.len() == 2 {
                    let x_expr = &content.elems[0];
                    let y_expr = &content.elems[1];
                    position = Some((
                        quote! { (#x_expr) as f64 },
                        quote! { (#y_expr) as f64 },
                    ));
                }
            } else if meta.path.is_ident("field") {
                let value: syn::LitStr = meta.value()?.parse()?;
                field_name = Some(value.value());
            } else if meta.path.is_ident("order") {
                let value: syn::LitInt = meta.value()?.parse()?;
                order = Some(value.base10_parse::<u32>()?);
            } else if meta.path.is_ident("default") {
                let value: syn::LitStr = meta.value()?.parse()?;
                default_value = Some(value.value());
            } else if meta.path.is_ident("format") {
                let value: syn::LitStr = meta.value()?.parse()?;
                format = Some(value.value());
            } else if meta.path.is_ident("font") {
                let value: syn::Expr = meta.value()?.parse()?;
                text_attrs.extend(quote! { .font(#value) });
            } else if meta.path.is_ident("size") {
                let value: syn::Expr = meta.value()?.parse()?;
                let core = core_crate();
                text_attrs.extend(quote! { .font(#core::PdfFont::helvetica(#value as f64)) });
            } else if let Some(id) = ident_str {
                // Consume the value for known attributes that we already handled via bare scan
                match id.as_str() {
                    "text" | "table" | "image" | "ignore" | "skip" | "required" | "nested" => {
                        // These are bare identifiers without values; if they appear
                        // with a value somehow, just consume it.
                        if meta.input.peek(syn::Token![=]) {
                            let _: syn::Expr = meta.value()?.parse()?;
                        }
                    }
                    _ => {
                        return Err(Error::new_spanned(
                            &meta.path,
                            format!("unknown pdf attribute: `{id}`. Expected one of: field, order, skip, default, required, format, nested, text, table, image, ignore, position, font, size"),
                        ));
                    }
                }
            }
            Ok(())
        })?;
    }

    // Validate: skip and field are mutually exclusive
    if skip && field_name.is_some() {
        return Err(Error::new_spanned(
            field_ident,
            "field cannot be both skipped (`ignore`/`skip`) and mapped (`field = ...`)",
        ));
    }

    // Validate: required without field name is a warning (but we allow it)
    // Validate: nested with text/table/image doesn't make sense
    if nested && !matches!(render_kind, RenderKind::None) {
        return Err(Error::new_spanned(
            field_ident,
            "`nested` cannot be combined with `text`, `table`, or `image`",
        ));
    }

    Ok(ParsedField {
        ident: field_ident,
        ty: field.ty.clone(),
        render_kind,
        position,
        text_attrs,
        field_name,
        order,
        skip,
        default_value,
        required,
        format,
        nested,
    })
}

// ============================================================================
// Code generation: render()
// ============================================================================

fn generate_render_arms(fields: &[ParsedField], core: &TokenStream) -> Result<TokenStream> {
    let mut arms = TokenStream::new();

    for pf in fields {
        // Skip fields marked as skip/ignore
        if pf.skip {
            continue;
        }

        let field_name = &pf.ident;

        match &pf.render_kind {
            RenderKind::Text => {
                let (x, y) = pf.position.clone().unwrap_or_else(|| {
                    (quote! { 100.0_f64 }, quote! { 700.0_f64 })
                });
                let text_attrs = &pf.text_attrs;
                arms.extend(quote! {
                    elements.push(#core::RenderedElement::Text {
                        x: #x,
                        y: #y,
                        text: #core::PdfText::new(self.#field_name.clone())
                            #text_attrs,
                    });
                });
            }
            RenderKind::Table => {
                let (x, y) = pf.position.clone().unwrap_or_else(|| {
                    (quote! { 72.0_f64 }, quote! { 700.0_f64 })
                });
                arms.extend(quote! {
                    elements.push(#core::RenderedElement::Table {
                        x: #x,
                        y: #y,
                        table: self.#field_name.clone(),
                    });
                });
            }
            RenderKind::Image => {
                let (x, y) = pf.position.clone().unwrap_or_else(|| {
                    (quote! { 72.0_f64 }, quote! { 700.0_f64 })
                });
                arms.extend(quote! {
                    elements.push(#core::RenderedElement::Image {
                        x: #x,
                        y: #y,
                        image: self.#field_name.clone(),
                    });
                });
            }
            RenderKind::None => {
                // For nested models, render the inner model's elements
                if pf.nested {
                    arms.extend(quote! {
                        elements.extend(self.#field_name.render()?);
                    });
                }
                // For fields with `field` attribute but no render kind,
                // they are form-field-mapped only -- no visual rendering.
            }
        }
    }

    Ok(arms)
}

// ============================================================================
// Code generation: field_descriptors()
// ============================================================================

fn generate_field_descriptors(fields: &[ParsedField], core: &TokenStream) -> TokenStream {
    let mut body = TokenStream::new();

    for pf in fields {
        // Skip fields with no form-related attributes
        if pf.skip && pf.field_name.is_none() {
            continue;
        }
        if pf.field_name.is_none() && !pf.required && pf.default_value.is_none() && !pf.nested {
            continue;
        }

        let rust_name = pf.ident.to_string();
        let pdf_field = pf
            .field_name
            .clone()
            .unwrap_or_else(|| rust_name.clone());
        let order_val = pf.order.unwrap_or(u32::MAX);
        let required_val = pf.required;
        let nested_val = pf.nested;

        let format_val = match &pf.format {
            Some(f) => quote! { Some(#f.to_string()) },
            None => quote! { None },
        };
        let default_val = match &pf.default_value {
            Some(d) => quote! { Some(#d.to_string()) },
            None => quote! { None },
        };

        body.extend(quote! {
            descriptors.push(#core::PdfFieldDescriptor {
                field_name: #pdf_field.to_string(),
                rust_field_name: #rust_name.to_string(),
                order: #order_val,
                format: #format_val,
                default_value: #default_val,
                required: #required_val,
                nested: #nested_val,
            });
        });
    }

    body
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse a string of Rust item into a `DeriveInput`.
    fn parse_derive_input(src: &str) -> DeriveInput {
        syn::parse_str(src).expect("failed to parse DeriveInput")
    }

    // ---- core_crate() ----

    #[test]
    fn core_crate_returns_ident() {
        let ts = core_crate();
        let s = ts.to_string();
        // In test context, proc_macro_crate can't find the crate, so fallback
        assert_eq!(s, "easypdf_core");
    }

    // ---- get_named_fields() ----

    #[test]
    fn get_named_fields_rejects_enum() {
        let input = parse_derive_input("enum Foo { Bar }");
        let result = get_named_fields(&input);
        assert!(result.is_err());
    }

    #[test]
    fn get_named_fields_rejects_tuple_struct() {
        let input = parse_derive_input("struct Foo(String);");
        let result = get_named_fields(&input);
        assert!(result.is_err());
    }

    #[test]
    fn get_named_fields_accepts_named_struct() {
        let input = parse_derive_input("struct Foo { x: i32 }");
        let result = get_named_fields(&input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    // ---- parse_struct_attrs() ----

    #[test]
    fn parse_struct_attrs_defaults() {
        let input = parse_derive_input("struct Foo { x: i32 }");
        let result = parse_struct_attrs(&input.attrs).unwrap();
        // Check defaults compile (we can't easily compare TokenStreams for equality)
        assert!(!result.page_size.is_empty());
        assert!(!result.orientation.is_empty());
        assert!(!result.margins.is_empty());
    }

    #[test]
    fn parse_struct_attrs_with_page() {
        let input = parse_derive_input(
            r#"#[pdf(page = easypdf_core::PageSize::A3)]
            struct Foo { x: i32 }"#,
        );
        let result = parse_struct_attrs(&input.attrs).unwrap();
        assert!(!result.page_size.is_empty());
    }

    #[test]
    fn parse_struct_attrs_with_orientation() {
        let input = parse_derive_input(
            r#"#[pdf(orientation = easypdf_core::Orientation::Landscape)]
            struct Foo { x: i32 }"#,
        );
        let result = parse_struct_attrs(&input.attrs).unwrap();
        assert!(!result.orientation.is_empty());
    }

    #[test]
    fn parse_struct_attrs_with_margins() {
        let input = parse_derive_input(r#"#[pdf(margins = 36.0_f64)] struct Foo { x: i32 }"#);
        let result = parse_struct_attrs(&input.attrs).unwrap();
        assert!(!result.margins.is_empty());
    }

    #[test]
    fn parse_struct_attrs_ignores_non_pdf_attrs() {
        let input = parse_derive_input(
            r#"#[derive(Debug)]
            struct Foo { x: i32 }"#,
        );
        let result = parse_struct_attrs(&input.attrs).unwrap();
        assert!(!result.page_size.is_empty());
    }

    // ---- parse_field_attrs() ----

    #[test]
    fn parse_field_no_attrs() {
        let input = parse_derive_input("struct Foo { x: i32 }");
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(!pf.skip);
        assert!(pf.field_name.is_none());
        assert!(pf.position.is_none());
        assert!(pf.order.is_none());
        assert!(pf.default_value.is_none());
        assert!(!pf.required);
        assert!(pf.format.is_none());
        assert!(!pf.nested);
        assert!(matches!(pf.render_kind, RenderKind::None));
    }

    #[test]
    fn parse_field_text_with_position() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(text, position = (100, 700))]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(matches!(pf.render_kind, RenderKind::Text));
        assert!(pf.position.is_some());
    }

    #[test]
    fn parse_field_table() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(table, position = (72, 500))]
                data: Vec<Vec<String>>,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(matches!(pf.render_kind, RenderKind::Table));
    }

    #[test]
    fn parse_field_image() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(image, position = (72, 500))]
                logo: Vec<u8>,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(matches!(pf.render_kind, RenderKind::Image));
    }

    #[test]
    fn parse_field_ignore() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(ignore)]
                internal: i32,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(pf.skip);
    }

    #[test]
    fn parse_field_skip_alias() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(skip)]
                internal: i32,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(pf.skip);
    }

    #[test]
    fn parse_field_field_name() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(field = "pdf_title")]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert_eq!(pf.field_name.as_deref(), Some("pdf_title"));
    }

    #[test]
    fn parse_field_order() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(order = 5)]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert_eq!(pf.order, Some(5));
    }

    #[test]
    fn parse_field_default() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(default = "N/A")]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert_eq!(pf.default_value.as_deref(), Some("N/A"));
    }

    #[test]
    fn parse_field_required() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(required)]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(pf.required);
    }

    #[test]
    fn parse_field_format() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(format = "YYYY-MM-DD")]
                date: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert_eq!(pf.format.as_deref(), Some("YYYY-MM-DD"));
    }

    #[test]
    fn parse_field_nested() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(nested)]
                inner: Bar,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(pf.nested);
    }

    #[test]
    fn parse_field_text_with_size() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(text, size = 14)]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let pf = parse_field_attrs(&fields[0]).unwrap();
        assert!(matches!(pf.render_kind, RenderKind::Text));
        assert!(!pf.text_attrs.is_empty());
    }

    // ---- Validation errors ----

    #[test]
    fn skip_and_field_mutually_exclusive() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(skip, field = "x")]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let result = parse_field_attrs(&fields[0]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("skipped"));
        }
    }

    #[test]
    fn nested_with_text_rejected() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(nested, text)]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let result = parse_field_attrs(&fields[0]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("nested"));
        }
    }

    #[test]
    fn nested_with_table_rejected() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(nested, table)]
                data: Vec<String>,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let result = parse_field_attrs(&fields[0]);
        assert!(result.is_err());
    }

    #[test]
    fn nested_with_image_rejected() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(nested, image)]
                img: Vec<u8>,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let result = parse_field_attrs(&fields[0]);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_attribute_rejected() {
        let input = parse_derive_input(
            r#"struct Foo {
                #[pdf(bogus = "value")]
                title: String,
            }"#,
        );
        let fields = get_named_fields(&input).unwrap();
        let result = parse_field_attrs(&fields[0]);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("unknown pdf attribute"));
        }
    }

    // ---- expand_pdf_model() ----

    #[test]
    fn expand_simple_text_struct() {
        let input: TokenStream = quote! {
            struct Invoice {
                #[pdf(text, position = (100, 700))]
                title: String,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
        let expanded = result.unwrap().to_string();
        assert!(expanded.contains("impl"));
        assert!(expanded.contains("PdfModel"));
        assert!(expanded.contains("render"));
        assert!(expanded.contains("metadata"));
        assert!(expanded.contains("field_descriptors"));
    }

    #[test]
    fn expand_struct_with_ignore_field() {
        let input: TokenStream = quote! {
            struct Invoice {
                #[pdf(text, position = (100, 700))]
                title: String,
                #[pdf(ignore)]
                internal: i32,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_table_field() {
        let input: TokenStream = quote! {
            struct Report {
                #[pdf(table, position = (72, 500))]
                data: Vec<Vec<String>>,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_image_field() {
        let input: TokenStream = quote! {
            struct Doc {
                #[pdf(image, position = (72, 500))]
                logo: Vec<u8>,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_nested_field() {
        let input: TokenStream = quote! {
            struct Outer {
                #[pdf(nested)]
                inner: Inner,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_field_descriptor() {
        let input: TokenStream = quote! {
            struct Form {
                #[pdf(field = "name", required, default = "unknown", order = 1, format = "text")]
                name: String,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
        let expanded = result.unwrap().to_string();
        assert!(expanded.contains("field_descriptors"));
    }

    #[test]
    fn expand_struct_with_text_no_position() {
        // Text field without explicit position uses default
        let input: TokenStream = quote! {
            struct Doc {
                #[pdf(text)]
                title: String,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_table_no_position() {
        let input: TokenStream = quote! {
            struct Doc {
                #[pdf(table)]
                data: Vec<Vec<String>>,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_image_no_position() {
        let input: TokenStream = quote! {
            struct Doc {
                #[pdf(image)]
                img: Vec<u8>,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_struct_with_struct_attrs() {
        let input: TokenStream = quote! {
            #[pdf(page = easypdf_core::PageSize::A3, orientation = easypdf_core::Orientation::Landscape, margins = 36.0)]
            struct Doc {
                #[pdf(text, position = (100, 700))]
                title: String,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_enum_fails() {
        let input: TokenStream = quote! {
            enum Foo { Bar }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_err());
    }

    #[test]
    fn expand_tuple_struct_fails() {
        let input: TokenStream = quote! {
            struct Foo(String);
        };
        let result = expand_pdf_model(input);
        assert!(result.is_err());
    }

    #[test]
    fn expand_empty_struct() {
        let input: TokenStream = quote! {
            struct Empty {}
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    #[test]
    fn expand_multiple_fields_mixed() {
        let input: TokenStream = quote! {
            struct Invoice {
                #[pdf(text, position = (100, 700))]
                title: String,
                #[pdf(table, position = (72, 500))]
                items: Vec<Vec<String>>,
                #[pdf(ignore)]
                internal: i32,
                #[pdf(field = "total", required)]
                total: String,
                #[pdf(image, position = (400, 100))]
                qr: Vec<u8>,
            }
        };
        let result = expand_pdf_model(input);
        assert!(result.is_ok());
    }

    // ---- generate_render_arms() ----

    #[test]
    fn render_arms_skip_field() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
            ty: syn::parse_str("i32").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: true,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_render_arms(&fields, &core);
        assert!(result.is_ok());
        // Should produce empty output since field is skipped
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn render_arms_text_field_default_position() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("title", proc_macro2::Span::call_site()),
            ty: syn::parse_str("String").unwrap(),
            render_kind: RenderKind::Text,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_render_arms(&fields, &core).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn render_arms_table_field_default_position() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("data", proc_macro2::Span::call_site()),
            ty: syn::parse_str("Vec<Vec<String>>").unwrap(),
            render_kind: RenderKind::Table,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_render_arms(&fields, &core).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn render_arms_image_field_default_position() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("img", proc_macro2::Span::call_site()),
            ty: syn::parse_str("Vec<u8>").unwrap(),
            render_kind: RenderKind::Image,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_render_arms(&fields, &core).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn render_arms_nested_field() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("inner", proc_macro2::Span::call_site()),
            ty: syn::parse_str("Inner").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: true,
        }];
        let core = core_crate();
        let result = generate_render_arms(&fields, &core).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn render_arms_none_kind_no_nested() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
            ty: syn::parse_str("i32").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: Some("x_field".to_string()),
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_render_arms(&fields, &core).unwrap();
        // None kind without nested produces no render code
        assert!(result.is_empty());
    }

    // ---- generate_field_descriptors() ----

    #[test]
    fn descriptors_skip_no_field_name() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
            ty: syn::parse_str("i32").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: true,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(result.is_empty());
    }

    #[test]
    fn descriptors_skip_with_field_name() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
            ty: syn::parse_str("i32").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: Some("pdf_x".to_string()),
            order: None,
            skip: true,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(!result.is_empty());
    }

    #[test]
    fn descriptors_required_field() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("name", proc_macro2::Span::call_site()),
            ty: syn::parse_str("String").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: true,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(!result.is_empty());
    }

    #[test]
    fn descriptors_field_with_default() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("name", proc_macro2::Span::call_site()),
            ty: syn::parse_str("String").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: Some("N/A".to_string()),
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(!result.is_empty());
    }

    #[test]
    fn descriptors_nested_field() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("inner", proc_macro2::Span::call_site()),
            ty: syn::parse_str("Inner").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: true,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(!result.is_empty());
    }

    #[test]
    fn descriptors_with_order() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("title", proc_macro2::Span::call_site()),
            ty: syn::parse_str("String").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: Some("pdf_title".to_string()),
            order: Some(3),
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(!result.is_empty());
    }

    #[test]
    fn descriptors_with_format() {
        let fields = vec![ParsedField {
            ident: syn::Ident::new("date", proc_macro2::Span::call_site()),
            ty: syn::parse_str("String").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: Some("pdf_date".to_string()),
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: Some("YYYY-MM-DD".to_string()),
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(!result.is_empty());
    }

    #[test]
    fn descriptors_no_attrs_skipped() {
        // Field with no form-related attributes should be skipped
        let fields = vec![ParsedField {
            ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
            ty: syn::parse_str("i32").unwrap(),
            render_kind: RenderKind::None,
            position: None,
            text_attrs: TokenStream::new(),
            field_name: None,
            order: None,
            skip: false,
            default_value: None,
            required: false,
            format: None,
            nested: false,
        }];
        let core = core_crate();
        let result = generate_field_descriptors(&fields, &core);
        assert!(result.is_empty());
    }
}
