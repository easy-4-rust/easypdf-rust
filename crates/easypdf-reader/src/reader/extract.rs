//! [`PdfReader`] 的文本、元数据和文档模型提取。

use easypdf_core::error::{PdfError, Result};
use easypdf_core::{PageIndex, PdfMetadata, PdfReadListener};
use easypdf_core::{PdfBlock, PdfDocumentModel, PdfPageModel, SourceLocation};

use crate::strategy::{LazyPageLoader, ReadStrategy};
use crate::streaming::StreamScanner;

use super::PdfReader;
use super::usize_to_u64_saturating;

impl PdfReader {
    /// 从所有选定页面提取文本，以换行符连接。
    ///
    /// 对于 [`ReadStrategy::Streaming`]，直接扫描原始字节而不构建
    /// `lopdf::Document` 对象树。
    ///
    /// # Errors
    ///
    /// 当 PDF 内容无法读取时返回 `PdfError::Parse`。
    pub fn extract_text(&self) -> Result<String> {
        if self.strategy == ReadStrategy::Streaming {
            return self.extract_text_streaming();
        }

        let mut all_text = String::new();
        for (_, page_number) in self.selected_pages() {
            let text = self.extract_page_text(page_number)?;
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            self.ensure_text_limit(all_text.len() + text.len())?;
            all_text.push_str(&text);
        }
        Ok(all_text)
    }

