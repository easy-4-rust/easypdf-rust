//! 通过公开门面验证 PDF 写入、读取与 Markdown 转换的组合工作流。

use easypdf::{EasyPdf, MarkdownProfile, PdfFont};

#[test]
fn create_read_and_convert_in_one_public_workflow() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let pdf_path = directory.path().join("workflow.pdf");

    EasyPdf::create(&pdf_path)
        .title("Workflow")
        .add_text("Easy PDF workflow")
        .font(PdfFont::helvetica(14.0))
        .do_write()
        .expect("write PDF through public facade");

    assert_eq!(
        EasyPdf::read(&pdf_path)
            .page_count()
            .expect("read page count"),
        1
    );
    let markdown = EasyPdf::to_markdown(&pdf_path)
        .profile(MarkdownProfile::Llm)
        .do_convert()
        .expect("convert through public facade");
    assert!(markdown.markdown().contains("Easy PDF workflow"));
    assert_eq!(markdown.report().pages_read(), 1);
    // 允许后端能力相关的启发式警告（如表格检测不可用），但不应有处理器执行的严重错误。
    let has_processor_failure = markdown
        .report()
        .warnings()
        .iter()
        .any(|w| matches!(w, easypdf::MarkdownWarning::ProcessorFailed { .. }));
    assert!(
        !has_processor_failure,
        "unexpected processor failures: {:?}",
        markdown.report().warnings()
    );
}

#[test]
fn security_operations_fail_on_missing_input() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.pdf");

    let encryption = EasyPdf::encrypt(&input, &output, "secret");
    let signature = EasyPdf::sign(
        &input,
        &output,
        directory.path().join("key.der").as_ref(),
        directory.path().join("cert.der").as_ref(),
        "approval",
    );

    // Both fail because the input file (and key/cert) don't exist.
    assert!(encryption.is_err());
    assert!(signature.is_err());
    assert!(!output.exists());
}
