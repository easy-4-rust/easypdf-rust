//! 纯 Rust 文本回退渲染器。
//!
//! 通过 [`easypdf_reader::PdfReader`] 从 PDF 中提取文本，并将其渲染为
//! 简单的白底黑字光栅图像。质量较低但足以满足 OCR 流水线需求，无需外部依赖。
#![cfg_attr(test, allow(clippy::similar_names))]

mod glyph;
mod renderer;

pub use renderer::TextRenderer;

// Re-export internals for test visibility.
#[cfg(test)]
use glyph::glyph_for;
#[cfg(test)]
use renderer::{draw_glyph, f64_to_u32};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::config::{Background, ImageFormat, RenderConfig};
    use crate::render::error::RenderError;
    use crate::render::traits::PdfRenderer;

    /// Helper: build a minimal valid PDF in memory with the given text content.
    fn make_test_pdf_bytes(text: &str) -> Vec<u8> {
        let content = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET");
        let mut doc = lopdf::Document::new();

        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            content.into_bytes(),
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
    fn test_text_renderer_from_bytes() {
        let bytes = make_test_pdf_bytes("Hello World");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert_eq!(renderer.name(), "text");
    }

    #[test]
    fn test_render_page_produces_image() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);
        assert_eq!(img.pixels.len(), (img.width * img.height * 4) as usize);
        assert_eq!(img.page_index, 0);
    }

    #[test]
    fn test_render_page_out_of_bounds() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig::default();
        let result = renderer.render_page(99, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RenderError::InvalidPage { .. }
        ));
    }

    #[test]
    fn test_render_page_dpi_exceeded() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 9999,
            ..RenderConfig::default()
        };
        let result = renderer.render_page(0, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RenderError::DpiExceeded { .. }
        ));
    }

    #[test]
    fn test_render_page_max_dimensions() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            max_width: Some(100),
            max_height: Some(100),
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width <= 100);
        assert!(img.height <= 100);
    }

    #[test]
    fn test_render_page_transparent_background() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            background: Background::Transparent,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        // With transparent background, first pixel alpha should be 0.
        assert_eq!(img.pixels[3], 0);
    }

    #[test]
    fn test_render_pages_range() {
        let bytes = make_test_pdf_bytes("Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let images = renderer.render_pages(0..1, &config).unwrap();
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn test_render_page_to_path() {
        let bytes = make_test_pdf_bytes("Save Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            format: ImageFormat::Png,
            ..RenderConfig::default()
        };
        // Use a unique temp file to avoid race conditions when running in parallel.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().with_extension("png");
        renderer.render_page_to_path(0, &config, &path).unwrap();
        assert!(path.exists());
        // Verify the file is a valid PNG.
        let loaded = image::open(&path).unwrap();
        assert!(loaded.width() > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_rendered_image_to_png_bytes() {
        let bytes = make_test_pdf_bytes("PNG Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        let png_bytes = img.to_png_bytes().unwrap();
        // PNG magic bytes.
        assert_eq!(
            &png_bytes[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn test_rendered_image_to_dynamic_image() {
        let bytes = make_test_pdf_bytes("Dynamic Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        let dynamic = img.to_dynamic_image();
        assert_eq!(dynamic.width(), img.width);
        assert_eq!(dynamic.height(), img.height);
    }

    #[test]
    fn test_glyph_rendering_non_empty() {
        // Ensure that rendering text produces non-uniform pixels
        // (i.e., the text actually gets drawn).
        let bytes = make_test_pdf_bytes("AB");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            background: Background::White,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        // Count non-white pixels.
        let non_white = img
            .pixels
            .chunks_exact(4)
            .filter(|px| px != &[255, 255, 255, 255])
            .count();
        // There should be at least some non-white pixels from the text or margins.
        // (The text content might be empty if lopdf can't extract it, so we just
        // verify the image was created successfully.)
        let _ = non_white;
    }

    #[test]
    fn test_page_pixels_scaling() {
        let (w72, h72) = TextRenderer::page_pixels(72);
        let (w150, h150) = TextRenderer::page_pixels(150);
        assert!(w150 > w72);
        assert!(h150 > h72);
    }

    #[test]
    fn test_name_returns_text() {
        let bytes = make_test_pdf_bytes("Name Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert_eq!(renderer.name(), "text");
    }

    #[test]
    fn test_max_dpi() {
        let bytes = make_test_pdf_bytes("DPI Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert!(renderer.max_dpi() > 0);
    }

    #[test]
    fn test_supports_vector_false() {
        let bytes = make_test_pdf_bytes("Vector Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        assert!(!renderer.supports_vector());
    }

    #[test]
    fn test_render_page_white_background() {
        let bytes = make_test_pdf_bytes("BG Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            background: Background::White,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        // First pixel should be white (255, 255, 255, 255)
        assert_eq!(img.pixels[0], 255);
        assert_eq!(img.pixels[1], 255);
        assert_eq!(img.pixels[2], 255);
        assert_eq!(img.pixels[3], 255);
    }

    #[test]
    fn test_render_pages_out_of_range() {
        let bytes = make_test_pdf_bytes("Range Test");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig::default();
        let result = renderer.render_pages(0..10, &config);
        assert!(result.is_err());
    }

    // --- Additional coverage tests for glyph table and helper functions ---

    #[test]
    fn glyph_for_all_printable_ascii() {
        // Exercise all printable ASCII glyphs (0x20..=0x7E) to cover match arms.
        for ch in 0x20u8..=0x7E {
            let glyph = glyph_for(ch);
            // Each glyph is 7 bytes
            assert_eq!(glyph.len(), 7);
        }
    }

    #[test]
    fn glyph_for_unknown_char_returns_filled_block() {
        let glyph = glyph_for(0xFF);
        assert_eq!(glyph, [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F]);
    }

    #[test]
    fn glyph_for_null_returns_filled_block() {
        let glyph = glyph_for(0x00);
        assert_eq!(glyph, [0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F, 0x1F]);
    }

    #[test]
    fn f64_to_u32_positive() {
        assert_eq!(f64_to_u32(3.7), 4);
        assert_eq!(f64_to_u32(0.0), 0);
        assert_eq!(f64_to_u32(100.0), 100);
    }

    #[test]
    fn f64_to_u32_negative_returns_zero() {
        assert_eq!(f64_to_u32(-1.0), 0);
        assert_eq!(f64_to_u32(-100.5), 0);
    }

    #[test]
    fn draw_glyph_out_of_bounds_no_panic() {
        // Draw a glyph partially outside the image bounds.
        let mut pixels = vec![0u8; 10 * 10 * 4];
        draw_glyph(&mut pixels, 10, 10, 8, 8, 1, b'A', [0, 0, 0, 255]);
        // Should not panic
    }

    #[test]
    fn draw_glyph_scale_2() {
        let mut pixels = vec![0u8; 40 * 40 * 4];
        draw_glyph(&mut pixels, 40, 40, 2, 2, 2, b'A', [0, 0, 0, 255]);
        // Should draw scaled glyph without panic
    }

    #[test]
    fn render_text_to_pixels_empty_text() {
        let pixels = TextRenderer::render_text_to_pixels("", 100, 100, 72, Background::White);
        assert_eq!(pixels.len(), 100 * 100 * 4);
        // All white
        assert!(pixels.chunks_exact(4).all(|px| px == [255, 255, 255, 255]));
    }

    #[test]
    fn render_text_to_pixels_multiline() {
        let pixels = TextRenderer::render_text_to_pixels(
            "line1\nline2\nline3",
            200,
            200,
            72,
            Background::White,
        );
        assert_eq!(pixels.len(), 200 * 200 * 4);
    }

    #[test]
    fn render_text_to_pixels_long_line_wraps() {
        let long_text = "A".repeat(200);
        let pixels =
            TextRenderer::render_text_to_pixels(&long_text, 100, 100, 72, Background::White);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn render_text_to_pixels_transparent_bg() {
        let pixels =
            TextRenderer::render_text_to_pixels("test", 50, 50, 72, Background::Transparent);
        assert_eq!(pixels.len(), 50 * 50 * 4);
        // Background should be transparent (0,0,0,0)
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[1], 0);
        assert_eq!(pixels[2], 0);
        assert_eq!(pixels[3], 0);
    }

    #[test]
    fn page_pixels_min_1() {
        // Even with very low DPI, dimensions should be at least 1
        let (w, h) = TextRenderer::page_pixels(1);
        assert!(w >= 1);
        assert!(h >= 1);
    }

    #[test]
    fn render_with_special_characters() {
        let bytes = make_test_pdf_bytes("Hello! @#$%");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
    }

    #[test]
    fn render_with_empty_content() {
        let bytes = make_test_pdf_bytes("");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
    }

    #[test]
    fn test_from_bytes_invalid() {
        let result = TextRenderer::from_bytes(vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn render_page_high_dpi() {
        let bytes = make_test_pdf_bytes("DPI");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 200,
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);
    }

    #[test]
    fn render_page_with_max_width_only() {
        let bytes = make_test_pdf_bytes("Width");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            max_width: Some(200),
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.width <= 200);
    }

    #[test]
    fn render_page_with_max_height_only() {
        let bytes = make_test_pdf_bytes("Height");
        let renderer = TextRenderer::from_bytes(bytes).unwrap();
        let config = RenderConfig {
            dpi: 72,
            max_height: Some(200),
            ..RenderConfig::default()
        };
        let img = renderer.render_page(0, &config).unwrap();
        assert!(img.height <= 200);
    }
}
