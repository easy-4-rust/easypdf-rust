//! PDF 创建与写入（printpdf 后端）。
//!
//! 提供 `PdfWriter` 用于创建包含文本、表格、图片、图形和自定义字体的
//! 新 PDF 文档。底层使用 `printpdf` crate。
//!
//! # 写入后端
//!
//! [`WriteBackend`] 枚举控制已完成页面的存储方式：
//!
//! - [`InMemory`](WriteBackend::InMemory) -- 默认，适合小型文档。
//! - [`Spill`](WriteBackend::Spill) -- 页面级溢出到临时文件，适合大型文档。
//!
//! 使用 [`PdfWriterBuilder`] 配置后端、处理器优先级和常量内存模式。
//!
//! # Examples
//!
//! ```
//! use easypdf_writer::{PdfWriter, PdfWriterBuilder, WriteBackend};
//! use easypdf_core::*;
//!
//! // 简单用法（向后兼容）。
//! let mut w = PdfWriter::new("title");
//!
//! // 使用溢出后端的构建器。
//! let w = PdfWriterBuilder::new("Big Report")
//!     .backend(WriteBackend::auto(500))
//!     .build()
//!     .unwrap();
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_borrow,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::needless_pass_by_value
)]

mod backend;
mod builder;
mod font;
mod image;
mod shape;
mod template;
mod writer;

pub use backend::WriteBackend;
pub use builder::PdfWriterBuilder;
pub use font::map_builtin_font;
pub use template::PdfTemplateFiller;
pub use writer::PdfWriter;

#[cfg(test)]
#[allow(clippy::redundant_closure_for_method_calls, clippy::needless_borrow)]
mod tests {
    use super::*;
    use easypdf_core::*;

