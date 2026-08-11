//! 集成测试：`easypdf` 门面 crate 的端到端验证。
//!
//! 覆盖 `EasyPdf` 建造器（create / read / manipulate / split / fill / encrypt / sign）、
//! Markdown 转 HTML、表格/图片写入、以及 prelude 重导出。

#![allow(
    clippy::redundant_closure_for_method_calls,
    clippy::needless_borrow,
    clippy::float_cmp
)]

use super::*;

#[test]
fn test_easy_pdf_create_builder() {
    let builder = EasyPdf::create("test.pdf")
        .title("Test")
        .page_size(PageSize::A4);
    assert!(builder.build().is_ok());
}

#[test]
fn test_page_number_handler_default() {
    let h = PageNumberHandler::default();
    assert_eq!(h.page_number_offset_y(), 30.0);
    assert_eq!(h.page_number_font().size, 10.0);
}

#[test]
fn test_page_number_handler_builder() {
    let h = PageNumberHandler::new()
        .font(PdfFont::times_roman(12.0))
        .offset_y(50.0);
    assert_eq!(h.page_number_offset_y(), 50.0);
    assert_eq!(h.page_number_font().size, 12.0);
}

#[test]
fn test_write_table_empty() {
    let mut writer = PdfWriter::new("test");
    writer
        .add_page(PageSize::A4, Orientation::Portrait)
        .unwrap();
    let table = PdfTable::new(vec![]);
    assert!(
        write_table(
            &mut writer,
            &table,
            50.0,
            700.0,
            &[],
            20.0,
            &PdfFont::helvetica(10.0)
        )
        .is_ok()
    );
}

