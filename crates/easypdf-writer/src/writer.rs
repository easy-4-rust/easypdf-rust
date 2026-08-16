//! 主 `PdfWriter` 结构体与核心 PDF 写入方法。
//!
//! 底层使用引擎抽象层进行 PDF 构建。支持两种写入后端：
//! - **内存模式**（默认）：整个文档在内存中构建。
//! - **溢出模式**：已完成的页面序列化到临时文件，限制峰值内存。

use easypdf_core::AtomicFileOutput;
use easypdf_core::error::Result;
use easypdf_core::handler_chain::{PRIORITY_NORMAL, WriteHandlerChain};
use easypdf_core::layout::LayoutSink;
use easypdf_core::{
    FontFamily, Orientation, PageSize, PdfColor, PdfFont, PdfImage, PdfMetadata, PdfText,
    PdfWriteHandler,
};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::backend::{PageSpillWriter, SpilledPageData, WriteBackend};
use crate::engine::op::WriterOp;
use crate::engine::{FontKey, WriteEngine, WriteEngineKind, resolve_font_key};

/// 动态分发的写入引擎类型别名。
type DynEngine = Box<dyn WriteEngine>;

/// Default margin in points for auto-positioned text.
const DEFAULT_MARGIN: f64 = 72.0;

/// 用于创建新 PDF 文档的写入器。
///
/// 从操作构建页面，然后将文档序列化为字节。支持多页面、图片、
/// 自定义字体和图形。
///
/// # 写入后端
///
/// 使用 [`WriteBackend`] 在内存模式和页面级溢出模式之间选择。
/// 对于大型文档，溢出后端通过将已完成的页面序列化到临时文件来
/// 限制峰值内存。
///
/// # 处理器链
///
/// 处理器由按优先级排序的 [`WriteHandlerChain`] 管理。
/// [`register_handler`](Self::register_handler) 方法使用
/// [`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL)；
/// 使用 [`register_handler_with_priority`](Self::register_handler_with_priority)
/// 设置自定义优先级。
///
/// # Examples
///
/// ```
/// use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend};
/// use easypdf_core::*;
///
/// // 简单构造（向后兼容）。
/// let w = PdfWriter::new("title");
///
/// // 使用溢出后端的构建器。
/// let w = PdfWriterBuilder::new("Big Report")
///     .backend(WriteBackend::auto(500))
///     .build()
///     .unwrap();
/// ```
pub struct PdfWriter {
    /// PDF 写入引擎（持有文档和字体表）。
    pub(crate) engine: DynEngine,
    /// 当前页面正在构建的操作列表。
    pub(crate) current_page_ops: Vec<WriterOp>,
    /// 当前页面尺寸（宽, 高，单位 PDF 点）。
    current_page_size: (f64, f64),
    /// 当前页码（从 1 开始）。
    current_page_number: usize,
    /// 当前页面是否仍接受内容并等待终结。
    current_page_open: bool,
    /// 文档生命周期是否已开始。
    document_started: bool,
    /// 自定义字体键名 -> FontKey 映射（用于 write_text_with_custom_font 查找）。
    custom_font_keys: HashMap<String, FontKey>,
    /// 文档元数据。
    pub(crate) metadata: PdfMetadata,
    /// 按优先级排序的处理器链。
    chain: WriteHandlerChain,
    /// add_text 自动定位光标。
    text_cursor: (f64, f64),
    /// flush 模式的输出流。
    output: Option<Box<dyn Write>>,
    /// 写入后端配置。
    backend: WriteBackend,
    /// 页面级溢出写入器（后端为 Spill 时激活）。
    spill_writer: Option<PageSpillWriter>,
}