    /// 从 PDF 文档中提取元数据。
    ///
    /// 对于 [`ReadStrategy::Streaming`]，对原始字节进行启发式扫描
    /// （不解析 xref）。对于其他策略，通过已解析的 `lopdf::Document`
    /// 读取 `/Info` 字典。
    ///
    /// # Errors
    ///
    /// 当文档无法读取时返回 `PdfError::Parse`。
    ///
    /// # Panics
    ///
    /// 在非 Streaming 读取器上调用且 `document` 为 `None` 时 panic
    /// （正常使用中不应发生）。
    pub fn extract_metadata(&self) -> Result<PdfMetadata> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            return Ok(scanner.extract_metadata_quick());
        }

        // 非 Streaming：从已解析文档中读取 /Info 字典。
        let doc = self
            .document
            .as_ref()
            .expect("document must be Some for non-Streaming strategies");

        let info_dict = doc
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|info| {
                let info_id = info.as_reference().ok()?;
                doc.get_object(info_id).ok()
            })
            .and_then(|obj| obj.as_dict().ok());

        let title = info_dict.as_ref().and_then(|dict| {
            dict.get(b"Title")
                .ok()
                .and_then(|v| v.as_str().ok())
                .map(decode_pdf_string)
        });

        let author = info_dict.as_ref().and_then(|dict| {
            dict.get(b"Author")
                .ok()
                .and_then(|v| v.as_str().ok())
                .map(decode_pdf_string)
        });

        Ok(PdfMetadata {
            title,
            author,
            subject: None,
            keywords: None,
            creator: None,
            producer: None,
        })
    }

    /// 获取文档的总页数。
    ///
    /// 对于 [`ReadStrategy::Streaming`]，基于原始字节中 `/Type /Page`
    /// 条目的启发式计数返回结果。
    ///
    /// # Errors
    ///
    /// 当文档无法读取时返回 `PdfError::Parse`。
    ///
    /// # Panics
    ///
    /// 在非 Streaming 读取器上调用且 `document` 为 `None` 时 panic
    /// （正常使用中不应发生）。
    pub fn page_count(&self) -> Result<usize> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            return Ok(scanner.page_count());
        }
        Ok(self
            .document
            .as_ref()
            .expect("document must be Some for non-Streaming strategies")
            .get_pages()
            .len())
    }

    /// 提取引擎中立的语义文档模型。
    ///
    /// 初始读取器后端输出段落块。更高级的分析器可以后续用标题、
    /// 表格、图片和 OCR 结果来丰富这些块。
    ///
    /// # Errors
    ///
    /// 当文本提取失败或超出资源限制时返回错误。
    pub fn extract_document_model(&self) -> Result<PdfDocumentModel> {
        if self.strategy == ReadStrategy::Streaming {
            let text = self.extract_text_streaming()?;
            let source = SourceLocation::new(PageIndex::new(0), 1.0);
            let mut page = PdfPageModel::new(PageIndex::new(0));
            for paragraph in split_paragraphs(&text) {
                page = page.with_block(PdfBlock::paragraph(paragraph, source));
            }
            return Ok(PdfDocumentModel::new(self.extract_metadata()?, vec![page]));
        }

        let mut pages = Vec::new();
        let mut extracted_bytes = 0usize;
        for (page_index, page_number) in self.selected_pages() {
            let text = self.extract_page_text(page_number)?;
            extracted_bytes = extracted_bytes.saturating_add(text.len());
            self.ensure_text_limit(extracted_bytes)?;
            let source = SourceLocation::new(PageIndex::new(page_index), 1.0);
            let mut page = PdfPageModel::new(PageIndex::new(page_index));
            for paragraph in split_paragraphs(&text) {
                page = page.with_block(PdfBlock::paragraph(paragraph, source));
            }
            pages.push(page);
        }
        Ok(PdfDocumentModel::new(self.extract_metadata()?, pages))
    }

    /// 使用事件驱动监听器读取文档（动态分发）。
    ///
    /// 此方法使用 `&mut dyn PdfReadListener` 进行动态分发。
    /// 如需零开销单态化，请使用 [`Self::read_with_listener_typed`]。
    ///
    /// # Errors
    ///
    /// 当文档无法读取时返回 `PdfError::Parse`。
    pub fn read_with_listener(&self, listener: &mut dyn PdfReadListener) -> Result<()> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            scanner.scan(listener)?;
            return Ok(());
        }

        for (page_index, page_number) in self.selected_pages() {
            let displayed_page = page_index + 1;
            let page_text = self.extract_page_text(page_number)?;
            listener.on_page_start(displayed_page)?;
            if !page_text.is_empty() {
                listener.on_text(displayed_page, &page_text)?;
            }
            listener.on_page_end(displayed_page)?;
        }
        listener.on_document_end()?;
        Ok(())
    }

    /// 使用类型化的事件驱动监听器读取文档（静态分发）。
    ///
    /// 这是 [`read_with_listener`](Self::read_with_listener) 的单态化版本。
    /// 编译器为每个具体的监听器类型生成特化版本，消除 vtable 开销。
    ///
    /// # Errors
    ///
    /// 当文档无法读取时返回 `PdfError::Parse`。
    pub fn read_with_listener_typed<L: PdfReadListener>(&self, listener: &mut L) -> Result<()> {
        if self.strategy == ReadStrategy::Streaming {
            let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
            scanner.scan(listener)?;
            return Ok(());
        }

        for (page_index, page_number) in self.selected_pages() {
            let displayed_page = page_index + 1;
            let page_text = self.extract_page_text(page_number)?;
            listener.on_page_start(displayed_page)?;
            if !page_text.is_empty() {
                listener.on_text(displayed_page, &page_text)?;
            }
            listener.on_page_end(displayed_page)?;
        }
        listener.on_document_end()?;
        Ok(())
    }

    /// 使用懒加载策略提取文本。
    ///
    /// 当读取器以 [`ReadStrategy::Lazy`] 打开时，此方法使用懒加载页面
    /// 加载器按需加载页面内容并带缓存。对于 [`ReadStrategy::Full`]，
    /// 委托给 [`extract_text`](Self::extract_text)。对于
    /// [`ReadStrategy::Streaming`]，委托给流式扫描器。
    ///
    /// # Errors
    ///
    /// 当页面内容无法解码时返回错误。
    ///
    /// # Panics
    ///
    /// 在非 Streaming 读取器上调用且 `document` 为 `None` 时 panic
    /// （正常使用中不应发生）。
    pub fn extract_text_lazy(&mut self) -> Result<String> {
        if self.strategy == ReadStrategy::Streaming {
            return self.extract_text_streaming();
        }

        if self.strategy.is_full() {
            return self.extract_text();
        }

        let doc = self
            .document
            .as_ref()
            .expect("document must be Some for non-Streaming strategies");
        let mut loader = LazyPageLoader::new(doc);
        let mut all_text = String::new();
        let indices: Vec<usize> = self.selected_pages().map(|(index, _)| index).collect();

        for (idx, text) in loader.pages_text(&indices)? {
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            self.ensure_text_limit(all_text.len() + text.len())?;
            all_text.push_str(&text);
            let _ = idx;
        }
        Ok(all_text)
    }

    // --- 私有辅助方法 ---

    /// 通过 `StreamScanner` 进行流式文本提取。
    pub(super) fn extract_text_streaming(&self) -> Result<String> {
        struct TextCollector {
            parts: Vec<String>,
        }
        impl PdfReadListener for TextCollector {
            fn on_text(&mut self, _page: usize, text: &str) -> easypdf_core::Result<()> {
                self.parts.push(text.to_string());
                Ok(())
            }
        }

        let scanner = StreamScanner::new(&self.raw_bytes, self.limits);
        let mut collector = TextCollector { parts: vec![] };
        scanner.scan(&mut collector)?;

        let mut all_text = String::new();
        for part in &collector.parts {
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            self.ensure_text_limit(all_text.len() + part.len())?;
            all_text.push_str(part);
        }
        Ok(all_text)
    }

    /// 返回非 Streaming 策略下的选定页面。
    ///
    /// # Panics
    ///
    /// 当 `self.document` 为 `None`（Streaming 策略）时 panic。
    pub(super) fn selected_pages(&self) -> impl Iterator<Item = (usize, u32)> + '_ {
        self.document
            .as_ref()
            .expect("selected_pages called for Streaming strategy; use StreamScanner instead")
            .get_pages()
            .into_keys()
            .enumerate()
            .filter(|(index, _)| {
                self.pages
                    .as_ref()
                    .is_none_or(|range| range.contains(*index))
            })
    }

    /// 从单个页面提取文本（非 Streaming 策略）。
    ///
    /// # Panics
    ///
    /// 当 `self.document` 为 `None`（Streaming 策略）时 panic。
    pub(super) fn extract_page_text(&self, page_number: u32) -> Result<String> {
        self.document
            .as_ref()
            .expect("extract_page_text called for Streaming strategy")
            .extract_text(&[page_number])
            .map_err(|error| PdfError::Parse(error.to_string()))
    }

    pub(super) fn ensure_text_limit(&self, bytes: usize) -> Result<()> {
        if bytes > self.limits.max_extracted_text_bytes() {
            return Err(PdfError::ResourceLimitExceeded {
                resource: "extracted_text_bytes",
                limit: usize_to_u64_saturating(self.limits.max_extracted_text_bytes()),
                actual: usize_to_u64_saturating(bytes),
            });
        }
        Ok(())
    }
}

