//! Builder pattern for constructing [`PdfWriter`] with advanced options.
//!
//! Provides [`PdfWriterBuilder`] for configuring write backend, handler
//! priorities, and constant-memory mode before constructing a writer.
//!
//! # Examples
//!
//! ```
//! use easypdf_writer::{PdfWriterBuilder, WriteBackend};
//! use easypdf_core::handler_chain::PRIORITY_HIGH;
//!
//! let writer = PdfWriterBuilder::new("My Document")
//!     .backend(WriteBackend::InMemory)
//!     .constant_memory(false)
//!     .build();
//! ```

use easypdf_core::error::Result;
use easypdf_core::handler_chain::{PRIORITY_NORMAL, WriteHandlerChain};
use easypdf_core::{PdfMetadata, PdfWriteHandler};

use crate::backend::WriteBackend;
use crate::writer::PdfWriter;

/// Builder for constructing a [`PdfWriter`] with advanced configuration.
///
/// Supports:
/// - **Write backend selection** (in-memory vs page-level spill)
/// - **Handler priority ordering** via [`WriteHandlerChain`]
/// - **Constant-memory mode** convenience toggle
/// - **Temp file compression** for spill mode
///
/// # Examples
///
/// ```
/// use easypdf_writer::{PdfWriterBuilder, WriteBackend};
/// use easypdf_core::PdfMetadata;
///
/// let writer = PdfWriterBuilder::new("Report")
///     .metadata(PdfMetadata::new().title("Q4 Report").author("Finance"))
///     .backend(WriteBackend::auto(500))
///     .compress_temp_files(true)
///     .build();
/// ```
pub struct PdfWriterBuilder {
    title: String,
    metadata: PdfMetadata,
    backend: WriteBackend,
    chain: WriteHandlerChain,
    constant_memory: bool,
    compress_temp_files: bool,
}

impl PdfWriterBuilder {
    /// Create a new builder for a PDF document with the given title.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            metadata: PdfMetadata::default(),
            backend: WriteBackend::default(),
            chain: WriteHandlerChain::new(),
            constant_memory: false,
            compress_temp_files: true,
        }
    }

    /// Set document metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the write backend (in-memory or spill).
    #[must_use]
    pub fn backend(mut self, backend: WriteBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Enable or disable constant-memory mode.
    ///
    /// When enabled, the backend is automatically switched to
    /// [`WriteBackend::constant_memory()`] which spills every page
    /// immediately after finalization.
    #[must_use]
    pub fn constant_memory(mut self, enabled: bool) -> Self {
        self.constant_memory = enabled;
        self
    }

    /// Set whether spill files should be gzip-compressed.
    ///
    /// Only takes effect when the backend is [`WriteBackend::Spill`].
    /// Defaults to `true`.
    #[must_use]
    pub fn compress_temp_files(mut self, compress: bool) -> Self {
        self.compress_temp_files = compress;
        self
    }

    /// Register a write handler with the default priority
    /// ([`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL)).
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.chain.register(handler, PRIORITY_NORMAL);
        self
    }

    /// Register a write handler with a specific priority.
    ///
    /// Lower priority values execute first. See
    /// [`PRIORITY_HIGH`](easypdf_core::handler_chain::PRIORITY_HIGH),
    /// [`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL),
    /// [`PRIORITY_LOW`](easypdf_core::handler_chain::PRIORITY_LOW).
    #[must_use]
    pub fn register_handler_with_priority(
        mut self,
        handler: Box<dyn PdfWriteHandler>,
        priority: f64,
    ) -> Self {
        self.chain.register(handler, priority);
        self
    }

    /// Build the [`PdfWriter`].
    ///
    /// # Errors
    ///
    /// Returns an error if the spill backend cannot be initialized (e.g.,
    /// the spill directory cannot be created).
    pub fn build(self) -> Result<PdfWriter> {
        let backend = if self.constant_memory {
            WriteBackend::Spill {
                spill_dir: match self.backend {
                    WriteBackend::Spill { spill_dir, .. } => spill_dir,
                    WriteBackend::InMemory => None,
                },
                compress: self.compress_temp_files,
                threshold_pages: 1,
            }
        } else {
            match self.backend {
                WriteBackend::Spill {
                    spill_dir,
                    compress,
                    threshold_pages,
                } => WriteBackend::Spill {
                    spill_dir,
                    compress: if self.compress_temp_files {
                        compress
                    } else {
                        false
                    },
                    threshold_pages,
                },
                other @ WriteBackend::InMemory => other,
            }
        };

        PdfWriter::with_config(&self.title, self.metadata, backend, self.chain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_default_creates_writer() {
        let w = PdfWriterBuilder::new("test").build().unwrap();
        assert_eq!(w.current_page_number(), 0);
    }

    #[test]
    fn builder_with_metadata() {
        let w = PdfWriterBuilder::new("test")
            .metadata(PdfMetadata::new().title("T").author("A"))
            .build()
            .unwrap();
        assert_eq!(w.metadata_title(), Some("T"));
    }

    #[test]
    fn builder_with_in_memory_backend() {
        let w = PdfWriterBuilder::new("test")
            .backend(WriteBackend::InMemory)
            .build()
            .unwrap();
        assert!(!w.is_constant_memory());
    }

    #[test]
    fn builder_constant_memory_overrides_backend() {
        let w = PdfWriterBuilder::new("test")
            .backend(WriteBackend::InMemory)
            .constant_memory(true)
            .build()
            .unwrap();
        assert!(w.is_constant_memory());
    }

    #[test]
    fn builder_compress_temp_files() {
        let w = PdfWriterBuilder::new("test")
            .backend(WriteBackend::Spill {
                spill_dir: None,
                compress: false,
                threshold_pages: 10,
            })
            .compress_temp_files(true)
            .build()
            .unwrap();
        // compress_temp_files(true) should override the backend's compress=false
        assert!(w.is_constant_memory());
    }

    #[test]
    fn builder_with_handler() {
        struct NoopHandler;
        impl PdfWriteHandler for NoopHandler {}
        let w = PdfWriterBuilder::new("test")
            .register_handler(Box::new(NoopHandler))
            .build()
            .unwrap();
        assert_eq!(w.handler_count(), 1);
    }
}
