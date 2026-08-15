//! PDF 操作 -- 合并、拆分、旋转和重排页面。
//!
//! 底层使用 `lopdf` crate 进行低层页面操作。

use easypdf_core::AtomicFileOutput;
use easypdf_core::Rotation;
use easypdf_core::error::{PdfError, Result};
use std::path::Path;

/// 对现有 PDF 文档执行操作的操作器。
///
/// 支持合并、拆分、旋转和重排页面。
pub struct PdfManipulator {
    doc: lopdf::Document,
}

impl PdfManipulator {
    /// 打开 PDF 文件进行操作。
    ///
    /// # Errors
    ///
    /// 当文件无法打开或不是有效的 PDF 时返回 `PdfError::Parse`。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let doc = lopdf::Document::load(path).map_err(|e| PdfError::Parse(e.to_string()))?;
        Ok(Self { doc })
    }

    /// 将多个 PDF 文件合并为一个新文档并保存。
    ///
    /// 这是最简单的合并方式；创建一个新文档并将所有输入文件的
    /// 所有页面复制到其中。
    ///
    /// # Errors
    ///
    /// 当任何源文件无法读取时返回 `PdfError::Parse`。
    /// 当输出文件无法写入时返回 `PdfError::Io`。
    pub fn merge_files(paths: &[impl AsRef<Path>], output: impl AsRef<Path>) -> Result<()> {
        if paths.is_empty() {
            return Err(PdfError::Other("No input files specified".into()));
        }

        let mut base_doc = lopdf::Document::load(&paths[0]).map_err(|e| {
            PdfError::Parse(format!(
                "Failed to load {}: {}",
                paths[0].as_ref().display(),
                e
            ))
        })?;
        let base_pages_id = root_pages_id(&base_doc)?;

        for path in &paths[1..] {
            let mut other_doc = lopdf::Document::load(path).map_err(|e| {
                PdfError::Parse(format!("Failed to load {}: {}", path.as_ref().display(), e))
            })?;
            let page_count = other_doc.get_pages().len();
            other_doc.renumber_objects_with(base_doc.max_id + 1);
            let other_pages_id = root_pages_id(&other_doc)?;
            base_doc.max_id = base_doc.max_id.max(other_doc.max_id);
            base_doc.objects.extend(other_doc.objects);
            append_page_tree(&mut base_doc, base_pages_id, other_pages_id, page_count)?;
        }

        save_document_atomically(base_doc, output)
    }

    /// 旋转指定页面（从 1 开始的索引）。
    ///
    /// # Errors
    ///
    /// 当页码越界时返回 `PdfError::InvalidPage`。
    pub fn rotate_page(&mut self, page_number: usize, rotation: Rotation) -> Result<()> {
        let pages = self.doc.get_pages();
        let page_id = pages
            .get(&u32::try_from(page_number).map_err(|_| PdfError::InvalidPage(page_number))?)
            .copied()
            .ok_or(PdfError::InvalidPage(page_number))?;

        let current_rotate = self
            .doc
            .get_object(page_id)
            .ok()
            .and_then(|obj| obj.as_dict().ok())
            .and_then(|dict| dict.get(b"Rotate").ok())
            .and_then(|v| v.as_i64().ok())
            .unwrap_or(0);

        let new_rotate = match rotation {
            Rotation::None => 0,
            Rotation::Clockwise90 => (current_rotate + 90) % 360,
            Rotation::Clockwise180 => (current_rotate + 180) % 360,
            Rotation::Clockwise270 => (current_rotate + 270) % 360,
        };

        if let Ok(dict) = self
            .doc
            .get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
        {
            dict.set("Rotate", lopdf::Object::Integer(new_rotate));
        }
        Ok(())
    }

    /// 按给定排列顺序重排页面（从 0 开始的索引）。
    ///
    /// # Errors
    ///
    /// 当任何索引越界时返回 `PdfError::InvalidPage`。
    pub fn reorder_pages(&mut self, order: &[usize]) -> Result<()> {
        let pages = self.doc.get_pages();
        let old_order: Vec<_> = pages.values().copied().collect();

        let mut new_order = Vec::with_capacity(order.len());
        for &idx in order {
            let page_id = old_order
                .get(idx)
                .copied()
                .ok_or(PdfError::InvalidPage(idx))?;
            new_order.push(page_id);
        }

        // 更新页面树：修改 catalog 的 /Pages -> /Kids 数组
        let count = i64::try_from(new_order.len()).unwrap_or(i64::MAX);
        if let Some(pages_dict) = self
            .doc
            .catalog_mut()
            .ok()
            .and_then(|c| c.get(b"Pages").ok()?.as_reference().ok())
            .and_then(|id| self.doc.get_object_mut(id).ok())
            .and_then(|obj| obj.as_dict_mut().ok())
        {
            pages_dict.set("Count", lopdf::Object::Integer(count));
            let kids: Vec<lopdf::Object> = new_order
                .into_iter()
                .map(lopdf::Object::Reference)
                .collect();
            pages_dict.set("Kids", lopdf::Object::Array(kids));
        }
        Ok(())
    }

    /// 提取指定范围的页面（从 0 开始）为新文档。
    ///
    /// # Errors
    ///
    /// 当范围越界时返回 `PdfError::InvalidPage`。
    pub fn extract_pages(&self, range: std::ops::Range<usize>) -> Result<lopdf::Document> {
        let pages: Vec<lopdf::ObjectId> = self.doc.page_iter().collect();
        let selected = range
            .map(|index| {
                pages
                    .get(index)
                    .copied()
                    .ok_or(PdfError::InvalidPage(index))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut new_doc = self.doc.clone();
        let pages_id = root_pages_id(&new_doc)?;
        for page_id in &selected {
            let page = new_doc
                .get_object_mut(*page_id)
                .map_err(|error| PdfError::Parse(error.to_string()))?
                .as_dict_mut()
                .map_err(|error| PdfError::Parse(error.to_string()))?;
            page.set("Parent", lopdf::Object::Reference(pages_id));
        }
        replace_page_tree_kids(&mut new_doc, pages_id, &selected)?;
        Ok(new_doc)
    }

    /// 为所有页面添加简单的文本水印叠加层。
    ///
    /// 水印文本作为原始 PDF 内容流算子注入到每个页面内容的末尾。
    ///
    /// # Errors
    ///
    /// 当页面内容无法修改时返回 `PdfError::Parse`。
    pub fn add_text_watermark(
        &mut self,
        text: &str,
        font_size: f32,
        _opacity: f32,
    ) -> Result<&mut Self> {
        let page_ids: Vec<lopdf::ObjectId> = self.doc.page_iter().collect();
        for page_id in page_ids {
            // 构建居中文本的原始 PDF 内容流
            let content = format!(
                "q BT /F1 {font_size} Tf 0.5 0.5 0.5 rg 1 0 0 1 200 400 Tm ({text}) Tj ET Q"
            );
            self.doc
                .add_page_contents(page_id, content.into_bytes())
                .map_err(|e| PdfError::Parse(format!("Watermark error: {e}")))?;
        }
        Ok(self)
    }

    /// 获取文档中的页数。
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    /// 将操作后的文档保存到文件。
    ///
    /// # Errors
    ///
    /// 当文件无法写入时返回 `PdfError::Io`。
    pub fn save(self, path: impl AsRef<Path>) -> Result<()> {
        save_document_atomically(self.doc, path)
    }

    /// 消费并返回内部的 `lopdf::Document`，供高级用途使用。
    #[must_use]
    pub fn into_inner(self) -> lopdf::Document {
        self.doc
    }

    /// 向文档添加可选内容组（PDF 图层）。
    ///
    /// 图层允许在 PDF 查看器中选择性显示或隐藏内容。
    ///
    /// # Errors
    ///
    /// 当无法修改 catalog 时返回 `PdfError::Parse`。
    pub fn add_layer(&mut self, name: &str) -> Result<lopdf::ObjectId> {
        // 创建 OCG 字典
        let mut ocg = lopdf::Dictionary::new();
        ocg.set("Type", lopdf::Object::Name(b"OCG".to_vec()));
        ocg.set(
            "Name",
            lopdf::Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
        let ocg_id = self.doc.add_object(lopdf::Object::Dictionary(ocg));

        // 添加到 catalog 的 /OCProperties
        if let Ok(catalog) = self.doc.catalog_mut() {
            let mut ocprops = lopdf::Dictionary::new();
            ocprops.set(
                "OCGs",
                lopdf::Object::Array(vec![lopdf::Object::Reference(ocg_id)]),
            );
            let mut d_dict = lopdf::Dictionary::new();
            d_dict.set(
                "Name",
                lopdf::Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
            d_dict.set(
                "OCGs",
                lopdf::Object::Array(vec![lopdf::Object::Reference(ocg_id)]),
            );
            ocprops.set("D", lopdf::Object::Dictionary(d_dict));
            catalog.set("OCProperties", lopdf::Object::Dictionary(ocprops));
        }
        Ok(ocg_id)
    }

    /// 验证 PDF/A-1b 合规性。
    #[must_use]
    pub fn validate_pdfa(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.doc.is_encrypted() {
            issues.push("Document is encrypted (PDF/A forbids encryption)".into());
        }
        let has_meta = self
            .doc
            .catalog()
            .ok()
            .and_then(|c| c.get(b"Metadata").ok())
            .is_some();
        if !has_meta {
            issues.push("Missing XMP metadata stream (required for PDF/A)".into());
        }
        for pid in self.doc.page_iter() {
            if let Ok(fonts) = self.doc.get_page_fonts(pid) {
                for (n, fd) in &fonts {
                    if !fd.has(b"FontFile") && !fd.has(b"FontFile2") && !fd.has(b"FontFile3") {
                        issues.push(format!("Font not embedded: {}", String::from_utf8_lossy(n)));
                    }
                }
            }
        }
        issues
    }
}

// --- 内部辅助函数 ---

fn root_pages_id(document: &lopdf::Document) -> Result<lopdf::ObjectId> {
    document
        .catalog()
        .and_then(|catalog| catalog.get(b"Pages"))
        .and_then(lopdf::Object::as_reference)
        .map_err(|error| PdfError::Parse(format!("invalid PDF page tree: {error}")))
}

fn append_page_tree(
    document: &mut lopdf::Document,
    destination_pages_id: lopdf::ObjectId,
    source_pages_id: lopdf::ObjectId,
    source_page_count: usize,
) -> Result<()> {
    let source_pages = document
        .get_object_mut(source_pages_id)
        .map_err(|error| PdfError::Parse(error.to_string()))?
        .as_dict_mut()
        .map_err(|error| PdfError::Parse(error.to_string()))?;
    source_pages.set("Parent", lopdf::Object::Reference(destination_pages_id));

    let destination_pages = document
        .get_object_mut(destination_pages_id)
        .map_err(|error| PdfError::Parse(error.to_string()))?
        .as_dict_mut()
        .map_err(|error| PdfError::Parse(error.to_string()))?;
    let current_count = destination_pages
        .get(b"Count")
        .and_then(lopdf::Object::as_i64)
        .map_err(|error| PdfError::Parse(error.to_string()))?;
    let source_count = i64::try_from(source_page_count)
        .map_err(|_| PdfError::Other("page count exceeds i64".to_string()))?;
    let kids = destination_pages
        .get_mut(b"Kids")
        .and_then(lopdf::Object::as_array_mut)
        .map_err(|error| PdfError::Parse(error.to_string()))?;
    kids.push(lopdf::Object::Reference(source_pages_id));
    destination_pages.set(
        "Count",
        lopdf::Object::Integer(current_count + source_count),
    );
    Ok(())
}

fn replace_page_tree_kids(
    document: &mut lopdf::Document,
    pages_id: lopdf::ObjectId,
    page_ids: &[lopdf::ObjectId],
) -> Result<()> {
    let pages = document
        .get_object_mut(pages_id)
        .map_err(|error| PdfError::Parse(error.to_string()))?
        .as_dict_mut()
        .map_err(|error| PdfError::Parse(error.to_string()))?;
    let count = i64::try_from(page_ids.len())
        .map_err(|_| PdfError::Other("page count exceeds i64".to_string()))?;
    pages.set("Count", lopdf::Object::Integer(count));
    pages.set(
        "Kids",
        lopdf::Object::Array(
            page_ids
                .iter()
                .copied()
                .map(lopdf::Object::Reference)
                .collect(),
        ),
    );
    Ok(())
}

fn save_document_atomically(mut document: lopdf::Document, path: impl AsRef<Path>) -> Result<()> {
    let mut bytes = Vec::new();
    document.save_to(&mut bytes)?;
    AtomicFileOutput::new(path.as_ref()).write(&bytes)
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::*;

    fn make_test_pdf(path: &std::path::Path) {
        let mut doc = lopdf::Document::new();
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));

        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages_dict.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages_dict.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));

        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn test_open_valid_pdf() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_manip_test.pdf");
        make_test_pdf(&path);
        assert!(PdfManipulator::open(&path).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_open_invalid_path() {
        assert!(PdfManipulator::open("/nonexistent/file.pdf").is_err());
    }

    #[test]
    fn test_rotate_invalid_page() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_manip_rot_invalid.pdf");
        make_test_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        assert!(m.rotate_page(99, Rotation::Clockwise90).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_manip_save_in.pdf");
        make_test_pdf(&path);
        let out = dir.join("easypdf_manip_save_out.pdf");
        PdfManipulator::open(&path).unwrap().save(&out).unwrap();
        assert!(out.exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_merge_empty() {
        let empty: &[&str] = &[];
        let result = PdfManipulator::merge_files(empty, "out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_two_files() {
        let dir = std::env::temp_dir();
        let path1 = dir.join("easypdf_merge1.pdf");
        let path2 = dir.join("easypdf_merge2.pdf");
        let out = dir.join("easypdf_merged.pdf");
        make_test_pdf(&path1);
        make_test_pdf(&path2);
        PdfManipulator::merge_files(&[&path1, &path2], &out).unwrap();
        let merged = lopdf::Document::load(&out).unwrap();
        assert_eq!(merged.get_pages().len(), 2);
        let _ = std::fs::remove_file(&path1);
        let _ = std::fs::remove_file(&path2);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_add_text_watermark() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_watermark.pdf");
        make_test_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        let result = m.add_text_watermark("DRAFT", 48.0, 0.3);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_extract_pages_valid() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_extract.pdf");
        make_test_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        let result = m.extract_pages(0..1).unwrap();
        assert_eq!(result.get_pages().len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_add_layer() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_layer_test.pdf");
        make_test_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        let result = m.add_layer("watermark");
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_validate_pdfa() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_pdfa_test.pdf");
        make_test_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        let issues = m.validate_pdfa();
        assert!(issues.iter().any(|issue| issue.contains("metadata")));
        let _ = std::fs::remove_file(&path);
    }

    // --- Additional coverage tests ---

    /// 创建 3 页测试 PDF。
    fn make_three_page_pdf(path: &std::path::Path) {
        let mut doc = lopdf::Document::new();
        let mut page_ids = Vec::new();

        for _ in 0..3 {
            let mut page_dict = lopdf::Dictionary::new();
            page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
            page_dict.set(
                "MediaBox",
                lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
            );
            page_ids.push(doc.add_object(lopdf::Object::Dictionary(page_dict)));
        }

        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages_dict.set(
            "Kids",
            lopdf::Object::Array(
                page_ids
                    .iter()
                    .map(|&id| lopdf::Object::Reference(id))
                    .collect(),
            ),
        );
        pages_dict.set("Count", lopdf::Object::Integer(3));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages_dict));

        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn test_rotate_page_success() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_rotate_success.pdf");
        make_test_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        assert!(m.rotate_page(1, Rotation::Clockwise90).is_ok());
        assert!(m.rotate_page(1, Rotation::Clockwise180).is_ok());
        assert!(m.rotate_page(1, Rotation::Clockwise270).is_ok());
        assert!(m.rotate_page(1, Rotation::None).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_rotate_page_zero_is_invalid() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_rotate_zero.pdf");
        make_test_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        // Page 0 is out of bounds (1-based).
        assert!(m.rotate_page(0, Rotation::Clockwise90).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_reorder_pages() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_reorder.pdf");
        make_three_page_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        // Reorder: page 3, 1, 2 (0-based indices: 2, 0, 1).
        assert!(m.reorder_pages(&[2, 0, 1]).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_reorder_pages_invalid_index() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_reorder_invalid.pdf");
        make_three_page_pdf(&path);
        let mut m = PdfManipulator::open(&path).unwrap();
        assert!(m.reorder_pages(&[0, 5]).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_extract_pages_out_of_range() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_extract_oor.pdf");
        make_test_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        assert!(m.extract_pages(0..5).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_page_count() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_page_count.pdf");
        make_three_page_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        assert_eq!(m.page_count(), 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_into_inner() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_into_inner.pdf");
        make_test_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        let doc = m.into_inner();
        assert_eq!(doc.get_pages().len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_merge_three_files() {
        let dir = std::env::temp_dir();
        let paths: Vec<_> = (0..3)
            .map(|i| dir.join(format!("easypdf_merge3_{i}.pdf")))
            .collect();
        let out = dir.join("easypdf_merged3.pdf");
        for p in &paths {
            make_test_pdf(p);
        }
        PdfManipulator::merge_files(&paths, &out).unwrap();
        let merged = lopdf::Document::load(&out).unwrap();
        assert_eq!(merged.get_pages().len(), 3);
        for p in &paths {
            let _ = std::fs::remove_file(p);
        }
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_merge_invalid_second_file() {
        let dir = std::env::temp_dir();
        let path1 = dir.join("easypdf_merge_ok.pdf");
        make_test_pdf(&path1);
        let bad_path = std::path::PathBuf::from("/nonexistent/file.pdf");
        let result = PdfManipulator::merge_files(&[&path1, &bad_path], "out.pdf");
        assert!(result.is_err());
        let _ = std::fs::remove_file(&path1);
    }

    #[test]
    fn test_extract_pages_three_page() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_extract3.pdf");
        make_three_page_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        let result = m.extract_pages(1..3).unwrap();
        assert_eq!(result.get_pages().len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_validate_pdfa_encrypted() {
        // A non-encrypted test PDF should not report encryption issue.
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_pdfa_nonenc.pdf");
        make_test_pdf(&path);
        let m = PdfManipulator::open(&path).unwrap();
        let issues = m.validate_pdfa();
        // Should NOT contain encryption issue (we didn't encrypt it).
        assert!(!issues.iter().any(|issue| issue.contains("encrypted")));
        let _ = std::fs::remove_file(&path);
    }
}
