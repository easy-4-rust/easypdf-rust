//! Response parser for Tencent Cloud OCR API.
//!
//! Implements [`OcrResponseParser`] for the two response formats:
//!
//! - **General OCR** (`GeneralBasicOCR`, `GeneralAccurateOCR`):
//!   `Response.TextDetections[].DetectedText`
//! - **Document extraction** (`SmartStructuralOCR`):
//!   `Response.WordList[].Text`

use easypdf_markdown::ocr::{OcrResult, WordBox};
use crate::http::error::{OcrHttpError, Result};
use crate::http::response::OcrResponseParser;

use super::config::HunyuanMode;

/// Response parser for Tencent Cloud OCR API.
///
/// Parses the JSON response from Tencent Cloud OCR into a standard
/// [`OcrResult`]. Supports both general OCR and document extraction
/// response formats.
///
/// # Response Formats
///
/// ## General OCR (`GeneralBasicOCR`, `GeneralAccurateOCR`)
///
/// ```json
/// {
///   "Response": {
///     "TextDetections": [
///       { "DetectedText": "Hello", "Confidence": 95 }
///     ],
///     "RequestId": "..."
///   }
/// }
/// ```
///
/// ## Document Extraction (`SmartStructuralOCR`)
///
/// ```json
/// {
///   "Response": {
///     "WordList": [
///       { "Text": "Hello" }
///     ],
///     "StructuralList": [...],
///     "RequestId": "..."
///   }
/// }
/// ```
pub struct HunyuanOcrParser {
    mode: HunyuanMode,
}

impl HunyuanOcrParser {
    /// Create a new parser for the given mode.
    #[must_use]
    pub fn new(mode: HunyuanMode) -> Self {
        Self { mode }
    }
}

impl OcrResponseParser for HunyuanOcrParser {
    fn parse_response(&self, raw: &serde_json::Value) -> Result<OcrResult> {
        // Tencent Cloud wraps all responses in a "Response" object.
        let response = raw
            .get("Response")
            .ok_or_else(|| {
                OcrHttpError::InvalidResponse(
                    "missing top-level 'Response' field in Tencent Cloud OCR response".to_owned(),
                )
            })?;

        // Check for error response.
        if let Some(error) = response.get("Error") {
            let code = error
                .get("Code")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let message = error
                .get("Message")
                .and_then(|v| v.as_str())
                .unwrap_or("no message");
            return Err(OcrHttpError::Engine(format!(
                "Tencent Cloud OCR error [{code}]: {message}"
            )));
        }

        // Parse based on mode.
        if self.mode.uses_text_detections() {
            parse_text_detections(response)
        } else {
            parse_word_list(response)
        }
    }
}

/// Parse the `TextDetections` response format (`GeneralBasic`, `GeneralAccurate`).
///
/// Concatenates all `DetectedText` values with newlines and computes
/// an average confidence score.
#[allow(clippy::cast_possible_truncation)]
fn parse_text_detections(response: &serde_json::Value) -> Result<OcrResult> {
    let detections = response
        .get("TextDetections")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            OcrHttpError::InvalidResponse(
                "missing or invalid 'TextDetections' array in response".to_owned(),
            )
        })?;

    let mut texts = Vec::with_capacity(detections.len());
    let mut total_confidence: f64 = 0.0;
    let mut confidence_count: u32 = 0;
    let mut word_boxes = Vec::new();

    for detection in detections {
        let text = detection
            .get("DetectedText")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        texts.push(text.to_owned());

        // Collect confidence scores.
        if let Some(conf) = detection.get("Confidence").and_then(serde_json::Value::as_f64) {
            total_confidence += conf;
            confidence_count += 1;
        }

        // Collect word boxes from Polygon coordinates (if available).
        if let Some(polygon) = detection.get("Polygon").and_then(|v| v.as_array())
            && polygon.len() >= 4
            && let (Some(x), Some(y)) = (
                polygon[0].get("X").and_then(serde_json::Value::as_u64),
                polygon[0].get("Y").and_then(serde_json::Value::as_u64),
            )
        {
            let x2 = polygon[2]
                .get("X")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(x);
            let y2 = polygon[2]
                .get("Y")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(y);
            let word_conf = detection
                .get("Confidence")
                .and_then(serde_json::Value::as_f64)
                .map(|c| c as f32 / 100.0);
            word_boxes.push(WordBox {
                text: text.to_owned(),
                x: u32::try_from(x).unwrap_or(0),
                y: u32::try_from(y).unwrap_or(0),
                width: u32::try_from(x2.saturating_sub(x)).unwrap_or(0),
                height: u32::try_from(y2.saturating_sub(y)).unwrap_or(0),
                confidence: word_conf,
            });
        }
    }

    let confidence = if confidence_count > 0 {
        Some((total_confidence / f64::from(confidence_count)) as f32 / 100.0)
    } else {
        None
    };

    Ok(OcrResult {
        text: texts.join("\n"),
        confidence,
        word_boxes,
    })
}