    fn make_test_png() -> Vec<u8> {
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ]
    }

    #[test]
    fn test_writer_new() {
        let w = PdfWriter::new("t");
        assert_eq!(w.current_page_number(), 0);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_add_page() {
        let mut w = PdfWriter::new("t");
        assert_eq!(w.add_page(PageSize::A4, Orientation::Portrait).unwrap(), 1);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_multi_page() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_text(
            &PdfText::new("P1").font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        )
        .unwrap();
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_text(
            &PdfText::new("P2").font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        )
        .unwrap();
        assert!(w.page_count() >= 1);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_finish_creates_file() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_text(
            &PdfText::new("H").font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        )
        .unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_tf.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_finish_empty_document_produces_one_page() {
        let mut w = PdfWriter::new("e");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_fe.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_write_image() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let i = PdfImage {
            data: make_test_png(),
            width: 0.0,
            height: 0.0,
        };
        w.write_image(&i, 100.0, 600.0, 50.0, 50.0).unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_wi.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_write_image_natural_size() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let i = PdfImage {
            data: make_test_png(),
            width: 0.0,
            height: 0.0,
        };
        w.write_image(&i, 50.0, 700.0, 0.0, 0.0).unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_wn.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_draw_line() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.draw_line(10.0, 10.0, 200.0, 10.0, 1.0);
        let d = std::env::temp_dir();
        let p = d.join("ew_dl.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_draw_rect_stroke() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.draw_rect_stroke(50.0, 600.0, 200.0, 100.0, 1.0);
        let d = std::env::temp_dir();
        let p = d.join("ew_drs.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_draw_circle() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.draw_circle(300.0, 400.0, 100.0, 1.0);
        let d = std::env::temp_dir();
        let p = d.join("ew_dc.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_invalid_image_data() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let i = PdfImage {
            data: vec![0, 1, 2, 3],
            width: 0.0,
            height: 0.0,
        };
        assert!(w.write_image(&i, 0.0, 0.0, 100.0, 100.0).is_err());
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_all_builtin_fonts() {
        for f in &[
            BuiltInFont::TimesBoldItalic,
            BuiltInFont::CourierBold,
            BuiltInFont::CourierOblique,
            BuiltInFont::HelveticaBoldOblique,
            BuiltInFont::TimesBold,
            BuiltInFont::TimesItalic,
            BuiltInFont::CourierBoldOblique,
            BuiltInFont::ZapfDingbats,
        ] {
            let mut w = PdfWriter::new("t");
            w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
            let ff = PdfFont {
                family: FontFamily::BuiltIn(*f),
                size: 10.0,
                style: Default::default(),
            };
            w.write_text(&PdfText::new("x").font(ff), 100.0, 700.0)
                .unwrap();
        }
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_empty_finish() {
        let mut w = PdfWriter::new("e");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_ef.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_custom_font_fallback_bold() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let f = PdfFont {
            family: FontFamily::Custom("x.ttf".into()),
            size: 12.0,
            style: FontStyle {
                bold: true,
                italic: false,
            },
        };
        w.write_text(&PdfText::new("x").font(f), 100.0, 700.0)
            .unwrap();
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_custom_font_not_registered() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        assert!(
            w.write_text_with_custom_font("h", "nx", 12.0, 100.0, 700.0)
                .is_err()
        );
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_write_svg() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="red"/></svg>"#, 100.0, 600.0, 100.0, 100.0).unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_svg.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_write_svg_invalid() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        assert!(w.write_svg("not svg", 100.0, 600.0, 100.0, 100.0).is_err());
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_write_text_with_symbol_font() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let f = PdfFont {
            family: FontFamily::BuiltIn(BuiltInFont::Symbol),
            size: 12.0,
            style: Default::default(),
        };
        w.write_text(&PdfText::new("t").font(f), 100.0, 700.0)
            .unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_sym.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_metadata_chaining() {
        let w = PdfWriter::new("t").metadata(PdfMetadata::new().title("T").author("A"));
        assert_eq!(w.metadata.title.as_deref(), Some("T"));
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_page_size_dimensions() {
        assert_eq!(PageSize::A4.dimensions(), (595.0, 842.0));
        assert_eq!(PageSize::Letter.dimensions(), (612.0, 792.0));
        assert_eq!(PageSize::Custom(100.0, 200.0).dimensions(), (100.0, 200.0));
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_register_font_from_nonexistent_path() {
        let mut w = PdfWriter::new("t");
        assert!(w.register_font_from_path("/nonexistent/f.ttf").is_err());
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_register_font_success() {
        let mut w = PdfWriter::new("t");
        let p = "/System/Library/Fonts/Helvetica.ttc";
        if std::path::Path::new(p).exists() {
            assert!(w.register_font_from_path(p).is_ok());
            w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
            assert!(
                w.write_text_with_custom_font("CF!", p, 14.0, 100.0, 600.0)
                    .is_ok()
            );
        }
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_register_handler_direct() {
        struct H;
        impl PdfWriteHandler for H {}
        let _ = PdfWriter::new("t").register_handler(Box::new(H));
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_register_font_from_bytes_error() {
        let mut w = PdfWriter::new("t");
        assert!(w.register_font_from_bytes("bad", &[0, 1, 2]).is_err());
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_new_from_writer() {
        let mut w = PdfWriter::new_from_writer(Vec::new());
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.add_text(&PdfFont::helvetica(12.0), "Hello stream")
            .unwrap();
        w.flush().unwrap();
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_add_text_convenience() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.add_text(&PdfFont::helvetica(14.0), "L1").unwrap();
        w.add_text(&PdfFont::times_roman(12.0), "L2").unwrap();
        w.add_text_colored(&PdfFont::helvetica(12.0), &PdfColor::red(), "R")
            .unwrap();
        let d = std::env::temp_dir();
        let p = d.join("ew_at.pdf");
        w.finish(&p).unwrap();
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_add_image_from_path() {
        let mut w = PdfWriter::new("t");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let d = std::env::temp_dir();
        let ip = d.join("ew_ti.png");
        std::fs::write(&ip, make_test_png()).unwrap();
        w.add_image_from_path(ip.clone(), 50.0, 50.0).unwrap();
        let op = d.join("ew_ai.pdf");
        w.finish(&op).unwrap();
        assert!(op.exists());
        let _ = std::fs::remove_file(&ip);
        let _ = std::fs::remove_file(&op);
    }
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    #[test]
    fn test_flush_to_writer() {
        let mut w = PdfWriter::new_from_writer(Vec::new());
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.add_text(&PdfFont::helvetica(10.0), "F!").unwrap();
        w.flush().unwrap();
    }

    #[test]
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    fn test_write_handler_lifecycle_order() {
        use std::sync::{Arc, Mutex};

        struct RecordingHandler(Arc<Mutex<Vec<String>>>);
        impl PdfWriteHandler for RecordingHandler {
            fn before_document(&mut self) -> easypdf_core::Result<()> {
                self.0.lock().unwrap().push("before_document".to_string());
                Ok(())
            }

            fn before_page(&mut self, page_number: usize) -> easypdf_core::Result<()> {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("before_page:{page_number}"));
                Ok(())
            }

            fn after_page(&mut self, page_number: usize) -> easypdf_core::Result<()> {
                self.0
                    .lock()
                    .unwrap()
                    .push(format!("after_page:{page_number}"));
                Ok(())
            }

            fn after_document(&mut self) -> easypdf_core::Result<()> {
                self.0.lock().unwrap().push("after_document".to_string());
                Ok(())
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut writer = PdfWriter::new("lifecycle")
            .register_handler(Box::new(RecordingHandler(Arc::clone(&events))));
        writer
            .add_page(PageSize::A4, Orientation::Portrait)
            .unwrap();
        writer
            .add_page(PageSize::A4, Orientation::Portrait)
            .unwrap();
        let output = std::env::temp_dir().join("easypdf_writer_lifecycle.pdf");
        writer.finish(&output).unwrap();
        let actual = events.lock().unwrap().clone();
        assert_eq!(
            actual,
            [
                "before_document",
                "before_page:1",
                "after_page:1",
                "before_page:2",
                "after_page:2",
                "after_document",
            ]
        );
        let _ = std::fs::remove_file(output);
    }

    #[test]
    #[allow(
        clippy::redundant_closure_for_method_calls,
        clippy::needless_borrow,
        clippy::redundant_clone
    )]
    fn test_landscape_orientation_swaps_page_dimensions() {
        let output = std::env::temp_dir().join("easypdf_writer_landscape.pdf");
        let mut writer = PdfWriter::new("landscape");
        writer
            .add_page(PageSize::A4, Orientation::Landscape)
            .unwrap();
        writer.finish(&output).unwrap();

        let document = lopdf::Document::load(&output).unwrap();
        let page_id = *document.get_pages().values().next().unwrap();
        let page = document.get_object(page_id).unwrap().as_dict().unwrap();
        let media_box = page.get(b"MediaBox").unwrap().as_array().unwrap();
        let width = media_box[2].as_float().unwrap();
        let height = media_box[3].as_float().unwrap();
        assert!(width > height);
        let _ = std::fs::remove_file(output);
    }

    // --- New tests for builder, backend, and spill ---

    #[test]
    fn test_builder_basic() {
        let w = PdfWriterBuilder::new("test").build().unwrap();
        assert_eq!(w.current_page_number(), 0);
        assert!(!w.is_constant_memory());
    }

    #[test]
    fn test_builder_with_metadata() {
        let w = PdfWriterBuilder::new("test")
            .metadata(PdfMetadata::new().title("T"))
            .build()
            .unwrap();
        assert_eq!(w.metadata_title(), Some("T"));
    }

    #[test]
    fn test_builder_constant_memory() {
        let w = PdfWriterBuilder::new("test")
            .constant_memory(true)
            .build()
            .unwrap();
        assert!(w.is_constant_memory());
    }

    #[test]
    fn test_builder_register_handler_with_priority() {
        struct NoopHandler;
        impl PdfWriteHandler for NoopHandler {}
        let w = PdfWriterBuilder::new("test")
            .register_handler_with_priority(Box::new(NoopHandler), 5.0)
            .build()
            .unwrap();
        assert_eq!(w.handler_count(), 1);
    }

    #[test]
    fn test_set_constant_memory() {
        let mut w = PdfWriter::new("t");
        assert!(!w.is_constant_memory());
        w.set_constant_memory(true);
        assert!(w.is_constant_memory());
        w.set_constant_memory(false);
        assert!(!w.is_constant_memory());
    }

    #[test]
    fn test_handler_count() {
        struct H;
        impl PdfWriteHandler for H {}
        let w = PdfWriter::new("t")
            .register_handler(Box::new(H))
            .register_handler(Box::new(H));
        assert_eq!(w.handler_count(), 2);
    }

    #[test]
    fn test_register_handler_with_priority_api() {
        struct H;
        impl PdfWriteHandler for H {}
        let w = PdfWriter::new("t").register_handler_with_priority(Box::new(H), 0.5);
        assert_eq!(w.handler_count(), 1);
    }

    #[test]
    fn test_write_backend_auto() {
        assert_eq!(WriteBackend::auto(10), WriteBackend::InMemory);
        assert!(WriteBackend::auto(200).is_constant_memory());
    }

    #[test]
    fn test_spill_finish_produces_valid_pdf() {
        let d = std::env::temp_dir();
        let p = d.join("ew_spill.pdf");
        let mut w = PdfWriterBuilder::new("spill-test")
            .constant_memory(true)
            .build()
            .unwrap();
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_text(
            &PdfText::new("Spilled!").font(PdfFont::helvetica(14.0)),
            100.0,
            700.0,
        )
        .unwrap();
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.write_text(
            &PdfText::new("Page 2").font(PdfFont::helvetica(14.0)),
            100.0,
            700.0,
        )
        .unwrap();
        w.finish(&p).unwrap();
        assert!(p.exists());
        // Verify the PDF is valid by loading it.
        let doc = lopdf::Document::load(&p).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_spill_finish_with_handler_lifecycle() {
        use std::sync::{Arc, Mutex};

        struct RecordingHandler(Arc<Mutex<Vec<String>>>);
        impl PdfWriteHandler for RecordingHandler {
            fn before_document(&mut self) -> easypdf_core::Result<()> {
                self.0.lock().unwrap().push("before_document".into());
                Ok(())
            }
            fn before_page(&mut self, n: usize) -> easypdf_core::Result<()> {
                self.0.lock().unwrap().push(format!("before_page:{n}"));
                Ok(())
            }
            fn after_page(&mut self, n: usize) -> easypdf_core::Result<()> {
                self.0.lock().unwrap().push(format!("after_page:{n}"));
                Ok(())
            }
            fn after_document(&mut self) -> easypdf_core::Result<()> {
                self.0.lock().unwrap().push("after_document".into());
                Ok(())
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut w = PdfWriterBuilder::new("spill-lifecycle")
            .constant_memory(true)
            .register_handler(Box::new(RecordingHandler(Arc::clone(&events))))
            .build()
            .unwrap();
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let p = std::env::temp_dir().join("ew_spill_lifecycle.pdf");
        w.finish(&p).unwrap();

        let actual = events.lock().unwrap().clone();
        assert_eq!(
            actual,
            [
                "before_document",
                "before_page:1",
                "after_page:1",
                "before_page:2",
                "after_page:2",
                "after_document",
            ]
        );
        let _ = std::fs::remove_file(&p);
    }
}
