//! `render()` 和 `field_descriptors()` 的代码生成。

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use super::model::{ParsedField, RenderKind};

// ============================================================================
// Code generation: render()
// ============================================================================

pub(super) fn generate_render_arms(
    fields: &[ParsedField],
    core: &TokenStream,
) -> Result<TokenStream> {
    let mut arms = TokenStream::new();

    for pf in fields {
        // Skip fields marked as skip/ignore
        if pf.skip {
            continue;
        }

        let field_name = &pf.ident;

        match &pf.render_kind {
            RenderKind::Text => {
                let (x, y) = pf
                    .position
                    .clone()
                    .unwrap_or_else(|| (quote! { 100.0_f64 }, quote! { 700.0_f64 }));
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
                let (x, y) = pf
                    .position
                    .clone()
                    .unwrap_or_else(|| (quote! { 72.0_f64 }, quote! { 700.0_f64 }));
                arms.extend(quote! {
                    elements.push(#core::RenderedElement::Table {
                        x: #x,
                        y: #y,
                        table: self.#field_name.clone(),
                    });
                });
            }
            RenderKind::Image => {
                let (x, y) = pf
                    .position
                    .clone()
                    .unwrap_or_else(|| (quote! { 72.0_f64 }, quote! { 700.0_f64 }));
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

pub(super) fn generate_field_descriptors(
    fields: &[ParsedField],
    core: &TokenStream,
) -> TokenStream {
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
        let pdf_field = pf.field_name.clone().unwrap_or_else(|| rust_name.clone());
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
