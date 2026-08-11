//! Engine-neutral semantic document model.
//!
//! [`PdfBlock`] is the core enum of the model, covering headings, paragraphs,
//! lists, tables, images, code, formulas, footnotes, quotes and other semantic blocks.
//! Organized by page via [`PdfPageModel`], aggregated into [`PdfDocumentModel`].

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
