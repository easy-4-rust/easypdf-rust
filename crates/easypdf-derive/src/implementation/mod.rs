//! `#[derive(PdfModel)]` 过程宏的实现。
//!
//! 支持以下字段级属性：
//!
//! - `#[pdf(text, position = (x, y), font = ..., size = ...)]` -- 渲染为文本
//! - `#[pdf(table, position = (x, y))]` -- 渲染为表格
//! - `#[pdf(image, position = (x, y))]` -- 渲染为图片
//! - `#[pdf(ignore)]` -- 完全跳过字段（不渲染、不生成描述符）
//! - `#[pdf(skip)]` -- `ignore` 的别名
//! - `#[pdf(field = "pdf_field_name")]` -- PDF 表单字段映射
//! - `#[pdf(order = N)]` -- 显示/渲染顺序
//! - `#[pdf(default = "value")]` -- 字段为空时的默认值
//! - `#[pdf(required)]` -- 字段必须非空
//! - `#[pdf(format = "pattern")]` -- 格式模式（如 "YYYY-MM-DD"）
//! - `#[pdf(nested)]` -- 递归包含内部模型的元素

mod codegen;
pub(crate) mod expand;
mod model;
#[cfg(test)]
mod tests;

pub(crate) use expand::{core_crate, expand_pdf_model};
