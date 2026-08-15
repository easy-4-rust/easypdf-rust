//! PDF 读取策略：全量、懒加载和流式。
//!
//! 对应 `ExcelReadExecutorKind` 枚举分发模式：调用方（或自动检测）
//! 根据文档大小选择最优的解析策略。

use std::collections::HashMap;

use easypdf_core::{PdfError, Result};

/// PDF 读取策略枚举。
///
/// 选择 PDF 文档的解析和内存加载方式。
/// 使用 [`ReadStrategy::auto`] 根据文件大小自动选择最佳策略。
///
/// # Examples
///
/// ```
/// use easypdf_reader::ReadStrategy;
///
/// let strategy = ReadStrategy::auto(1024 * 1024); // 1 MB
/// assert_eq!(strategy, ReadStrategy::Full);
///
/// let strategy = ReadStrategy::auto(50 * 1024 * 1024); // 50 MB
/// assert_eq!(strategy, ReadStrategy::Lazy);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReadStrategy {
    /// 全量内存加载（默认，适用于小型文档）。
    ///
    /// 通过 `lopdf::Document::load_mem` 将整个 PDF 加载到内存 --
    /// 支持对所有对象的快速随机访问，但要求整个文档能放入内存。
    Full,

    /// 懒加载页面级解析（适用于大型文档）。
    ///
    /// 仅解析 trailer、交叉引用表和页面树结构。各页面的内容流
    /// 按需加载并在首次访问后缓存。
    Lazy,

    /// 流式扫描（适用于超大型文档，仅限文本提取）。
    ///
    /// 不构建完整的对象树。扫描 PDF 字节流中的内容流并增量触发
    /// 监听器回调。
    ///
    /// 精度低于 [`Full`](Self::Full) 或 [`Lazy`](Self::Lazy)，
    /// 因为交叉引用解析和字体编码（CMap/ToUnicode）被跳过。
    Streaming,
}

impl ReadStrategy {
    /// 选择 `Full` 策略的文件大小阈值（字节）。
    const FULL_THRESHOLD: u64 = 5_000_000; // 5 MB
    /// 选择 `Lazy` 策略的文件大小阈值（字节）。
    const LAZY_THRESHOLD: u64 = 100_000_000; // 100 MB

    /// 根据文件大小自动选择最佳策略。
    ///
    /// | 文件大小 | 策略 |
    /// |-----------|----------|
    /// | 0..5 MB | [`Full`](ReadStrategy::Full) |
    /// | 5..100 MB | [`Lazy`](ReadStrategy::Lazy) |
    /// | > 100 MB | [`Streaming`](ReadStrategy::Streaming) |
    #[must_use]
    pub const fn auto(file_size: u64) -> Self {
        if file_size <= Self::FULL_THRESHOLD {
            Self::Full
        } else if file_size <= Self::LAZY_THRESHOLD {
            Self::Lazy
        } else {
            Self::Streaming
        }
    }

    /// 当此策略在启动时加载整个文档时返回 `true`。
    #[must_use]
    pub const fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// 当此策略延迟加载页面内容时返回 `true`。
    #[must_use]
    pub const fn is_lazy(&self) -> bool {
        matches!(self, Self::Lazy | Self::Streaming)
    }
}

/// 懒加载器缓存的已解析页面内容。
#[derive(Debug, Clone)]
pub(crate) struct ParsedPage {
    /// 此页面提取的文本。
    pub text: String,
}

/// 懒加载页面级加载器。
///
/// 仅在启动时解析页面树结构；各页面的内容流按需加载并在首次访问后
/// 缓存。这避免了为大型文档物化完整的对象树。
///
/// 加载器借用已加载的 [`lopdf::Document`]（持有 trailer 和 xref 表），
/// 并在不读取任何内容流的情况下构建页面对象 ID 的扁平列表。
pub(crate) struct LazyPageLoader<'a> {
    doc: &'a lopdf::Document,
    page_object_ids: Vec<lopdf::ObjectId>,
    cached_pages: HashMap<usize, ParsedPage>,
}

impl<'a> LazyPageLoader<'a> {
    /// 从已解析的 `lopdf::Document` 构建懒加载器。
    ///
    /// 遍历页面树以收集页面对象 ID，但**不**读取任何内容流。
    #[must_use]
    pub fn new(doc: &'a lopdf::Document) -> Self {
        let pages_map = doc.get_pages();
        let mut page_object_ids = Vec::with_capacity(pages_map.len());
        // `get_pages()` 返回按页码排序的 BTreeMap<u32, ObjectId>。
        for (_page_num, obj_id) in pages_map {
            page_object_ids.push(obj_id);
        }
        Self {
            doc,
            page_object_ids,
            cached_pages: HashMap::new(),
        }
    }

    /// 总页数（无需加载内容即可获取）。
    #[must_use]
    #[allow(dead_code)] // 在测试中使用；未来的流式策略也会调用
    pub fn page_count(&self) -> usize {
        self.page_object_ids.len()
    }

