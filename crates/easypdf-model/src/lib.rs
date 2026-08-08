//! 与具体 PDF 引擎无关的语义文档模型。

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

mod pdf_block;
mod pdf_document_model;
mod pdf_page_model;
mod source_location;

pub use pdf_block::PdfBlock;
pub use pdf_document_model::PdfDocumentModel;
pub use pdf_page_model::PdfPageModel;
pub use source_location::SourceLocation;
