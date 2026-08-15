//! 主 `PdfWriter` 结构体与核心 PDF 写入方法。
//!
//! 底层使用 `printpdf` 进行 PDF 构建。支持两种写入后端：
//! - **内存模式**（默认）：整个文档在内存中构建。
//! - **溢出模式**：已完成的页面序列化到临时文件，限制峰值内存。

use easypdf_core::AtomicFileOutput;
use easypdf_core::error::{PdfError, Result};
use easypdf_core::handler_chain::{PRIORITY_NORMAL, WriteHandlerChain};
use easypdf_core::layout::LayoutSink;
use easypdf_core::{
    FontFamily, Orientation, PageSize, PdfColor, PdfFont, PdfImage, PdfMetadata, PdfText,
    PdfWriteHandler,
};
use printpdf::{Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::backend::{PageSpillWriter, SpilledPageData, WriteBackend};
use crate::font::map_builtin_font;

/// PDF measurement units.
const PT_TO_MM: f64 = 25.4 / 72.0;
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
    pub(crate) doc: PdfDocument,
    /// Accumulated completed pages (in-memory mode only).
    pages: Vec<PdfPage>,
    /// Operations being built for the current page.
    pub(crate) current_page_ops: Vec<Op>,
    /// Current page size for the page being built.
    current_page_size: (f64, f64),
    /// Current page number (1-based).
    current_page_number: usize,
    /// Whether the current page still accepts content and awaits finalization.
    current_page_open: bool,
    /// Whether the document lifecycle has started.
    document_started: bool,
    /// Registered custom font IDs keyed by path.
    custom_fonts: HashMap<String, printpdf::FontId>,
    /// Document metadata.
    pub(crate) metadata: PdfMetadata,
    /// Priority-sorted handler chain.
    chain: WriteHandlerChain,
    /// Auto-cursor for add_text convenience.
    text_cursor: (f64, f64),
    /// Output stream for flush-based writing.
    output: Option<Box<dyn Write>>,
    /// Write backend configuration.
    backend: WriteBackend,
    /// Page-level spill writer (active when backend is `Spill`).
    spill_writer: Option<PageSpillWriter>,
}

impl PdfWriter {
    /// 创建新的 PDF 文档（通过 `finish` 写入文件）。
    ///
    /// 使用默认的内存后端。如需高级配置，
    /// 请使用 [`PdfWriterBuilder`](crate::PdfWriterBuilder)。
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            doc: PdfDocument::new(title),
            pages: Vec::new(),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            current_page_open: false,
            document_started: false,
            custom_fonts: HashMap::new(),
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
            doc: PdfDocument::new(title),
            pages: Vec::new(),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            current_page_open: false,
            document_started: false,
            custom_fonts: HashMap::new(),
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
        let mut warnings = Vec::new();
        let parsed = printpdf::ParsedFont::from_bytes(font_data, 0, &mut warnings)
            .ok_or_else(|| PdfError::Parse(format!("Failed to parse font: {key}")))?;
        let font_id = self.doc.add_font(&parsed);
        self.custom_fonts.insert(key.to_string(), font_id);
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
        let font_id = self.custom_fonts.get(font_key).cloned().ok_or_else(|| {
            PdfError::UnsupportedFeature(format!("Custom font '{font_key}' not registered."))
        })?;
        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos },
            Op::SetFont {
                font: PdfFontHandle::External(font_id),
                size: Pt(font_size as f32),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
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
        self.pages.push(PdfPage::new(
            Mm(w as f32 * PT_TO_MM as f32),
            Mm(h as f32 * PT_TO_MM as f32),
            ops,
        ));
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
        self.pages.len() + spilled
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
        if let FontFamily::Custom(ref path) = text.font.family
            && let Some(font_id) = self.custom_fonts.get(path.as_ref())
        {
            let pos = Point {
                x: Pt(x_pt as f32),
                y: Pt(y_pt as f32),
            };
            let ops = vec![
                Op::StartTextSection,
                Op::SetTextCursor { pos },
                Op::SetFont {
                    font: PdfFontHandle::External(font_id.clone()),
                    size: Pt(text.font.size as f32),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(text.content.clone())],
                },
                Op::EndTextSection,
            ];
            self.current_page_ops.extend(ops);
            return Ok(());
        }
        let bf = map_builtin_font(&text.font);
        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos },
            Op::SetFont {
                font: PdfFontHandle::Builtin(bf),
                size: Pt(text.font.size as f32),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.content.clone())],
            },
            Op::EndTextSection,
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

        // Apply easypdf metadata to printpdf document info before saving.
        // This ensures PdfMetadata set via builder methods is written into
        // the PDF's /Info dictionary, overriding the default title from
        // PdfDocument::new().
        self.apply_metadata();

        // Collect spilled pages (if any) and merge with in-memory pages.
        let mut all_pages = std::mem::take(&mut self.pages);
        if let Some(ref spill) = self.spill_writer {
            let spilled = spill.collect_all()?;
            for data in spilled {
                all_pages.push(PdfPage::new(
                    Mm(data.width_pt as f32 * PT_TO_MM as f32),
                    Mm(data.height_pt as f32 * PT_TO_MM as f32),
                    data.ops,
                ));
            }
        }

        self.doc.with_pages(all_pages);
        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        let bytes = self.doc.save(&opts, &mut warnings);
        AtomicFileOutput::new(path.as_ref()).write_with_fsync(&bytes)
    }

    /// 将 easypdf 元数据字段复制到 printpdf 文档信息中。
    fn apply_metadata(&mut self) {
        let info = &mut self.doc.metadata.info;
        if let Some(ref title) = self.metadata.title {
            info.document_title.clone_from(title);
        }
        if let Some(ref author) = self.metadata.author {
            info.author.clone_from(author);
        }
        if let Some(ref subject) = self.metadata.subject {
            info.subject.clone_from(subject);
        }
        if let Some(ref keywords) = self.metadata.keywords {
            info.keywords = keywords.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Some(ref creator) = self.metadata.creator {
            info.creator.clone_from(creator);
        }
        if let Some(ref producer) = self.metadata.producer {
            info.producer.clone_from(producer);
        }
    }

    /// 刷新到预配置的输出流。
    #[allow(clippy::similar_names)]
    pub fn flush(&mut self) -> Result<()> {
        let mut pages = std::mem::take(&mut self.pages);
        let ops = std::mem::take(&mut self.current_page_ops);
        if !ops.is_empty() {
            let (w, h) = self.current_page_size;
            pages.push(PdfPage::new(
                Mm(w as f32 * PT_TO_MM as f32),
                Mm(h as f32 * PT_TO_MM as f32),
                ops,
            ));
        }
        if pages.is_empty() {
            let (w, h) = self.current_page_size;
            pages.push(PdfPage::new(
                Mm(w as f32 * PT_TO_MM as f32),
                Mm(h as f32 * PT_TO_MM as f32),
                Vec::new(),
            ));
        }
        self.apply_metadata();
        self.doc.with_pages(pages);
        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        if let Some(ref mut w) = self.output {
            self.doc.save_writer(w, &opts, &mut warnings);
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
