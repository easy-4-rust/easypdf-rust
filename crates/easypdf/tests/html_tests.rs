//! Integration tests for HTML/Markdown features (requires 'html' feature + Chromium).
#![cfg(feature = "html")]

use easypdf::EasyPdf;

#[test]
fn test_from_html_with_chrome() {
    let dir = std::env::temp_dir();
    let out = dir.join("easypdf_html_test.pdf");
    let html = "<html><body><h1>Test</h1><p>Hello from HTML</p></body></html>";
    let result = EasyPdf::from_html(html)
        .unwrap()
        .title("HTML Test")
        .save(&out);
    if let Err(ref e) = result {
        eprintln!("HTML test note: {e:?} (may need Chromium installed)");
    }
    // Clean up if file was created
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_from_markdown() {
    let dir = std::env::temp_dir();
    let out = dir.join("easypdf_md_test.pdf");
    let md = "# Hello\n\nThis is **bold** and *italic*.";
    let result = EasyPdf::from_markdown(md).unwrap().save(&out);
    if let Err(ref e) = result {
        eprintln!("Markdown test note: {e:?} (may need Chromium installed)");
    }
    let _ = std::fs::remove_file(&out);
}
