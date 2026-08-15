//! [`EasyPdf`] 的 feature-gated 门面方法。

#[cfg(any(feature = "render", feature = "resident"))]
use std::path::Path;

use crate::EasyPdf;

#[cfg(feature = "mcp")]
use crate::McpServer;
#[cfg(feature = "render")]
use crate::RenderError;
#[cfg(feature = "resident")]
use crate::ResidentClient;
#[cfg(feature = "markdown-table")]
use crate::TableDetectorProcessor;
#[cfg(feature = "markdown")]
use crate::{MarkdownProfile, ProcessorPipeline};

impl EasyPdf {
    /// 创建一个带后端选择的 PDF Writer。
    ///
    /// 返回一个 `PdfWriterBuilder`，用于配置写入后端
    /// （`InMemory` 或 `Spill`）、处理器优先级和常量内存模式。
    ///
    /// # 示例
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

    /// 创建一个 Markdown 转换管道。
    ///
    /// 返回一个针对给定 profile 预配置的 [`ProcessorPipeline`]。
    /// 在运行前注册额外的处理器（表格检测器、OCR）。
    ///
    /// # 示例
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

    /// 创建一个表格检测器处理器。
    ///
    /// 返回一个使用默认配置的 [`TableDetectorProcessor`]。
    /// 将其注册到 [`ProcessorPipeline`] 以在 Markdown 转换期间启用表格检测。
    ///
    /// # 示例
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

    /// 将单个 PDF 页面渲染为 PNG 文件。
    ///
    /// 使用默认渲染后端（文本渲染器或 pdfium，如果可用）。
    ///
    /// # Errors
    ///
    /// 如果页面无法渲染或输出无法写入，返回 [`RenderError`]。
    ///
    /// # 示例
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

    /// 启动 MCP（Model Context Protocol）服务器。
    ///
    /// 服务器从 stdin 读取 JSON-RPC 2.0 请求并将响应写入 stdout，
    /// 向 LLM agent 暴露 PDF 操作。
    ///
    /// # 示例
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

    /// 启动常驻守护进程，在内存中保持 PDF 文档打开。
    ///
    /// 绑定到 Unix socket 并等待客户端连接。
    /// 如果 `socket` 为 `None`，使用默认 socket 路径。
    ///
    /// # Errors
    ///
    /// 如果 socket 无法绑定或服务器遇到 I/O 错误，
    /// 返回 [`ResidentError`]。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use easypdf::EasyPdf;
    ///
    /// EasyPdf::serve(None)?;
    /// ```
    #[cfg(feature = "resident")]
    pub fn serve(
        socket: Option<&Path>,
    ) -> std::result::Result<(), easypdf_runtime::resident::ResidentError> {
        easypdf_runtime::resident::serve(socket)
    }

    /// 连接到正在运行的 resident 守护进程。
    ///
    /// 如果守护进程正在默认 socket 上监听，返回 `Some(ResidentClient)`；
    /// 如果没有守护进程在运行，返回 `None`。
    ///
    /// # 示例
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
