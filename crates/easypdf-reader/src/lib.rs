//! PDF reading and text extraction (lopdf backend).
//!
//! Provides [`PdfReader`] for parsing PDF documents and extracting text,
//! metadata, and page information.
//!
//! # Reading strategies
//!
//! The [`ReadStrategy`] enum selects how the PDF is parsed:
//!
//! - [`Full`](ReadStrategy::Full) -- loads the entire document into memory
//!   (default, best for small files).
//! - [`Lazy`](ReadStrategy::Lazy) -- parses the page tree only; page content
//!   is loaded on demand (best for large files).
//! - [`Streaming`](ReadStrategy::Streaming) -- scans content streams without
//!   building a full object tree (best for very large files or constrained
//!   environments).
//!
//! Use [`ReadStrategy::auto`] to pick the optimal strategy based on file size,
//! or pass an explicit strategy to [`PdfReader::open_with_strategy`].
//!
//! # Examples
//!
//! ```no_run
//! use easypdf_reader::{PdfReader, ReadStrategy};
//!
//! // Auto-select strategy based on file size:
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
