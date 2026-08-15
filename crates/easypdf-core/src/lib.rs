#![doc = "Core types, traits, enums, converters, and errors for `easypdf-rust`."]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::similar_names))]

// --- Modules ---
pub mod content;
pub mod converter_registry;
pub mod enums;
pub mod error;
pub mod event;
pub mod handler_chain;
pub mod logging;
pub mod metadata;
pub mod page_index;
pub mod page_number;
pub mod page_range;
pub mod style;
pub mod traits;

// --- Crypto (encryption + signing) ---
pub mod crypto;

// --- Merged sub-crates (Wave 1) ---
pub mod io;
pub mod layout;
pub mod model;

// --- Convenience re-exports ---
pub use content::{PdfImage, PdfLine, PdfRect, PdfTable, PdfTableCell, PdfText};
pub use enums::{Orientation, PageSize, Rotation, TextAlignment, VerticalAlignment};
pub use error::{PdfError, PdfErrorCode, Result};
pub use event::PdfReadListener;
pub use logging::{init_logging, init_logging_json};
pub use metadata::{PdfBookmark, PdfMetadata};
pub use page_index::PageIndex;
pub use page_number::PageNumber;
pub use page_range::PageRange;
pub use style::{BuiltInFont, FontFamily, FontStyle, PdfColor, PdfFont, TableBorder, TableStyle};
pub use traits::{
    CapabilityLevel, DetailedEngineCapabilities, EngineCapabilities, PdfConverter, PdfEngine,
    PdfFieldDescriptor, PdfModel, PdfModelMetadata, PdfWriteHandler, RenderedElement,
};

// --- Re-exports from merged model crate ---
pub use model::{
    ImageData, ImageFormat, ListItem, PdfBlock, PdfBlockType, PdfDocumentModel, PdfPageModel,
    SourceLocation,
};

// --- Re-exports from merged io crate (flat types) ---
pub use io::{AtomicFileOutput, PdfInput, ResourceLimits};

// --- Re-exports from merged layout crate ---
pub use layout::{Direction, FlowLayout, LayoutSink};

#[cfg(test)]
mod tests {
    use super::*;

    /// Approximate equality for f64 in tests.
    fn assert_f64_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    // --- Enums ---

    #[test]
    fn test_page_size_dimensions() {
        assert_eq!(PageSize::A4.dimensions(), (595.0, 842.0));
        assert_eq!(PageSize::A0.dimensions(), (2384.0, 3370.0));
        assert_eq!(PageSize::A1.dimensions(), (1684.0, 2384.0));
        assert_eq!(PageSize::A2.dimensions(), (1191.0, 1684.0));
        assert_eq!(PageSize::A3.dimensions(), (842.0, 1191.0));
        assert_eq!(PageSize::Letter.dimensions(), (612.0, 792.0));
        assert_eq!(PageSize::Legal.dimensions(), (612.0, 1008.0));
        assert_eq!(PageSize::A5.dimensions(), (420.0, 595.0));
        assert_eq!(PageSize::Custom(100.0, 200.0).dimensions(), (100.0, 200.0));
    }

    #[test]
    fn test_orientation_default() {
        assert_eq!(Orientation::default(), Orientation::Portrait);
    }

    #[test]
    fn test_text_alignment_default() {
        assert_eq!(TextAlignment::default(), TextAlignment::Left);
    }

    #[test]
    fn test_vertical_alignment_default() {
        assert_eq!(VerticalAlignment::default(), VerticalAlignment::Top);
    }

    // --- Error ---

    #[test]
    fn test_pdf_error_display() {
        let err = PdfError::InvalidPage(5);
        assert!(format!("{err}").contains('5'));
        let err = PdfError::Parse("bad data".into());
        assert!(format!("{err}").contains("bad data"));
        let err = PdfError::UnsupportedFeature("tables".into());
        assert!(format!("{err}").contains("tables"));
    }

