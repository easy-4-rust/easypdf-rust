//! `PdfModel` trait 的过程宏 derive。
//!
//! 提供 `#[derive(PdfModel)]`，生成将 Rust 结构体字段映射到
//! PDF 内容元素的编译期反射代码。
//!
//! ## 用法
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
//! ## 字段属性
//!
//! | 属性 | 说明 |
//! |---|---|
//! | `#[pdf(text, position = (x, y))]` | 将字段渲染为定位文本 |
//! | `#[pdf(table, position = (x, y))]` | 将字段渲染为表格 |
//! | `#[pdf(image, position = (x, y))]` | 将字段渲染为图片 |
//! | `#[pdf(ignore)]` / `#[pdf(skip)]` | 完全跳过字段 |
//! | `#[pdf(field = "name")]` | 映射到 PDF 表单字段名 |
//! | `#[pdf(order = N)]` | 显示/渲染顺序 |
//! | `#[pdf(default = "value")]` | 为空时的默认值 |
//! | `#[pdf(required)]` | 字段必须非空 |
//! | `#[pdf(format = "pattern")]` | 格式模式（如 `"YYYY-MM-DD"`） |
//! | `#[pdf(nested)]` | 递归包含内部模型的元素 |
//! | `#[pdf(font = ...)]` | 设置文本渲染字体 |
//! | `#[pdf(size = N)]` | 设置文本渲染字号 |

use proc_macro::TokenStream;

mod implementation;

/// 生成 [`PdfModel`] trait 实现的 derive 宏。
///
/// # 属性
///
/// - `#[pdf(page = ..., orientation = ..., margins = ...)]` -- 结构体级
/// - `#[pdf(text, position = (x, y), font = ...)]` -- 文本字段
/// - `#[pdf(table, position = (x, y), headers = [...])]` -- 集合字段
/// - `#[pdf(field = "field_name")]` -- 表单/模板字段映射
/// - `#[pdf(order = N)]` -- 显示顺序
/// - `#[pdf(ignore)]` / `#[pdf(skip)]` -- 跳过字段
/// - `#[pdf(default = "value")]` -- 默认值
/// - `#[pdf(required)]` -- 标记字段为必填
/// - `#[pdf(format = "pattern")]` -- 格式模式
/// - `#[pdf(nested)]` -- 递归渲染内部模型
#[proc_macro_derive(PdfModel, attributes(pdf))]
pub fn derive_pdf_model(input: TokenStream) -> TokenStream {
    implementation::expand_pdf_model(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
