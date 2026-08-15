mod codegen_tests;
mod parse_tests;

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::codegen::{generate_field_descriptors, generate_render_arms};
use super::expand::{core_crate, expand_pdf_model};
use super::model::{
    ParsedField, RenderKind, get_named_fields, parse_field_attrs, parse_struct_attrs,
};

/// 辅助函数：将 Rust 源码字符串解析为 `DeriveInput`。
fn parse_derive_input(src: &str) -> DeriveInput {
    syn::parse_str(src).expect("failed to parse DeriveInput")
}