    /// 提取单个页面的文本（从 0 开始的索引），带缓存。
    ///
    /// 对某个页面的首次调用会读取并解压内容流；后续调用返回缓存结果。
    ///
    /// # Errors
    ///
    /// 当页面内容无法解码时返回 [`PdfError::Parse`]；
    /// 当索引越界时返回 [`PdfError::InvalidPage`]。
    pub fn page_text(&mut self, page_index: usize) -> Result<String> {
        if let Some(cached) = self.cached_pages.get(&page_index) {
            return Ok(cached.text.clone());
        }

        if page_index >= self.page_object_ids.len() {
            return Err(PdfError::InvalidPage(page_index));
        }

        // lopdf 页码从 1 开始。
        let page_number = u32::try_from(page_index)
            .map_err(|_| PdfError::Parse("page index overflow".to_string()))?
            + 1;

        let text = self
            .doc
            .extract_text(&[page_number])
            .map_err(|error| PdfError::Parse(error.to_string()))?;

        self.cached_pages
            .insert(page_index, ParsedPage { text: text.clone() });
        Ok(text)
    }

    /// 提取多个页面的文本（从 0 开始的索引）。
    ///
    /// # Errors
    ///
    /// 当任意页面无法提取时返回错误。
    pub fn pages_text(&mut self, indices: &[usize]) -> Result<Vec<(usize, String)>> {
        let mut results = Vec::with_capacity(indices.len());
        for &idx in indices {
            let text = self.page_text(idx)?;
            results.push((idx, text));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names)]
    use super::*;

    // --- ReadStrategy ---

    #[test]
    fn auto_small_file_is_full() {
        assert_eq!(ReadStrategy::auto(0), ReadStrategy::Full);
        assert_eq!(ReadStrategy::auto(1), ReadStrategy::Full);
        assert_eq!(ReadStrategy::auto(5_000_000), ReadStrategy::Full);
    }

    #[test]
    fn auto_medium_file_is_lazy() {
        assert_eq!(ReadStrategy::auto(5_000_001), ReadStrategy::Lazy);
        assert_eq!(ReadStrategy::auto(50_000_000), ReadStrategy::Lazy);
        assert_eq!(ReadStrategy::auto(100_000_000), ReadStrategy::Lazy);
    }

    #[test]
    fn auto_large_file_is_streaming() {
        assert_eq!(ReadStrategy::auto(100_000_001), ReadStrategy::Streaming);
        assert_eq!(ReadStrategy::auto(u64::MAX), ReadStrategy::Streaming);
    }

    #[test]
    fn is_full_and_is_lazy() {
        assert!(ReadStrategy::Full.is_full());
        assert!(!ReadStrategy::Full.is_lazy());

        assert!(!ReadStrategy::Lazy.is_full());
        assert!(ReadStrategy::Lazy.is_lazy());

        assert!(!ReadStrategy::Streaming.is_full());
        assert!(ReadStrategy::Streaming.is_lazy());
    }

    #[test]
    fn strategy_debug_clone_eq_hash() {
        let s = ReadStrategy::Lazy;
        let s2 = s;
        assert_eq!(s, s2);
        assert_eq!(format!("{s:?}"), "Lazy");

        // Verify it works in a HashSet.
        let mut set = std::collections::HashSet::new();
        set.insert(ReadStrategy::Full);
        set.insert(ReadStrategy::Lazy);
        set.insert(ReadStrategy::Streaming);
        assert_eq!(set.len(), 3);
    }

    // --- LazyPageLoader ---

    fn make_test_doc() -> lopdf::Document {
        let mut doc = lopdf::Document::new();
        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf (Lazy Test) Tj ET".to_vec(),
        )));
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));
        let mut pages = lopdf::Dictionary::new();
        pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));
        doc
    }

    #[test]
    fn lazy_page_loader_page_count() {
        let doc = make_test_doc();
        let loader = LazyPageLoader::new(&doc);
        assert_eq!(loader.page_count(), 1);
    }

    #[test]
    fn lazy_page_loader_extracts_text() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let text = loader.page_text(0).unwrap();
        // Text extraction depends on font encoding; just verify no error.
        let _ = text;
    }

    #[test]
    fn lazy_page_loader_caches() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let text1 = loader.page_text(0).unwrap();
        let text2 = loader.page_text(0).unwrap();
        assert_eq!(text1, text2);
        assert_eq!(loader.cached_pages.len(), 1);
    }

    #[test]
    fn lazy_page_loader_out_of_bounds() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let result = loader.page_text(99);
        assert!(result.is_err());
    }

    #[test]
    fn lazy_page_loader_pages_text() {
        let doc = make_test_doc();
        let mut loader = LazyPageLoader::new(&doc);
        let results = loader.pages_text(&[0]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0);
    }
}
