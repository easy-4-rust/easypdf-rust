//! PDF 到 Markdown 的端到端集成测试。

use std::sync::Arc;

use easypdf_markdown::{
    MarkdownProcessorCapabilities, MarkdownProfile, PdfMarkdownBuilder, PdfMarkdownExportBuilder,
    PdfMarkdownProcessor,
};

fn make_two_page_pdf(path: &std::path::Path) {
    let mut document = lopdf::Document::new();
    let mut font = lopdf::Dictionary::new();
    font.set("Type", lopdf::Object::Name(b"Font".to_vec()));
    font.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
    font.set("BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
    let font_id = document.add_object(lopdf::Object::Dictionary(font));

    let mut fonts = lopdf::Dictionary::new();
    fonts.set("F1", lopdf::Object::Reference(font_id));
    let mut resources = lopdf::Dictionary::new();
    resources.set("Font", lopdf::Object::Dictionary(fonts));
    let resources_id = document.add_object(lopdf::Object::Dictionary(resources));

    let mut page_ids = Vec::new();
    for text in ["First Page", "Second Page"] {
        let content_id = document.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET").into_bytes(),
        )));
        let mut page = lopdf::Dictionary::new();
        page.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page.set("Contents", lopdf::Object::Reference(content_id));
        page.set("Resources", lopdf::Object::Reference(resources_id));
        page_ids.push(document.add_object(lopdf::Object::Dictionary(page)));
    }

    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
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
    pages.set("Count", lopdf::Object::Integer(2));
    let pages_id = document.add_object(lopdf::Object::Dictionary(pages));

    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", lopdf::Object::Reference(pages_id));
    let catalog_id = document.add_object(lopdf::Object::Dictionary(catalog));
    document
        .trailer
        .set("Root", lopdf::Object::Reference(catalog_id));
    document.save(path).expect("save PDF fixture");
}

#[test]
fn exports_selected_zero_based_page_atomically() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.md");
    make_two_page_pdf(&input);

    let result = PdfMarkdownExportBuilder::new(&input, &output)
        .pages(0..1)
        .profile(MarkdownProfile::Llm)
        .do_export()
        .expect("export Markdown");
    let markdown = std::fs::read_to_string(&output).expect("read Markdown");

    assert!(markdown.contains("## Page 1"));
    assert!(markdown.contains("First Page"));
    assert!(!markdown.contains("Second Page"));
    assert_eq!(result.report().pages_read(), 1);
    assert_eq!(result.output(), output);
}

#[test]
fn rejects_input_above_resource_limit_without_replacing_output() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.md");
    make_two_page_pdf(&input);
    std::fs::write(&output, "existing").expect("seed output");

    let limits = easypdf_core::ResourceLimits::new().with_max_input_bytes(8);
    let error = PdfMarkdownExportBuilder::new(&input, &output)
        .resource_limits(limits)
        .do_export()
        .expect_err("oversized input must fail");

    assert_eq!(
        error.code(),
        easypdf_core::PdfErrorCode::ResourceLimitExceeded
    );
    assert_eq!(
        std::fs::read_to_string(output).expect("read original output"),
        "existing"
    );
}

#[test]
fn converts_from_bytes_without_creating_an_output_file() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("input.pdf");
    make_two_page_pdf(&input);
    let bytes = std::fs::read(input).expect("read PDF fixture");

    let result = PdfMarkdownBuilder::from_bytes(bytes)
        .pages(1..2)
        .profile(MarkdownProfile::Llm)
        .do_convert()
        .expect("convert Markdown");

    assert!(result.markdown().contains("## Page 2"));
    assert!(result.markdown().contains("Second Page"));
    assert!(!result.markdown().contains("First Page"));
    assert_eq!(result.report().pages_read(), 1);
}

struct TableAwareProcessor;

impl PdfMarkdownProcessor for TableAwareProcessor {
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        MarkdownProcessorCapabilities::new().with_table_detection()
    }

    fn process(
        &self,
        _input: &easypdf_core::PdfInput,
        document: easypdf_core::PdfDocumentModel,
    ) -> easypdf_core::Result<(
        easypdf_core::PdfDocumentModel,
        Vec<easypdf_markdown::MarkdownWarning>,
    )> {
        Ok((document, Vec::new()))
    }
}

#[test]
fn registered_processor_satisfies_only_its_declared_capability() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let input = directory.path().join("input.pdf");
    make_two_page_pdf(&input);

    let result = PdfMarkdownBuilder::new(&input)
        .processor(Arc::new(TableAwareProcessor))
        .do_convert()
        .expect("convert Markdown");

    assert!(!result.report().warnings().iter().any(|warning| matches!(
        warning,
        easypdf_markdown::MarkdownWarning::TableDetectionUnavailable
    )));
}
