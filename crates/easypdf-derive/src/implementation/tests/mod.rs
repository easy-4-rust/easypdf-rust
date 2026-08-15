mod codegen_tests;
mod parse_tests;

use super::model::RenderKind;
use super::*;

/// Helper: parse a string of Rust item into a `DeriveInput`.
fn parse_derive_input(src: &str) -> DeriveInput {
    syn::parse_str(src).expect("failed to parse DeriveInput")
}
