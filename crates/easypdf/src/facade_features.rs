//! Feature-gated facade methods for [`EasyPdf`].

#[cfg(any(feature = "render", feature = "resident"))]
use std::path::Path;

use crate::EasyPdf;

#[cfg(feature = "markdown")]
use crate::{MarkdownProfile, ProcessorPipeline};
#[cfg(feature = "markdown-table")]
use crate::TableDetectorProcessor;
#[cfg(feature = "render")]
use crate::RenderError;
#[cfg(feature = "mcp")]
use crate::McpServer;
#[cfg(feature = "resident")]
use crate::ResidentClient;

impl EasyPdf {
    /// Create a PDF Writer with backend selection.
    ///
    /// Returns a `PdfWriterBuilder` for configuring the write backend
    /// (`InMemory` or `Spill`), handler priorities, and constant-memory mode.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    /// use easypdf::WriteBackend;
    ///
    /// let writer = EasyPdf::writer("My Document")
    ///     .backend(WriteBackend::auto(500))
    ///     .build()?;
    /// ```
    #[must_use = "builder method"]
    pub fn writer(title: impl Into<String>) -> crate::PdfWriterBuilder {
        crate::PdfWriterBuilder::new(title)
    }

    /// Create a Markdown conversion pipeline.
    ///
    /// Returns a [`ProcessorPipeline`] pre-configured for the given profile.
    /// Register additional processors (table detector, OCR) before running.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    /// use easypdf::MarkdownProfile;
    ///
    /// let mut pipeline = EasyPdf::markdown_pipeline(MarkdownProfile::Gfm);
    /// // pipeline.register(Box::new(my_processor));
    /// ```
    #[cfg(feature = "markdown")]
    #[must_use = "builder method"]
    pub fn markdown_pipeline(profile: MarkdownProfile) -> ProcessorPipeline {
        let _ = profile;
        ProcessorPipeline::new()
    }

    /// Create a table detector processor.
    ///
    /// Returns a [`TableDetectorProcessor`] with default configuration.
    /// Register it into a [`ProcessorPipeline`] to enable table detection
    /// during markdown conversion.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    ///
    /// let detector = EasyPdf::table_detector();
    /// ```
    #[cfg(feature = "markdown-table")]
    #[must_use = "builder method"]
    pub fn table_detector() -> TableDetectorProcessor {
        TableDetectorProcessor::new()
    }

    /// Render a single PDF page to a PNG file.
    ///
    /// Uses the default render backend (text renderer or pdfium if available).
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if the page cannot be rendered or the output
    /// cannot be written.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    ///
    /// EasyPdf::render_page("input.pdf".as_ref(), 0, "page_0.png".as_ref(), 150)?;
    /// ```
    #[cfg(feature = "render")]
    pub fn render_page(
        pdf_path: &Path,
        page: usize,
        output: &Path,
        dpi: u32,
    ) -> std::result::Result<(), RenderError> {
        easypdf_markdown::render::render_page_to_png(pdf_path, page, output, dpi)
    }

    /// Launch the MCP (Model Context Protocol) server.
    ///
    /// The server reads JSON-RPC 2.0 requests from stdin and writes
    /// responses to stdout, exposing PDF operations to LLM agents.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    ///
    /// let server = EasyPdf::mcp_server();
    /// server.run()?;
    /// ```
    #[cfg(feature = "mcp")]
    #[must_use]
    pub fn mcp_server() -> McpServer {
        McpServer::new()
    }

    /// Start the resident daemon that keeps PDF documents open in memory.
    ///
    /// Binds to a Unix socket and waits for client connections.
    /// If `socket` is `None`, uses the default socket path.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentError`] if the socket cannot be bound or the
    /// server encounters an I/O error.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    ///
    /// EasyPdf::serve(None)?;
    /// ```
    #[cfg(feature = "resident")]
    pub fn serve(socket: Option<&Path>) -> std::result::Result<(), easypdf_runtime::resident::ResidentError> {
        easypdf_runtime::resident::serve(socket)
    }

    /// Attach to a running resident daemon.
    ///
    /// Returns `Some(ResidentClient)` if a daemon is listening on the
    /// default socket, or `None` if no daemon is running.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    ///
    /// if let Some(client) = EasyPdf::attach() {
    ///     let session = client.open("doc.pdf", easypdf_runtime::resident::OpenMode::ReadOnly)?;
    /// }
    /// ```
    #[cfg(feature = "resident")]
    #[must_use]
    pub fn attach() -> Option<ResidentClient> {
        easypdf_runtime::resident::try_attach()
    }
}