/// 在双换行符处分割文本为段落。
fn split_paragraphs(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(ToOwned::to_owned)
}

/// 将 PDF 字符串对象解码为 Rust `String`。
///
/// PDF `/Info` 字典中的字符串可能采用以下编码：
/// - **UTF-16BE** 带 BOM（`\xFE\xFF`）前缀 -- printpdf 和大多数现代
///   PDF 生产者对非 ASCII 甚至所有文本使用的编码。
/// - **`PDFDocEncoding`**（Latin-1 的超集）-- 无 BOM 时的默认编码。
///
/// 此函数检测 UTF-16BE BOM 并相应解码，对 `PDFDocEncoding` / Latin-1
/// 字节回退到有损 UTF-8 解码。
fn decode_pdf_string(bytes: &[u8]) -> String {
    // 检测 UTF-16BE BOM（0xFE 0xFF）。
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let utf16: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    u16::from_be_bytes([chunk[0], chunk[1]])
                } else {
                    // 尾部单字节 -- 替换为 U+FFFD。
                    0xFFFD
                }
            })
            .collect();
        return String::from_utf16_lossy(&utf16);
    }
    // 无 BOM：按 PDFDocEncoding / Latin-1 / UTF-8 处理。
    // from_utf8_lossy 对无效字节使用 U+FFFD 替换。
    String::from_utf8_lossy(bytes).into_owned()
}
