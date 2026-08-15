//! PDF 读取与文本提取（lopdf 后端）。
//!
//! 提供 [`PdfReader`] 用于解析 PDF 文档并提取文本、元数据和页面信息。
//!
//! # 读取策略
//!
//! [`ReadStrategy`] 枚举选择 PDF 的解析方式：
//!
//! - [`Full`](ReadStrategy::Full) -- 将整个文档加载到内存
//!   （默认，适合小型文件）。
//! - [`Lazy`](ReadStrategy::Lazy) -- 仅解析页面树；页面内容按需加载
//!   （适合大型文件）。
//! - [`Streaming`](ReadStrategy::Streaming) -- 扫描内容流而不构建完整的
//!   对象树（适合超大型文件或受限环境）。
//!
//! 使用 [`ReadStrategy::auto`] 根据文件大小自动选择最优策略，
//! 或向 [`PdfReader::open_with_strategy`] 传入显式策略。
//!
//! # Examples
//!
//! ```no_run
//! use easypdf_reader::{PdfReader, ReadStrategy};
//!
//! // 根据文件大小自动选择策略：
//! let reader = PdfReader::open_with_strategy("large.pdf", ReadStrategy::Lazy)?;
//! let text = reader.extract_text()?;
//! # Ok::<(), easypdf_core::PdfError>(())
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]
#![allow(clippy::uninlined_format_args, clippy::manual_string_new)]
#![cfg_attr(test, allow(clippy::similar_names))]

mod manipulate;
mod reader;
mod strategy;
mod streaming;

pub use manipulate::PdfManipulator;
pub use reader::PdfReader;
pub use strategy::ReadStrategy;