    #[test]
    fn test_pdf_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let pdf_err: PdfError = io_err.into();
        assert!(matches!(pdf_err, PdfError::Io(_)));
    }

    // --- Style ---

    #[test]
    fn test_pdf_color_rgb_u8() {
        let color = PdfColor::rgb_u8(255, 0, 128);
        match color {
            PdfColor::Rgb(r, g, b) => {
                assert!((r - 1.0).abs() < 0.001);
                assert!((g - 0.0).abs() < 0.001);
                assert!((b - 0.5).abs() < 0.01);
            }
            _ => panic!("expected Rgb"),
        }
    }

    #[test]
    fn test_pdf_color_constants() {
        assert_eq!(PdfColor::black(), PdfColor::Rgb(0.0, 0.0, 0.0));
        assert_eq!(PdfColor::white(), PdfColor::Rgb(1.0, 1.0, 1.0));
        assert_eq!(PdfColor::red(), PdfColor::Rgb(1.0, 0.0, 0.0));
        assert_eq!(PdfColor::green(), PdfColor::Rgb(0.0, 1.0, 0.0));
        assert_eq!(PdfColor::blue(), PdfColor::Rgb(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_pdf_color_cmyk() {
        let c = PdfColor::Cmyk(0.1, 0.2, 0.3, 0.4);
        if let PdfColor::Cmyk(c1, m, y, k) = c {
            assert!((c1 - 0.1).abs() < 0.001);
            assert!((m - 0.2).abs() < 0.001);
            assert!((y - 0.3).abs() < 0.001);
            assert!((k - 0.4).abs() < 0.001);
        } else {
            panic!("expected Cmyk");
        }
    }

    #[test]
    fn test_pdf_color_gray() {
        assert_eq!(PdfColor::Gray(0.5), PdfColor::Gray(0.5));
        assert_eq!(PdfColor::light_gray(), PdfColor::Gray(0.8));
        assert_eq!(PdfColor::gray(), PdfColor::Gray(0.5));
    }

    #[test]
    fn test_pdf_font_constructors() {
        let f = PdfFont::helvetica(12.0);
        assert_f64_eq(f.size, 12.0);
        assert!(matches!(
            f.family,
            FontFamily::BuiltIn(BuiltInFont::Helvetica)
        ));

        let f = PdfFont::times_roman(14.0);
        assert_f64_eq(f.size, 14.0);

        let f = PdfFont::courier(10.0);
        assert_f64_eq(f.size, 10.0);
    }

    #[test]
    fn test_pdf_font_builder() {
        let f = PdfFont::helvetica(12.0).bold().italic().with_size(16.0);
        assert_f64_eq(f.size, 16.0);
        assert!(f.style.bold);
        assert!(f.style.italic);
    }

    #[test]
    fn test_pdf_font_default() {
        let f = PdfFont::default();
        assert_f64_eq(f.size, 12.0);
        assert!(!f.style.bold);
        assert!(!f.style.italic);
    }

    #[test]
    fn test_table_style_defaults() {
        let s = TableStyle::default();
        assert!(s.header_bg.is_some());
        assert!(s.header_font.style.bold);
        assert!(!s.striped);
    }

    #[test]
    fn test_table_style_simple() {
        let s = TableStyle::simple();
        assert!(s.header_bg.is_none());
        assert!(!s.striped);
    }

    #[test]
    fn test_table_style_striped() {
        let s = TableStyle::striped();
        assert!(s.header_bg.is_some());
        assert!(s.striped);
    }

    #[test]
    fn test_table_border_default() {
        let b = TableBorder::default();
        assert_f64_eq(b.width, 0.5);
        assert_eq!(b.color, PdfColor::black());
    }

    // --- Content ---

    #[test]
    fn test_pdf_text_new() {
        let t = PdfText::new("hello");
        assert_eq!(t.content, "hello");
        assert_eq!(t.alignment, TextAlignment::Left);
    }

    #[test]
    fn test_pdf_text_builder() {
        let t = PdfText::new("hi")
            .font(PdfFont::courier(10.0))
            .alignment(TextAlignment::Center)
            .color(PdfColor::red());
        assert_eq!(t.content, "hi");
        assert_eq!(t.alignment, TextAlignment::Center);
        assert_eq!(t.color, PdfColor::red());
    }

    #[test]
    fn test_pdf_table_builder() {
        let t = PdfTable::new(vec!["A".into(), "B".into()])
            .row(vec!["1".into(), "2".into()])
            .rows(vec![vec!["3".into(), "4".into()]])
            .width(500.0);
        assert_eq!(t.headers.len(), 2);
        assert_eq!(t.rows.len(), 2);
        assert_f64_eq(t.width, 500.0);
    }

    #[test]
    fn test_pdf_table_cell_default() {
        let c = PdfTableCell::default();
        assert!(c.content.is_empty());
        assert_eq!(c.h_alignment, TextAlignment::Left);
        assert_eq!(c.v_alignment, VerticalAlignment::Top);
    }

    #[test]
    fn test_pdf_image_from_bytes() {
        let img = PdfImage::from_bytes(vec![1, 2, 3]);
        assert_eq!(img.data, vec![1, 2, 3]);
        assert_f64_eq(img.width, 0.0);
        assert_f64_eq(img.height, 0.0);
    }

    #[test]
    fn test_pdf_image_from_path_nonexistent() {
        let result = PdfImage::from_path("/nonexistent/image.png");
        assert!(result.is_err());
    }

    #[test]
    fn test_pdf_image_from_path_temp() {
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_real.png");
        // Create a minimal PNG
        let png = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&path, &png).unwrap();
        let result = PdfImage::from_path(&path);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&path);
    }

    // --- Metadata ---

    #[test]
    fn test_pdf_metadata_builder() {
        let m = PdfMetadata::new()
            .title("T")
            .author("A")
            .subject("S")
            .keywords("K");
        assert_eq!(m.title.as_deref(), Some("T"));
        assert_eq!(m.author.as_deref(), Some("A"));
        assert_eq!(m.subject.as_deref(), Some("S"));
        assert_eq!(m.keywords.as_deref(), Some("K"));
    }

    #[test]
    fn test_pdf_bookmark_new() {
        let b = PdfBookmark::new("Chapter 1", 1);
        assert_eq!(b.title, "Chapter 1");
        assert_eq!(b.page, 1);
        assert!(b.children.is_empty());
    }

    #[test]
    fn test_pdf_bookmark_child() {
        let b = PdfBookmark::new("Ch1", 1).child(PdfBookmark::new("1.1", 2));
        assert_eq!(b.children.len(), 1);
        assert_eq!(b.children[0].title, "1.1");
    }

    // --- Traits ---

    #[test]
    fn test_pdf_model_metadata_default() {
        let m = PdfModelMetadata::default();
        assert_eq!(m.page_size, PageSize::A4);
        assert_eq!(m.orientation, Orientation::Portrait);
        assert_f64_eq(m.margins, 72.0);
    }

    struct TestListener {
        pages: Vec<String>,
    }

    impl PdfReadListener for TestListener {
        fn on_text(&mut self, _page: usize, text: &str) -> Result<()> {
            self.pages.push(text.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_pdf_read_listener_defaults() {
        let mut listener = TestListener { pages: vec![] };
        // Default no-ops should not panic
        assert!(listener.on_page_start(1).is_ok());
        assert!(listener.on_page_end(1).is_ok());
        assert!(listener.on_document_end().is_ok());
    }

    struct TestHandler;

    impl PdfWriteHandler for TestHandler {}

    #[test]
    fn test_pdf_write_handler_defaults() {
        let mut handler = TestHandler;
        assert!(handler.before_document().is_ok());
        assert!(handler.before_page(1).is_ok());
        assert!(handler.after_page(1).is_ok());
        assert!(handler.after_document().is_ok());
    }

    struct TestConverter;

    impl PdfConverter<i32> for TestConverter {
        fn to_pdf_string(&self, value: &i32) -> Result<String> {
            Ok(value.to_string())
        }
        fn from_pdf_string(&self, s: &str) -> Result<i32> {
            s.parse::<i32>().map_err(|e| PdfError::Other(e.to_string()))
        }
    }

    #[test]
    fn test_pdf_converter_roundtrip() {
        let c = TestConverter;
        assert_eq!(c.to_pdf_string(&42).unwrap(), "42");
        assert_eq!(c.from_pdf_string("99").unwrap(), 99);
        assert!(c.from_pdf_string("abc").is_err());
    }

    #[test]
    fn test_rendered_element_debug() {
        let e = RenderedElement::Text {
            x: 1.0,
            y: 2.0,
            text: PdfText::new("hi"),
        };
        assert!(format!("{e:?}").contains("Text"));
    }

    #[test]
    fn test_xmp_metadata() {
        let xmp = PdfMetadata::new()
            .title("Test Title")
            .author("Author Name")
            .to_xmp();
        assert!(xmp.contains("Test Title"));
        assert!(xmp.contains("Author Name"));
        assert!(xmp.contains("xmpmeta"));
    }

    #[test]
    fn test_engine_capabilities_lopdf() {
        let c = EngineCapabilities::lopdf();
        assert!(c.read);
        assert!(c.manipulate);
        assert!(c.fill_forms);
        assert!(!c.create);
    }

    #[test]
    fn test_engine_capabilities_printpdf() {
        let c = EngineCapabilities::printpdf();
        assert!(c.create);
        assert!(!c.read);
    }
}