/// Parse the `WordList` response format (`SmartStructuralOCR`).
///
/// Concatenates all `Text` values from `WordList` entries.
fn parse_word_list(response: &serde_json::Value) -> Result<OcrResult> {
    let word_list = response
        .get("WordList")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            OcrHttpError::InvalidResponse(
                "missing or invalid 'WordList' array in response".to_owned(),
            )
        })?;

    let mut texts = Vec::with_capacity(word_list.len());

    for item in word_list {
        let text = item
            .get("Text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        texts.push(text.to_owned());
    }

    Ok(OcrResult {
        text: texts.join("\n"),
        confidence: None,
        word_boxes: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_detections_success() {
        let response = serde_json::json!({
            "Response": {
                "TextDetections": [
                    { "DetectedText": "Hello", "Confidence": 95.0 },
                    { "DetectedText": "World", "Confidence": 90.0 }
                ],
                "RequestId": "req-123"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralBasic);
        let result = parser.parse_response(&response).unwrap();
        assert_eq!(result.text, "Hello\nWorld");
        assert!(result.confidence.is_some());
        let conf = result.confidence.unwrap();
        assert!((conf - 0.925).abs() < 0.01); // (95+90)/2/100
    }

    #[test]
    fn test_parse_text_detections_empty_array() {
        let response = serde_json::json!({
            "Response": {
                "TextDetections": [],
                "RequestId": "req-456"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralBasic);
        let result = parser.parse_response(&response).unwrap();
        assert!(result.text.is_empty());
        assert!(result.confidence.is_none());
    }

    #[test]
    fn test_parse_word_list_success() {
        let response = serde_json::json!({
            "Response": {
                "WordList": [
                    { "Text": "Document" },
                    { "Text": "Title" }
                ],
                "RequestId": "req-789"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::SmartStructural);
        let result = parser.parse_response(&response).unwrap();
        assert_eq!(result.text, "Document\nTitle");
    }

    #[test]
    fn test_parse_word_list_empty() {
        let response = serde_json::json!({
            "Response": {
                "WordList": [],
                "RequestId": "req-000"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::SmartStructural);
        let result = parser.parse_response(&response).unwrap();
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_parse_error_response() {
        let response = serde_json::json!({
            "Response": {
                "Error": {
                    "Code": "FailedOperation.UnOpenError",
                    "Message": "OCR service not activated"
                },
                "RequestId": "req-err"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralBasic);
        let err = parser.parse_response(&response).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("FailedOperation.UnOpenError"), "msg: {msg}");
        assert!(msg.contains("OCR service not activated"), "msg: {msg}");
    }

    #[test]
    fn test_parse_missing_response_field() {
        let response = serde_json::json!({
            "SomeOtherField": {}
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralBasic);
        let err = parser.parse_response(&response).unwrap_err();
        assert!(err.to_string().contains("missing top-level 'Response'"));
    }

    #[test]
    fn test_parse_missing_text_detections() {
        let response = serde_json::json!({
            "Response": {
                "RequestId": "req-no-data"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralBasic);
        let err = parser.parse_response(&response).unwrap_err();
        assert!(err.to_string().contains("TextDetections"));
    }

    #[test]
    fn test_parse_missing_word_list() {
        let response = serde_json::json!({
            "Response": {
                "RequestId": "req-no-words"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::SmartStructural);
        let err = parser.parse_response(&response).unwrap_err();
        assert!(err.to_string().contains("WordList"));
    }

    #[test]
    fn test_parse_text_detections_with_polygon() {
        let response = serde_json::json!({
            "Response": {
                "TextDetections": [
                    {
                        "DetectedText": "Test",
                        "Confidence": 98.0,
                        "Polygon": [
                            { "X": 10, "Y": 20 },
                            { "X": 100, "Y": 20 },
                            { "X": 100, "Y": 50 },
                            { "X": 10, "Y": 50 }
                        ]
                    }
                ],
                "RequestId": "req-polygon"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralAccurate);
        let result = parser.parse_response(&response).unwrap();
        assert_eq!(result.text, "Test");
        assert_eq!(result.word_boxes.len(), 1);
        let wb = &result.word_boxes[0];
        assert_eq!(wb.text, "Test");
        assert_eq!(wb.x, 10);
        assert_eq!(wb.y, 20);
        assert_eq!(wb.width, 90);
        assert_eq!(wb.height, 30);
    }

    #[test]
    fn test_parse_general_accurate_uses_text_detections() {
        let response = serde_json::json!({
            "Response": {
                "TextDetections": [
                    { "DetectedText": "Accurate", "Confidence": 99.0 }
                ],
                "RequestId": "req-acc"
            }
        });
        let parser = HunyuanOcrParser::new(HunyuanMode::GeneralAccurate);
        let result = parser.parse_response(&response).unwrap();
        assert_eq!(result.text, "Accurate");
    }
}
