//! OCR HTTP 请求的图像编码工具。
//!
//! 不同的 OCR API 期望不同格式的图像：
//! - **Base64 内联**：图像字节编码为 base64 字符串放在 JSON 请求体中
//! - **Multipart**：图像字节作为表单文件字段上传
//! - **远程 URL**：指向已上传图像的 URL

use base64::Engine;
use easypdf_markdown::ocr::OcrImage;

use super::error::{OcrHttpError, Result};

/// HTTP 请求中图像的编码策略。
#[derive(Debug, Clone)]
pub enum ImageEncoding {
    /// 将图像编码为 base64 字符串内联在 JSON 请求体中。
    ///
    /// GLM-OCR、DeepSeek-OCR 及其他接受
    /// `image_url: { url: "data:image/png;base64,..." }` 的 API 使用此方式。
    Base64Inline,

    /// 以 multipart/form-data 方式上传图像。
    ///
    /// 接受文件上传的百度 OCR API（千帆、PP-OCRv6）使用此方式。
    Multipart {
        /// 图像文件的表单字段名（如 `"image"`）。
        field_name: String,
    },

    /// 通过 URL 引用图像（用户需先上传至自己的存储）。
    RemoteUrl,
}

/// 已编码的图像，可直接包含在 HTTP 请求中。
#[derive(Debug, Clone)]
pub struct EncodedImage {
    /// 图像格式提示（`"png"` 或 `"jpeg"`）。
    pub format: String,
    /// Base64 编码的图像数据（`Base64Inline` 时填充）。
    pub base64: Option<String>,
    /// URL 引用（`RemoteUrl` 时填充）。
    pub url: Option<String>,
    /// 原始图像字节（`Multipart` 时填充）。
    pub bytes: Option<Vec<u8>>,
}

/// 按照给定编码策略编码 `OcrImage`。
///
/// # Errors
///
/// 若像素数据无法编码为 PNG，返回 `OcrHttpError::InvalidResponse`。
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

/// 将 RGBA 像素数据编码为 PNG 字节。
///
/// # Errors
///
/// 若像素数据无法编码，返回 `OcrHttpError::InvalidResponse`。
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
