use super::*;
use crate::baidu::config::{BaiduApi, BaiduError};

#[test]
fn test_parse_words_response() {
    let raw = serde_json::json!({
        "words_result": [
            { "words": "Hello World" },
            { "words": "Line 2" }
        ],
        "words_result_num": 2
    });
    let parser = BaiduOcrParser::new(BaiduApi::GeneralBasic);
    let result = parser.parse(&raw).unwrap();
    assert_eq!(result.text, "Hello World\nLine 2");
    assert!(result.word_boxes.is_empty());
}

#[test]
fn test_parse_words_response_with_location() {
    let raw = serde_json::json!({
        "words_result": [
            {
                "words": "OCR Text",
                "location": { "left": 10, "top": 20, "width": 100, "height": 30 }
            }
        ],
        "words_result_num": 1
    });
    let parser = BaiduOcrParser::new(BaiduApi::GeneralBasicWithLocation);
    let result = parser.parse(&raw).unwrap();
    assert_eq!(result.text, "OCR Text");
    assert_eq!(result.word_boxes.len(), 1);
    assert_eq!(result.word_boxes[0].x, 10);
    assert_eq!(result.word_boxes[0].y, 20);
    assert_eq!(result.word_boxes[0].width, 100);
    assert_eq!(result.word_boxes[0].height, 30);
}

#[test]
fn test_parse_table_response() {
    let raw = serde_json::json!({
        "tables_result": [{
            "body": [
                { "words": "Name", "row_start": 0, "col_start": 0, "row_end": 0, "col_end": 0 },
                { "words": "Age", "row_start": 0, "col_start": 1, "row_end": 0, "col_end": 1 },
                { "words": "Alice", "row_start": 1, "col_start": 0, "row_end": 1, "col_end": 0 },
                { "words": "30", "row_start": 1, "col_start": 1, "row_end": 1, "col_end": 1 }
            ]
        }]
    });
    let parser = BaiduOcrParser::new(BaiduApi::TableRecognitionV2);
    let result = parser.parse(&raw).unwrap();
    let lines: Vec<&str> = result.text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "Name\tAge");
    assert_eq!(lines[1], "Alice\t30");
}

#[test]
fn test_parse_qianfan_response() {
    let raw = serde_json::json!({
        "result": {
            "text": "Qianfan OCR result"
        }
    });
    let parser = BaiduOcrParser::new(BaiduApi::QianfanOcr);
    let result = parser.parse(&raw).unwrap();
    assert_eq!(result.text, "Qianfan OCR result");
}

#[test]
fn test_parse_error_response() {
    let raw = serde_json::json!({
        "error_code": 110,
        "error_msg": "Access token invalid or expired"
    });
    let parser = BaiduOcrParser::new(BaiduApi::GeneralBasic);
    let err = parser.parse(&raw).unwrap_err();
    match err {
        BaiduError::Api { code, message } => {
            assert_eq!(code, 110);
            assert!(message.contains("Access token"));
        }
        other => panic!("expected BaiduError::Api, got {other:?}"),
    }
}

#[test]
fn test_parse_missing_words_result() {
    let raw = serde_json::json!({ "log_id": 123 });
    let parser = BaiduOcrParser::new(BaiduApi::GeneralBasic);
    let err = parser.parse(&raw).unwrap_err();
    assert!(matches!(err, BaiduError::InvalidResponse(_)));
}

#[test]
fn test_parse_empty_words_result() {
    let raw = serde_json::json!({
        "words_result": [],
        "words_result_num": 0
    });
    let parser = BaiduOcrParser::new(BaiduApi::GeneralBasic);
    let result = parser.parse(&raw).unwrap();
    assert!(result.text.is_empty());
}

#[test]
fn test_parse_qianfan_fallback_to_words() {
    // Qianfan response without "result.text" should fall back to `words_result`.
    let raw = serde_json::json!({
        "words_result": [
            { "words": "fallback text" }
        ],
        "words_result_num": 1
    });
    let parser = BaiduOcrParser::new(BaiduApi::QianfanOcr);
    let result = parser.parse(&raw).unwrap();
    assert_eq!(result.text, "fallback text");
}
