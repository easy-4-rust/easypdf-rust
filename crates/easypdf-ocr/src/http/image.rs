//! Image encoding utilities for OCR HTTP requests.
//!
//! Different OCR APIs expect images in different formats:
//! - **Base64 inline**: image bytes encoded as a base64 string in the JSON body
//! - **Multipart**: image bytes uploaded as a form-data file field
//! - **Remote URL**: a URL pointing to an already-uploaded image

use base64::Engine;
use easypdf_markdown::ocr::OcrImage;

use super::error::{OcrHttpError, Result};

/// Strategy for encoding an image in an HTTP request.
#[derive(Debug, Clone)]
pub enum ImageEncoding {
    /// Encode image as a base64 string inline in the JSON request body.
    ///
    /// Used by GLM-OCR, DeepSeek-OCR, and other APIs that accept
    /// `image_url: { url: "data:image/png;base64,..." }`.
    Base64Inline,

    /// Upload image as multipart/form-data.
    ///
    /// Used by Baidu OCR APIs (Qianfan, PP-OCRv6) that accept file uploads.
    Multipart {
        /// The form field name for the image file (e.g., `"image"`).
        field_name: String,
    },

    /// Reference an image by URL (user must upload to their own storage first).
    RemoteUrl,
}

/// Encoded image ready for inclusion in an HTTP request.
#[derive(Debug, Clone)]
pub struct EncodedImage {
    /// Image format hint (`"png"` or `"jpeg"`).
    pub format: String,
    /// Base64-encoded image data (populated for `Base64Inline`).
    pub base64: Option<String>,
    /// URL reference (populated for `RemoteUrl`).
    pub url: Option<String>,
    /// Raw image bytes (populated for `Multipart`).
    pub bytes: Option<Vec<u8>>,
}

/// Encode an `OcrImage` according to the given encoding strategy.
///
/// # Errors
///
/// Returns `OcrHttpError::InvalidResponse` if the pixel data cannot be
/// encoded to PNG.
pub fn encode_for_request(image: &OcrImage, encoding: &ImageEncoding) -> Result<EncodedImage> {
    match encoding {
        ImageEncoding::Base64Inline => {
            let png_bytes = encode_to_png(image)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            Ok(EncodedImage {
                format: "png".to_owned(),
                base64: Some(b64),
                url: None,
                bytes: None,
            })
        }
        ImageEncoding::Multipart { .. } => {
            let png_bytes = encode_to_png(image)?;
            Ok(EncodedImage {
                format: "png".to_owned(),
                base64: None,
                url: None,
                bytes: Some(png_bytes),
            })
        }
        ImageEncoding::RemoteUrl => Ok(EncodedImage {
            format: "png".to_owned(),
            base64: None,
            url: None,
            bytes: None,
        }),
    }
}

/// Encode the RGBA pixel data as PNG bytes.
///
/// # Errors
///
/// Returns `OcrHttpError::InvalidResponse` if the pixel data cannot be encoded.
pub fn encode_to_png(image: &OcrImage) -> Result<Vec<u8>> {
    let rgba_img = image::RgbaImage::from_raw(image.width, image.height, image.pixels.clone())
        .ok_or_else(|| {
            OcrHttpError::InvalidResponse(format!(
                "pixel buffer length {} does not match {}x{}x4",
                image.pixels.len(),
                image.width,
                image.height,
            ))
        })?;

    let dynamic = image::DynamicImage::ImageRgba8(rgba_img);
    let mut buf = Vec::new();
    dynamic
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| OcrHttpError::InvalidResponse(format!("PNG encoding failed: {e}")))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(width: u32, height: u32) -> OcrImage {
        let pixels = vec![255u8; (width * height * 4) as usize];
        OcrImage::new(width, height, pixels)
    }

    #[test]
    fn test_encode_base64_inline() {
        let image = make_test_image(2, 2);
        let encoded = encode_for_request(&image, &ImageEncoding::Base64Inline).unwrap();
        assert_eq!(encoded.format, "png");
        assert!(encoded.base64.is_some());
        assert!(encoded.url.is_none());
        assert!(encoded.bytes.is_none());

        // Verify the base64 decodes to valid PNG bytes.
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.base64.unwrap())
            .unwrap();
        assert!(decoded.starts_with(b"\x89PNG"));
    }

    #[test]
    fn test_encode_multipart() {
        let image = make_test_image(2, 2);
        let encoding = ImageEncoding::Multipart {
            field_name: "image".to_owned(),
        };
        let encoded = encode_for_request(&image, &encoding).unwrap();
        assert_eq!(encoded.format, "png");
        assert!(encoded.base64.is_none());
        assert!(encoded.url.is_none());
        assert!(encoded.bytes.is_some());

        let bytes = encoded.bytes.unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn test_encode_remote_url() {
        let image = make_test_image(2, 2);
        let encoded = encode_for_request(&image, &ImageEncoding::RemoteUrl).unwrap();
        assert_eq!(encoded.format, "png");
        assert!(encoded.base64.is_none());
        assert!(encoded.url.is_none());
        assert!(encoded.bytes.is_none());
    }

    #[test]
    fn test_encode_empty_image() {
        // 0x0 image: from_raw returns None for empty buffer.
        let image = OcrImage::new(0, 0, vec![]);
        let result = encode_for_request(&image, &ImageEncoding::Base64Inline);
        assert!(result.is_err());
    }

    #[test]
    fn test_encoded_image_debug() {
        let image = make_test_image(1, 1);
        let encoded = encode_for_request(&image, &ImageEncoding::Base64Inline).unwrap();
        let debug = format!("{encoded:?}");
        assert!(debug.contains("png"));
    }

    #[test]
    fn test_image_encoding_debug() {
        let enc = ImageEncoding::Base64Inline;
        assert!(format!("{enc:?}").contains("Base64Inline"));

        let enc = ImageEncoding::Multipart {
            field_name: "file".to_owned(),
        };
        assert!(format!("{enc:?}").contains("file"));

        let enc = ImageEncoding::RemoteUrl;
        assert!(format!("{enc:?}").contains("RemoteUrl"));
    }
}
