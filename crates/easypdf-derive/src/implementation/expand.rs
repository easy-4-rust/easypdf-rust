//! `#[derive(PdfModel)]` 的展开入口与辅助函数。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::implementation::codegen::{generate_field_descriptors, generate_render_arms};
use crate::implementation::model::{
    ParsedField, PdfStructAttrs, get_named_fields, parse_field_attrs, parse_struct_attrs,
};

/// 在编译期解析 `easypdf_core` 的 crate 名称。
pub(crate) fn core_crate() -> TokenStream {
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

/// 入口函数：将 `#[derive(PdfModel)]` 展开为 trait 实现。
pub(crate) fn expand_pdf_model(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    let core = core_crate();

    // 解析结构体级别的 #[pdf(...)] 属性
    let PdfStructAttrs {
        page_size,
        orientation,
        margins,
    } = parse_struct_attrs(&input.attrs)?;

    // 解析所有字段及其属性
    let fields = get_named_fields(&input)?;
    let parsed_fields: Vec<ParsedField> = fields
        .iter()
        .map(parse_field_attrs)
        .collect::<Result<Vec<_>>>()?;

    // 生成字段渲染代码
    let render_arms = generate_render_arms(&parsed_fields, &core)?;

    // 生成 field_descriptors 代码
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