impl PdfWriter {
    /// 创建新的 PDF 文档（通过 `finish` 写入文件）。
    ///
    /// 使用默认的内存后端和默认引擎（printpdf）。如需高级配置，
    /// 请使用 [`PdfWriterBuilder`](crate::PdfWriterBuilder)。
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            engine: WriteEngineKind::default().create_engine(title),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            current_page_open: false,
            document_started: false,
            custom_font_keys: HashMap::new(),
            metadata: PdfMetadata::default(),
            chain: WriteHandlerChain::new(),
            text_cursor: (DEFAULT_MARGIN, 0.0),
            output: None,
            backend: WriteBackend::default(),
            spill_writer: None,
        }
    }

    /// 创建写入通用 writer 的新 PDF 文档。
    #[must_use]
    pub fn new_from_writer(writer: impl Write + 'static) -> Self {
        let mut s = Self::new("untitled");
        s.output = Some(Box::new(writer));
        s
    }

    /// 由 [`PdfWriterBuilder`] 使用的内部构造函数。
    ///
    /// # Errors
    ///
    /// 当溢出后端无法初始化时返回错误。
    pub(crate) fn with_config(
        title: &str,
        metadata: PdfMetadata,
        backend: WriteBackend,
        chain: WriteHandlerChain,
        engine_kind: WriteEngineKind,
    ) -> Result<Self> {
        let spill_writer = match &backend {
            WriteBackend::Spill {
                spill_dir,
                compress,
                threshold_pages,
            } => Some(PageSpillWriter::new(
                spill_dir.clone(),
                *compress,
                *threshold_pages,
            )?),
            WriteBackend::InMemory => None,
        };

        Ok(Self {
            engine: engine_kind.create_engine(title),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            current_page_open: false,
            document_started: false,
            custom_font_keys: HashMap::new(),
            metadata,
            chain,
            text_cursor: (DEFAULT_MARGIN, 0.0),
            output: None,
            backend,
            spill_writer,
        })
    }

    /// 设置文档元数据。
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// 注册使用默认优先级的写入处理器
    /// （[`PRIORITY_NORMAL`](easypdf_core::handler_chain::PRIORITY_NORMAL)）。
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.chain.register(handler, PRIORITY_NORMAL);
        self
    }

    /// 注册使用指定执行优先级的写入处理器。
    ///
    /// 优先级值越小越先执行。
    #[must_use]
    pub fn register_handler_with_priority(
        mut self,
        handler: Box<dyn PdfWriteHandler>,
        priority: f64,
    ) -> Self {
        self.chain.register(handler, priority);
        self
    }

    /// 从文件路径注册自定义 TTF/OTF 字体。
    pub fn register_font_from_path(&mut self, path: &str) -> Result<String> {
        let font_data = std::fs::read(path)?;
        self.register_font_from_bytes(path, &font_data)
    }

    /// 从字节数据注册自定义 TTF/OTF 字体。
    pub fn register_font_from_bytes(&mut self, key: &str, font_data: &[u8]) -> Result<String> {
        self.engine.register_font(key, font_data)?;
        self.custom_font_keys
            .insert(key.to_string(), FontKey::Custom(key.to_string()));
        Ok(key.to_string())
    }

    /// 使用自定义（非内置）字体写入文本。
    pub fn write_text_with_custom_font(
        &mut self,
        text: &str,
        font_key: &str,
        font_size: f64,
        x_pt: f64,
        y_pt: f64,
    ) -> Result<()> {
        let font_key = self
            .custom_font_keys
            .get(font_key)
            .cloned()
            .ok_or_else(|| {
                easypdf_core::error::PdfError::UnsupportedFeature(format!(
                    "Custom font '{font_key}' not registered."
                ))
            })?;
        let ops = vec![
            WriterOp::StartTextSection,
            WriterOp::SetTextCursor { x: x_pt, y: y_pt },
            WriterOp::SetFont {
                font: font_key,
                size: font_size,
            },
            WriterOp::ShowText {
                text: text.to_string(),
            },
            WriterOp::EndTextSection,
        ];
        self.current_page_ops.extend(ops);
        Ok(())
    }

    /// 添加新页面。
    pub fn add_page(&mut self, size: PageSize, orientation: Orientation) -> Result<usize> {
        self.finalize_current_page()?;
        self.ensure_document_started()?;
        self.current_page_number += 1;
        let (width, height) = size.dimensions();
        self.current_page_size = match orientation {
            Orientation::Portrait => (width, height),
            Orientation::Landscape => (height, width),
        };
        self.text_cursor = (DEFAULT_MARGIN, self.current_page_size.1 - DEFAULT_MARGIN);
        self.chain.before_page(self.current_page_number)?;
        self.current_page_open = true;
        Ok(self.current_page_number)
    }

    fn finalize_current_page(&mut self) -> Result<()> {
        if !self.current_page_open {
            return Ok(());
        }
        self.chain.after_page(self.current_page_number)?;
        let ops = std::mem::take(&mut self.current_page_ops);
        let (w, h) = self.current_page_size;

        // If spill writer is active, attempt to spill this page.
        if let Some(ref mut spill) = self.spill_writer {
            let page_data = SpilledPageData {
                page_number: self.current_page_number,
                width_pt: w,
                height_pt: h,
                ops: ops.clone(),
            };
            if spill.maybe_spill(&page_data)?.is_some() {
                // Page was spilled -- do not keep in memory.
                self.current_page_open = false;
                return Ok(());
            }
        }

        // Keep page in memory (in-memory mode, or below spill threshold).
        self.engine.add_page(w, h, ops);
        self.current_page_open = false;
        Ok(())
    }

    fn ensure_document_started(&mut self) -> Result<()> {
        if self.document_started {
            return Ok(());
        }
        self.chain.before_document()?;
        self.document_started = true;
        Ok(())
    }

    /// 获取当前页码（从 1 开始）。
    #[must_use]
    pub const fn current_page_number(&self) -> usize {
        self.current_page_number
    }

    /// 获取已完成的总页数。
    #[must_use]
    pub fn page_count(&self) -> usize {
        // Include both in-memory pages and spilled pages.
        let spilled = self.spill_writer.as_ref().map_or(0, |s| s.spilled_count());
        self.current_page_number + spilled
    }

    /// 返回此写入器是否处于常量内存（溢出）模式。
    #[must_use]
    pub fn is_constant_memory(&self) -> bool {
        self.backend.is_constant_memory()
    }

    /// 切换常量内存模式。
    ///
    /// 启用时，后端设置为 [`WriteBackend::constant_memory()`]，每个页面
    /// 在完成后立即溢出。禁用时，后端设置为 [`WriteBackend::InMemory`]。
    ///
    /// 注意：在文档中间切换模式对已完成的页面无效。
    pub fn set_constant_memory(&mut self, enabled: bool) {
        if enabled {
            if !self.backend.is_constant_memory() {
                self.backend = WriteBackend::constant_memory();
                // Initialize spill writer if not present.
                if self.spill_writer.is_none() {
                    self.spill_writer = PageSpillWriter::new(None, true, 1).ok();
                }
            }
        } else {
            self.backend = WriteBackend::InMemory;
            // We do not drop the spill writer -- already-spilled pages need
            // to be collected at finish time.
        }
    }

    /// 返回已注册的处理器数量。
    #[must_use]
    pub fn handler_count(&self) -> usize {
        self.chain.len()
    }

    /// 返回元数据中的文档标题（如已设置）。
    #[must_use]
    pub fn metadata_title(&self) -> Option<&str> {
        self.metadata.title.as_deref()
    }

    /// 在 PDF 点坐标 (x, y) 处写入文本。
    pub fn write_text(&mut self, text: &PdfText, x_pt: f64, y_pt: f64) -> Result<()> {
        // 解析字体键：自定义字体优先查表，内置字体实时解析。
        let font_key = if let FontFamily::Custom(ref path) = text.font.family {
            self.custom_font_keys
                .get(path.as_ref())
                .cloned()
                .unwrap_or_else(|| resolve_font_key(&text.font))
        } else {
            resolve_font_key(&text.font)
        };

        let ops = vec![
            WriterOp::StartTextSection,
            WriterOp::SetTextCursor { x: x_pt, y: y_pt },
            WriterOp::SetFont {
                font: font_key,
                size: text.font.size,
            },
            WriterOp::ShowText {
                text: text.content.clone(),
            },
            WriterOp::EndTextSection,
        ];
        self.current_page_ops.extend(ops);
        Ok(())
    }

    /// 添加自动定位的文本。
    pub fn add_text(&mut self, font: &PdfFont, text: &str) -> Result<&mut Self> {
        let (x, y) = self.text_cursor;
        self.write_text(&PdfText::new(text).font(font.clone()), x, y)?;
        self.text_cursor.1 -= font.size + 4.0;
        Ok(self)
    }

    /// 添加带显式颜色的自动定位文本。
    pub fn add_text_colored(
        &mut self,
        font: &PdfFont,
        color: &PdfColor,
        text: &str,
    ) -> Result<&mut Self> {
        let (x, y) = self.text_cursor;
        self.write_text(&PdfText::new(text).font(font.clone()).color(*color), x, y)?;
        self.text_cursor.1 -= font.size + 4.0;
        Ok(self)
    }

    /// 从文件路径添加图片。
    pub fn add_image_from_path(
        &mut self,
        path: impl AsRef<Path>,
        w_pt: f64,
        h_pt: f64,
    ) -> Result<&mut Self> {
        let img = PdfImage::from_path(path)?;
        let (x, y) = self.text_cursor;
        self.write_image(&img, x, y - h_pt, w_pt, h_pt)?;
        self.text_cursor.1 -= h_pt + 8.0;
        Ok(self)
    }

    /// 使用原子输出（fsync）将文档写入文件。
    ///
    /// 完成当前页面，对所有处理器触发 `after_document`，收集溢出的页面，
    /// 构建最终 PDF 并以原子方式写入（临时文件 + fsync + 重命名）。
    pub fn finish(mut self, path: impl AsRef<Path>) -> Result<()> {
        if self.current_page_number == 0 {
            self.add_page(PageSize::A4, Orientation::Portrait)?;
        }
        self.finalize_current_page()?;
        self.chain.after_document()?;

        // Collect spilled pages (if any) and add to engine.
        if let Some(ref spill) = self.spill_writer {
            let spilled = spill.collect_all()?;
            for data in spilled {
                self.engine
                    .add_page(data.width_pt, data.height_pt, data.ops);
            }
        }

        let bytes = self.engine.finish(&self.metadata)?;
        AtomicFileOutput::new(path.as_ref()).write_with_fsync(&bytes)
    }

    /// 刷新到预配置的输出流。
    #[allow(clippy::similar_names)]
    pub fn flush(&mut self) -> Result<()> {
        let ops = std::mem::take(&mut self.current_page_ops);
        if !ops.is_empty() {
            let (w, h) = self.current_page_size;
            self.engine.add_page(w, h, ops);
        }
        // Ensure at least one page exists.
        if self.current_page_number == 0 {
            let (w, h) = self.current_page_size;
            self.engine.add_page(w, h, Vec::new());
        }
        let bytes = self.engine.finish(&self.metadata)?;
        if let Some(ref mut w) = self.output {
            w.write_all(&bytes)?;
        }
        Ok(())
    }
}

impl LayoutSink for PdfWriter {
    fn add_page(&mut self, size: PageSize, orientation: Orientation) -> Result<usize> {
        Self::add_page(self, size, orientation)
    }

    fn write_text(&mut self, text: &PdfText, x: f64, y: f64) -> Result<()> {
        Self::write_text(self, text, x, y)
    }

    fn finish(self, path: &Path) -> Result<()> {
        Self::finish(self, path)
    }
}
