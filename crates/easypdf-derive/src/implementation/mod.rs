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

mod codegen;
mod model;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use codegen::{generate_field_descriptors, generate_render_arms};
use model::{ParsedField, PdfStructAttrs, get_named_fields, parse_field_attrs, parse_struct_attrs};

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
