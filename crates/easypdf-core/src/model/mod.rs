//! 与引擎无关的语义文档模型。
//!
//! [`PdfBlock`] 是模型的核心枚举，涵盖标题、段落、
//! 列表、表格、图片、代码、公式、脚注、引用及其他语义块。
//! 通过 [`PdfPageModel`] 按页组织，聚合为 [`PdfDocumentModel`]。

mod image_data;
mod list_item;
mod pdf_block;
mod pdf_block_type;
mod pdf_document_model;
mod pdf_page_model;
mod source_location;

pub use image_data::{ImageData, ImageFormat};
pub use list_item::ListItem;
pub use pdf_block::PdfBlock;
pub use pdf_block_type::PdfBlockType;
pub use pdf_document_model::PdfDocumentModel;
pub use pdf_page_model::PdfPageModel;
pub use source_location::SourceLocation;
