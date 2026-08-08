//! 比较单次解析会话与重复打开 PDF 的基准程序。

use std::hint::black_box;
use std::time::{Duration, Instant};

use easypdf_reader::PdfReader;

const ITERATIONS: usize = 200;

fn main() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("benchmark.pdf");
    make_pdf(&path);

    let session = PdfReader::open(&path).expect("open benchmark PDF");
    let reused = measure(|| {
        black_box(session.page_count().expect("page count"));
        black_box(session.extract_metadata().expect("metadata"));
    });
    let reopened = measure(|| {
        let reader = PdfReader::open(&path).expect("reopen benchmark PDF");
        black_box(reader.page_count().expect("page count"));
        black_box(reader.extract_metadata().expect("metadata"));
    });

    println!("reader_session iterations={ITERATIONS}");
    println!("reused_session_ns_per_iter={}", nanos_per_iteration(reused));
    println!("reopen_ns_per_iter={}", nanos_per_iteration(reopened));
    println!(
        "speedup={:.2}x",
        reopened.as_secs_f64() / reused.as_secs_f64()
    );
}

fn measure(mut operation: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        operation();
    }
    start.elapsed()
}

fn nanos_per_iteration(duration: Duration) -> u128 {
    duration.as_nanos() / ITERATIONS as u128
}

fn make_pdf(path: &std::path::Path) {
    let mut document = lopdf::Document::new();
    let content_id = document.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 12 Tf 72 700 Td (Benchmark) Tj ET".to_vec(),
    )));
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

    let mut page = lopdf::Dictionary::new();
    page.set("Type", lopdf::Object::Name(b"Page".to_vec()));
    page.set(
        "MediaBox",
        lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
    );
    page.set("Contents", lopdf::Object::Reference(content_id));
    page.set("Resources", lopdf::Object::Reference(resources_id));
    let page_id = document.add_object(lopdf::Object::Dictionary(page));

    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
    pages.set(
        "Kids",
        lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
    );
    pages.set("Count", lopdf::Object::Integer(1));
    let pages_id = document.add_object(lopdf::Object::Dictionary(pages));
    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", lopdf::Object::Reference(pages_id));
    let catalog_id = document.add_object(lopdf::Object::Dictionary(catalog));
    document
        .trailer
        .set("Root", lopdf::Object::Reference(catalog_id));
    document.save(path).expect("save benchmark PDF");
}
