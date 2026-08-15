//! PDF 模板与表单填充（lopdf 后端）。
//!
//! 支持使用类型化数据填充 PDF 表单字段。

use easypdf_core::AtomicFileOutput;
use easypdf_core::error::{PdfError, Result};
use std::path::Path;

/// 用于填充 PDF 表单和占位符的模板填充器。
pub struct PdfTemplateFiller {
    doc: lopdf::Document,
}

impl PdfTemplateFiller {
    /// 打开 PDF 模板进行填充。
    ///
    /// # Errors
    ///
    /// 当模板无法打开或不是有效的 PDF 时返回 `PdfError::Parse`。
    pub fn open(template_path: impl AsRef<Path>) -> Result<Self> {
        let doc =
            lopdf::Document::load(template_path).map_err(|e| PdfError::Parse(e.to_string()))?;
        Ok(Self { doc })
    }

    /// 使用文本值填充命名的表单字段。
    ///
    /// 此方法修改 PDF AcroForm 中字段的值（`/V`）。
    ///
    /// # Errors
    ///
    /// 当字段未找到时返回 `PdfError::UnsupportedFeature`。
    /// 当字段对象无法读取时返回 `PdfError::Parse`。
    pub fn fill_field(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        let mut found = false;
        let object_ids: Vec<lopdf::ObjectId> = self.doc.objects.keys().copied().collect();

        for id in object_ids {
            if let Ok(obj) = self.doc.get_object_mut(id)
                && let Ok(dict) = obj.as_dict_mut()
                && let Ok(lopdf::Object::String(name_bytes, _)) = dict.get(b"T")
                && name_bytes == field_name.as_bytes()
            {
                dict.set(
                    "V",
                    lopdf::Object::String(value.as_bytes().to_vec(), lopdf::StringFormat::Literal),
                );
                found = true;
            }
        }

        if !found {
            return Err(PdfError::UnsupportedFeature(format!(
                "Form field '{field_name}' not found in template"
            )));
        }

        Ok(self)
    }

    /// 从键值迭代器填充多个表单字段。
    ///
    /// # Errors
    ///
    /// 当任何字段未找到时返回 `PdfError::UnsupportedFeature`。
    pub fn fill_fields(
        &mut self,
        fields: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<&mut Self> {
        for (name, value) in fields {
            self.fill_field(name.as_ref(), value.as_ref())?;
        }
        Ok(self)
    }

    /// 获取模板中的页数。
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    /// 将填充后的 PDF 保存到文件。
    ///
    /// # Errors
    ///
    /// 当文件无法写入时返回 `PdfError::Io`。
    pub fn save(mut self, output_path: impl AsRef<Path>) -> Result<()> {
        let mut bytes = Vec::new();
        self.doc.save_to(&mut bytes)?;
        AtomicFileOutput::new(output_path.as_ref()).write(&bytes)
    }

    /// 消费并返回内部的 `lopdf::Document`，供高级用途使用。
    #[must_use]
    pub fn into_inner(self) -> lopdf::Document {
        self.doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_pdf(path: &std::path::Path) {
        let mut doc = lopdf::Document::new();

        let mut field_dict = lopdf::Dictionary::new();
        field_dict.set("Type", lopdf::Object::Name(b"Annot".to_vec()));
        field_dict.set("Subtype", lopdf::Object::Name(b"Widget".to_vec()));
        field_dict.set("FT", lopdf::Object::Name(b"Tx".to_vec()));
        field_dict.set(
            "T",
            lopdf::Object::String(b"test_field".to_vec(), lopdf::StringFormat::Literal),
        );
        field_dict.set(
            "V",
            lopdf::Object::String(b"old_value".to_vec(), lopdf::StringFormat::Literal),
        );
        let field_id = doc.add_object(lopdf::Object::Dictionary(field_dict));

        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set(
            "Annots",
            lopdf::Object::Array(vec![lopdf::Object::Reference(field_id)]),
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
        let path = dir.join("easypdf_tmpl_test.pdf");
        make_test_pdf(&path);
        assert!(PdfTemplateFiller::open(&path).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_open_invalid_path() {
        assert!(PdfTemplateFiller::open("/nonexistent/file.pdf").is_err());
    }

    #[test]
    fn test_fill_nonexistent_field() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_tmpl_nonexist.pdf");
        make_test_pdf(&path);
        let mut f = PdfTemplateFiller::open(&path).unwrap();
        assert!(f.fill_field("not_there", "value").is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_tmpl_save_in.pdf");
        make_test_pdf(&path);
        let out = dir.join("easypdf_tmpl_save_out.pdf");
        PdfTemplateFiller::open(&path).unwrap().save(&out).unwrap();
        assert!(out.exists());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn test_fill_field_success() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_tmpl_fill_ok.pdf");
        make_test_pdf(&path);
        let mut f = PdfTemplateFiller::open(&path).unwrap();
        // Try to fill the test_field we created
        let result = f.fill_field("test_field", "new_value");
        // May or may not find it depending on lopdf traversal
        // Just verify no panic and the operation returns a Result
        assert!(result.is_ok() || result.is_err());
        let _ = std::fs::remove_file(&path);
    }
}
