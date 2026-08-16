//! 构建 [`PdfWriter`] 的构建器模式。
//!
//! 提供 [`PdfWriterBuilder`]，用于在构造写入器之前配置写入后端、
//! 处理器优先级、写入引擎和常量内存模式。
//!
//! # Examples
//!
//! ```
//! use easypdf_writer::{PdfWriterBuilder, WriteBackend, WriteEngineKind};
//! use easypdf_core::handler_chain::PRIORITY_HIGH;
//!
//! let writer = PdfWriterBuilder::new("My Document")
//!     .backend(WriteBackend::InMemory)
//!     .engine(WriteEngineKind::Printpdf)
//!     .constant_memory(false)
//!     .build();
//! ```

use easypdf_core::error::Result;
use easypdf_core::handler_chain::{PRIORITY_NORMAL, WriteHandlerChain};
use easypdf_core::{PdfMetadata, PdfWriteHandler};

use crate::backend::WriteBackend;
use crate::engine::WriteEngineKind;
use crate::writer::PdfWriter;

/// 用于构造具有高级配置的 [`PdfWriter`] 的构建器。
///
/// 支持：
/// - **写入引擎选择**（printpdf 或 krilla）
/// - **写入后端选择**（内存模式 vs 页面级溢出）
/// - **处理器优先级排序**（通过 [`WriteHandlerChain`]）
/// - **常量内存模式**便捷切换
/// - **临时文件压缩**（溢出模式）
///
/// # Examples
///
/// ```
/// use easypdf_writer::{PdfWriterBuilder, WriteBackend, WriteEngineKind};
/// use easypdf_core::PdfMetadata;
///
/// let writer = PdfWriterBuilder::new("Report")
///     .metadata(PdfMetadata::new().title("Q4 Report").author("Finance"))
///     .backend(WriteBackend::auto(500))
///     .engine(WriteEngineKind::Printpdf)
///     .compress_temp_files(true)
///     .build();
/// ```
pub struct PdfWriterBuilder {
    title: String,
    metadata: PdfMetadata,
    backend: WriteBackend,
    chain: WriteHandlerChain,
    engine_kind: WriteEngineKind,
    constant_memory: bool,
    compress_temp_files: bool,
}

impl PdfWriterBuilder {
    /// 创建指定标题的 PDF 文档构建器。
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            metadata: PdfMetadata::default(),
            backend: WriteBackend::default(),
            chain: WriteHandlerChain::new(),
            engine_kind: WriteEngineKind::default(),
            constant_memory: false,
            compress_temp_files: true,
        }
    }

    /// 选择写入引擎。
    ///
    /// 默认使用 [`WriteEngineKind::Printpdf`]。
    /// 启用 `writer-krilla` feature 后可选择 `WriteEngineKind::Krilla`。
    ///
    /// # Examples
    ///
    /// ```
    /// use easypdf_writer::{PdfWriterBuilder, WriteEngineKind};
    ///
    /// let writer = PdfWriterBuilder::new("CJK Report")
    ///     .engine(WriteEngineKind::Printpdf)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn engine(mut self, engine: WriteEngineKind) -> Self {
        self.engine_kind = engine;
        self
    }

    /// 设置文档元数据。
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// 设置写入后端（内存模式或溢出模式）。
    #[must_use]
    pub fn backend(mut self, backend: WriteBackend) -> Self {
        self.backend = backend;
        self
    }

    /// 启用或禁用常量内存模式。
    ///
    /// 启用时，后端自动切换为 [`WriteBackend::constant_memory()`]，
    /// 每个页面在完成后立即溢出。
    #[must_use]
    pub fn constant_memory(mut self, enabled: bool) -> Self {
        self.constant_memory = enabled;
        self
    }

    /// 设置溢出文件是否使用 gzip 压缩。
    ///
    /// 仅在后端为 [`WriteBackend::Spill`] 时生效。默认为 `true`。
    #[must_use]
    pub fn compress_temp_files(mut self, compress: bool) -> Self {
        self.compress_temp_files = compress;
        self
    }

    /// 注册使用默认优先级的写入处理器
    /// （[`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL)）。
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.chain.register(handler, PRIORITY_NORMAL);
        self
    }

    /// 注册使用指定优先级的写入处理器。
    ///
    /// 优先级值越小越先执行。参见
    /// [`PRIORITY_HIGH`](easypdf_core::handler_chain::PRIORITY_HIGH)、
    /// [`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL)、
    /// [`PRIORITY_LOW`](easypdf_core::handler_chain::PRIORITY_LOW)。
    #[must_use]
    pub fn register_handler_with_priority(
        mut self,
        handler: Box<dyn PdfWriteHandler>,
        priority: f64,
    ) -> Self {
        self.chain.register(handler, priority);
        self
    }

    /// 构建 [`PdfWriter`]。
    ///
    /// # Errors
    ///
    /// 当溢出后端无法初始化（例如无法创建溢出目录）时返回错误。
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

        PdfWriter::with_config(
            &self.title,
            self.metadata,
            backend,
            self.chain,
            self.engine_kind,
        )
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
