//! PDF 页面渲染为光栅图像。
//!
//! 本模块提供基于 trait 的抽象（[`PdfRenderer`]）用于将 PDF 页面渲染为
//! 光栅图像（PNG、JPEG）。两个可用后端：
//!
//! - **`TextRenderer`**（默认，纯 Rust）——通过 [`easypdf_reader::PdfReader`]
//!   提取文本并渲染为简单位图图像。质量较低但足以满足 OCR 流水线需求，无外部依赖。
//!
//! - **`PdfiumRenderer`**（feature `pdfium`）——使用 Google `PDFium` 进行
//!   高质量渲染，运行时需要 `libpdfium` 动态库。
//!
//! # 快速开始
//!
//! ```no_run
//! use easypdf_markdown::render::{render_page_to_png, render_all_pages_to_dir};
//!
//! // 以 150 DPI 渲染第 0 页：
//! render_page_to_png("input.pdf".as_ref(), 0, "page_0.png".as_ref(), 150)?;
//!
//! // 将全部页面渲染到目录：
//! let paths = render_all_pages_to_dir("input.pdf".as_ref(), "output/".as_ref(), 150)?;
//! # Ok::<(), easypdf_markdown::render::RenderError>(())
//! ```
//!
//! # 选择后端
//!
//! 使用 [`RenderBackend::default_backend`] 自动选择最佳可用后端，或手动指定：
//!
//! ```no_run
//! use easypdf_markdown::render::RenderBackend;
//!
//! let renderer = RenderBackend::default_backend()
//!     .build_renderer("document.pdf".as_ref())?;
//! # Ok::<(), easypdf_markdown::render::RenderError>(())
//! ```

pub mod backend;
pub mod backends;
pub mod config;
mod convenience;
pub mod error;
pub mod traits;

// --- 公共类型重导出 ---
pub use backend::RenderBackend;
pub use config::{Background, ImageFormat, RenderConfig};
pub use convenience::{render_all_pages_to_dir, render_page, render_page_to_png};
pub use error::{RenderError, Result};
pub use traits::{PdfRenderer, RenderedImage};

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names, clippy::float_cmp)]
    use super::*;

    /// 辅助函数：在内存中构建一个最小有效 PDF。
    fn make_test_pdf_bytes() -> Vec<u8> {
        let mut doc = lopdf::Document::new();
        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf 72 700 Td (Hello) Tj ET".to_vec(),
        )));
        let mut font_dict = lopdf::Dictionary::new();
        font_dict.set("Type", lopdf::Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
        let font_id = doc.add_object(lopdf::Object::Dictionary(font_dict));
        let mut resources = lopdf::Dictionary::new();
        let mut fonts = lopdf::Dictionary::new();
        fonts.set("F1", lopdf::Object::Reference(font_id));
        resources.set("Font", lopdf::Object::Dictionary(fonts));
        let resources_id = doc.add_object(lopdf::Object::Dictionary(resources));
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        page_dict.set("Resources", lopdf::Object::Reference(resources_id));
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
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_render_page_to_png_convenience() {
        let bytes = make_test_pdf_bytes();
        let dir = std::env::temp_dir().join("easypdf_render_lib_test");
        let _ = std::fs::remove_dir_all(&dir);
        // 将 PDF 写入临时文件（便捷 API 需要路径）。
        let pdf_path = dir.join("test.pdf");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&pdf_path, &bytes).unwrap();

        let output = dir.join("page_0.png");
        render_page_to_png(&pdf_path, 0, &output, 72).unwrap();
        assert!(output.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_render_all_pages_to_dir() {
        let bytes = make_test_pdf_bytes();
        let dir = std::env::temp_dir().join("easypdf_render_all_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pdf_path = dir.join("test.pdf");
        std::fs::write(&pdf_path, &bytes).unwrap();

        let out_dir = dir.join("output");
        let paths = render_all_pages_to_dir(&pdf_path, &out_dir, 72).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_render_page_in_memory() {
        let bytes = make_test_pdf_bytes();
        let dir = std::env::temp_dir().join("easypdf_render_mem_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pdf_path = dir.join("test.pdf");
        std::fs::write(&pdf_path, &bytes).unwrap();

        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = render_page(&pdf_path, 0, &config).unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_default_backend_is_text() {
        let backend = RenderBackend::default_backend();
        // 未启用 pdfium feature 时，默认应为 Text。
        assert_eq!(backend, RenderBackend::Text);
    }

    #[test]
    fn test_backend_display() {
        assert_eq!(RenderBackend::Text.to_string(), "text");
        assert_eq!(RenderBackend::Pdfium.to_string(), "pdfium");
    }

    #[test]
    fn test_text_backend_is_available() {
        assert!(RenderBackend::Text.is_available());
    }

    #[test]
    fn test_pdfium_backend_not_available_without_feature() {
        // 未启用 pdfium feature 时，Pdfium 应不可用。
        #[cfg(not(feature = "pdfium"))]
        assert!(!RenderBackend::Pdfium.is_available());
    }
}
