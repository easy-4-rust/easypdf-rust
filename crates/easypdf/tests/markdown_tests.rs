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
