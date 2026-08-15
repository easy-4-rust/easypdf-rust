//! `#[derive(PdfModel)]` 的结构体级和字段级属性解析。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Result};

// ============================================================================
// Struct-level attributes
// ============================================================================

pub(super) struct PdfStructAttrs {
    pub page_size: TokenStream,
    pub orientation: TokenStream,
    pub margins: TokenStream,
}

pub(super) fn parse_struct_attrs(attrs: &[syn::Attribute]) -> Result<PdfStructAttrs> {
    let core = super::core_crate();
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

/// 单个字段的所有已解析属性。
pub(super) struct ParsedField {
    /// The Rust field identifier.
    pub ident: syn::Ident,
    /// The type of the field (reserved for future validation).
    #[allow(dead_code)]
    pub ty: syn::Type,
    /// Rendering kind: text, table, image, or none.
    pub render_kind: RenderKind,
    /// Position (x, y) for rendering.
    pub position: Option<(TokenStream, TokenStream)>,
    /// Text-specific attributes (font, size).
    pub text_attrs: TokenStream,
    /// PDF form field name (`#[pdf(field = "...")]`).
    pub field_name: Option<String>,
    /// Display order (`#[pdf(order = N)]`).
    pub order: Option<u32>,
    /// Whether this field is skipped (`#[pdf(ignore)]` or `#[pdf(skip)]`).
    pub skip: bool,
    /// Default value expression (`#[pdf(default = "...")]`).
    pub default_value: Option<String>,
    /// Whether this field is required (`#[pdf(required)]`).
    pub required: bool,
    /// Format pattern (`#[pdf(format = "...")]`).
    pub format: Option<String>,
    /// Whether this field is a nested model (`#[pdf(nested)]`).
    pub nested: bool,
}

pub(super) enum RenderKind {
    None,
    Text,
    Table,
    Image,
}

pub(super) fn get_named_fields(
    input: &DeriveInput,
) -> Result<&syn::punctuated::Punctuated<syn::Field, syn::Token![,]>> {
    match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(named) => Ok(&named.named),
            _ => Err(Error::new_spanned(input, "PdfModel requires named fields")),
        },
        _ => Err(Error::new_spanned(input, "PdfModel only supports structs")),
    }
}

pub(super) fn parse_field_attrs(field: &syn::Field) -> Result<ParsedField> {
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
                let core = super::core_crate();
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
