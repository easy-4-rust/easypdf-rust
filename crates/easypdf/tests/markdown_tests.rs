//! `EasyPdf` PDF 到 Markdown 外观 API 的端到端测试。

#![cfg(feature = "markdown")]

use easypdf::{EasyPdf, MarkdownProfile, PdfFont};

#[test]
fn facade_exports_pdf_to_markdown() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("facade-input.pdf");
    let output = directory.path().join("facade-output.md");

    EasyPdf::create(&input)
        .add_text("Facade Markdown")
        .font(PdfFont::helvetica(14.0))
        .do_write()
        .expect("create PDF");
    let result = EasyPdf::export_markdown(&input, &output)
        .profile(MarkdownProfile::Llm)
        .do_export()
        .expect("export Markdown");

    let markdown = std::fs::read_to_string(&output).expect("read Markdown");
    assert!(markdown.contains("## Page 1"));
    assert!(markdown.contains("Facade Markdown"));
    assert_eq!(result.report().pages_read(), 1);
}

#[test]
fn facade_converts_pdf_to_markdown_in_memory() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("facade-memory-input.pdf");

    EasyPdf::create(&input)
        .add_text("In-memory Markdown")
        .font(PdfFont::helvetica(14.0))
        .do_write()
        .expect("create PDF");
    let result = EasyPdf::to_markdown(&input)
        .profile(MarkdownProfile::Llm)
        .do_convert()
        .expect("convert Markdown");

    assert!(result.markdown().contains("## Page 1"));
    assert!(result.markdown().contains("In-memory Markdown"));
    assert_eq!(result.report().pages_read(), 1);
    assert_eq!(result.to_string(), result.markdown());
}
