//! PDF 写入引擎抽象层。
//!
//! 本模块将 PDF 写入操作与底层 PDF 库解耦：
//!
//! - [`op`]：引擎无关的中间操作表示（[`WriterOp`](op::WriterOp)）。
//! - [`write_engine`]：写入引擎抽象 trait（[`WriteEngine`](write_engine::WriteEngine)）。
//! - [`engine_kind`]：公共引擎选择器（[`WriteEngineKind`](engine_kind::WriteEngineKind)）。
//! - [`font_key`]：`PdfFont` 到 [`FontKey`](op::FontKey) 的解析。
//! - [`printpdf_engine`]：基于 printpdf 的引擎实现（默认引擎）。
//! - `krilla_engine`：基于 krilla 的引擎实现（`writer-krilla` feature）。

pub(crate) mod engine_kind;
pub(crate) mod font_key;
pub(crate) mod op;
mod printpdf_engine;
mod write_engine;

#[cfg(feature = "writer-krilla")]
pub(crate) mod krilla_engine;

pub use engine_kind::WriteEngineKind;
pub(crate) use font_key::resolve_font_key;
pub(crate) use op::{FontKey, PendingXObject, WriterOp};
pub(crate) use write_engine::WriteEngine;