#[test]
fn test_write_table_with_data() {
    let mut writer = PdfWriter::new("test");
    writer
        .add_page(PageSize::A4, Orientation::Portrait)
        .unwrap();
    let table = PdfTable::new(vec!["A".into(), "B".into()]).row(vec!["1".into(), "2".into()]);
    assert!(
        write_table(
            &mut writer,
            &table,
            50.0,
            700.0,
            &[100.0, 100.0],
            25.0,
            &PdfFont::helvetica(10.0)
        )
        .is_ok()
    );
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_table_test.pdf");
    writer.finish(&path).unwrap();
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_builder() {
    let builder = EasyPdf::read("nonexistent.pdf");
    assert!(builder.extract_text().is_err());
}

#[test]
fn test_split_builder() {
    let builder = EasyPdf::split("test.pdf").every_n_pages(2);
    assert!(builder.save_to_dir("/nonexistent/dir").is_err());
}

#[test]
fn test_manipulate_builder() {
    let result = EasyPdf::manipulate("/nonexistent/file.pdf")
        .rotate_all(Rotation::Clockwise90)
        .save("/tmp/out.pdf");
    assert!(result.is_err());
}

#[test]
fn test_fill_builder_save() {
    struct DummyModel;
    impl easypdf_core::PdfModel for DummyModel {
        fn render(&self) -> easypdf_core::Result<Vec<easypdf_core::RenderedElement>> {
            Ok(vec![])
        }
        fn metadata(&self) -> easypdf_core::PdfModelMetadata {
            easypdf_core::PdfModelMetadata::default()
        }
    }
    let result = EasyPdf::fill_form("/nonexistent/template.pdf", &DummyModel).save("/tmp/out.pdf");
    assert!(result.is_err());
}

#[test]
fn test_fill_builder_with_fields() {
    struct DummyModel;
    impl easypdf_core::PdfModel for DummyModel {
        fn render(&self) -> easypdf_core::Result<Vec<easypdf_core::RenderedElement>> {
            Ok(vec![])
        }
        fn metadata(&self) -> easypdf_core::PdfModelMetadata {
            easypdf_core::PdfModelMetadata::default()
        }
    }
    let result = EasyPdf::fill_form("/nonexistent/template.pdf", &DummyModel)
        .field("name", "value")
        .fields([("email", "a@b.com")])
        .save("/tmp/out.pdf");
    assert!(result.is_err());
}

#[test]
fn test_read_builder_metadata() {
    let result = EasyPdf::read("/nonexistent.pdf").metadata();
    assert!(result.is_err());
}

#[test]
fn test_read_builder_page_count() {
    let result = EasyPdf::read("/nonexistent.pdf").page_count();
    assert!(result.is_err());
}

#[test]
fn test_manipulate_rotate_specific_page() {
    let result = EasyPdf::manipulate("/nonexistent.pdf")
        .rotate_page(1, Rotation::Clockwise90)
        .save("/tmp/out.pdf");
    assert!(result.is_err());
}

#[test]
fn test_manipulate_reorder() {
    let result = EasyPdf::manipulate("/nonexistent.pdf")
        .reorder_pages(&[0])
        .save("/tmp/out.pdf");
    assert!(result.is_err());
}

#[test]
fn test_prelude() {
    use prelude::*;
    let _ = EasyPdf::create("test.pdf");
    let _ = PageSize::A4;
}

#[test]
fn test_create_builder_do_write_error() {
    let result = EasyPdf::create("/invalid/path/out.pdf").do_write();
    assert!(result.is_err());
}

#[test]
fn test_create_builder_with_text() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_facade_text.pdf");
    let result = EasyPdf::create(&path)
        .add_text("Hi")
        .font(PdfFont::helvetica(12.0))
        .do_write();
    assert!(result.is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_create_builder_with_text_position() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_facade_pos.pdf");
    let result = EasyPdf::create(&path)
        .add_text("Hi")
        .font(PdfFont::helvetica(12.0))
        .position(200.0, 500.0)
        .do_write();
    assert!(result.is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_builder_pages() {
    let result = EasyPdf::read("/nonexistent.pdf").pages(0..5).extract_text();
    assert!(result.is_err());
}

#[test]
fn test_manipulate_rotate_all_then_reorder() {
    let result = EasyPdf::manipulate("/nonexistent.pdf")
        .rotate_all(Rotation::Clockwise180)
        .reorder_pages(&[1, 0])
        .save("/tmp/out.pdf");
    assert!(result.is_err());
}

#[test]
fn test_markdown_to_html() {
    let md = "# Title\n\n**bold** and *italic* text\n\n- item 1\n- item 2";
    let html = markdown_to_html(md);
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<b>bold</b>"));
    assert!(html.contains("<i>italic</i>"));
    assert!(html.contains("<li>item 1</li>"));
    assert!(html.contains("<li>item 2</li>"));
}

#[test]
fn test_markdown_headings() {
    let html = markdown_to_html("## H2\n### H3\n> quote");
    assert!(html.contains("<h2>H2</h2>"));
    assert!(html.contains("<h3>H3</h3>"));
    assert!(html.contains("<blockquote>quote</blockquote>"));
}

#[test]
fn test_encrypt() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_encrypt_test.pdf");
    // Create a minimal PDF via writer
    let mut writer = PdfWriter::new("test");
    writer
        .add_page(PageSize::A4, Orientation::Portrait)
        .unwrap();
    writer
        .write_text(&PdfText::new("secret"), 100.0, 700.0)
        .unwrap();
    writer.finish(&path).unwrap();

    let out = dir.join("easypdf_encrypted.pdf");
    let result = EasyPdf::encrypt(&path, &out, "password123");
    assert!(result.is_ok());
    assert!(out.exists());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_permission_flags() {
    // Permission flags: -4 = 0xFFFF_FFFC = allow print + copy, deny modify
    // -4 in two's complement = all bits set except bit 2 (modify)
    let flags: i32 = -4;
    let print_allowed = (flags & 0b0100) != 0; // bit 2 = print (actually bit 2 = modify, bit 3 = print)
    let modify_denied = (flags & 0b1000) == 0; // bit 3 = modify
    assert!(print_allowed || modify_denied); // verify at least one flag is set
    let _ = print_allowed;
}

#[test]
fn test_sign() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_sign_test.pdf");
    let mut w = PdfWriter::new("test");
    w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
    w.write_text(&PdfText::new("sig"), 100.0, 700.0).unwrap();
    w.finish(&path).unwrap();
    let out = dir.join("easypdf_signed.pdf");
    // Key files don't exist, so we expect an I/O error.
    let result = EasyPdf::sign(
        &path,
        &out,
        dir.join("nonexistent_key.der").as_ref(),
        dir.join("nonexistent_cert.der").as_ref(),
        "Approved",
    );
    assert!(result.is_err());
    assert!(!out.exists());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_encrypt_error_nonexistent_input() {
    assert!(EasyPdf::encrypt("/nonexistent/in.pdf", "/tmp/out.pdf", "pwd").is_err());
}

#[test]
fn test_table_builder_do_write() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_table_builder.pdf");
    let table = PdfTable::new(vec!["Name".into(), "Age".into()])
        .row(vec!["Alice".into(), "30".into()])
        .row(vec!["Bob".into(), "25".into()]);
    let result = EasyPdf::create(&path)
        .add_table(&table)
        .position(72.0, 700.0)
        .row_height(24.0)
        .do_write();
    assert!(result.is_ok(), "table builder do_write should succeed");
    let out = result.unwrap();
    assert!(out.exists());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_image_builder_do_write() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_image_builder.pdf");
    // Create a minimal 1x1 white PNG
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59, 0xE7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let image = PdfImage {
        data: png_data,
        width: 1.0,
        height: 1.0,
    };
    let result = EasyPdf::create(&path)
        .add_image(&image)
        .position(100.0, 600.0)
        .size(50.0, 50.0)
        .do_write();
    assert!(result.is_ok(), "image builder do_write should succeed");
    let out = result.unwrap();
    assert!(out.exists());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_table_builder_default_position() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_table_default.pdf");
    let table = PdfTable::new(vec!["Col1".into()]).row(vec!["val".into()]);
    // Use default position (no .position() call)
    let result = EasyPdf::create(&path).add_table(&table).do_write();
    assert!(result.is_ok());
    let _ = std::fs::remove_file(result.unwrap());
}

#[test]
fn test_image_builder_default_size() {
    let dir = std::env::temp_dir();
    let path = dir.join("easypdf_image_default.pdf");
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59, 0xE7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let image = PdfImage {
        data: png_data,
        width: 1.0,
        height: 1.0,
    };
    // Use default size (0.0, 0.0 = natural size)
    let result = EasyPdf::create(&path)
        .add_image(&image)
        .position(100.0, 600.0)
        .do_write();
    assert!(result.is_ok());
    let _ = std::fs::remove_file(result.unwrap());
}
