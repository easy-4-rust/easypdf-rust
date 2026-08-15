use easypdf_markdown::ocr::{OcrResult, WordBox};

use super::core::json_u32;
use crate::baidu::config::{BaiduError, BaiduResult};

/// 解析标准 `words_result` 响应。
///
/// Response format:
/// ```json
/// {
///   "words_result": [
///     { "words": "line 1" },
///     { "words": "line 2" }
///   ],
///   "words_result_num": 2
/// }
/// ```
pub(crate) fn parse_words_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    let words_result = raw
        .get("words_result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            BaiduError::InvalidResponse("missing or invalid `words_result` array".to_owned())
        })?;

    let mut lines = Vec::new();
    let mut word_boxes = Vec::new();

    for item in words_result {
        let words = item.get("words").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(words);

        // 提取位置信息（如有）。
        if let Some(loc) = item.get("location") {
            let x = json_u32(loc.get("left"));
            let y = json_u32(loc.get("top"));
            let width = json_u32(loc.get("width"));
            let height = json_u32(loc.get("height"));
            word_boxes.push(WordBox {
                text: words.to_owned(),
                x,
                y,
                width,
                height,
                confidence: None,
            });
        }
    }

    Ok(OcrResult {
        text: lines.join("\n"),
        confidence: None,
        word_boxes,
    })
}

/// 解析表格识别响应。
///
/// Response format:
/// ```json
/// {
///   "tables_result": [
///     {
///       "body": [
///         { "words": "cell text", "row_start": 0, "col_start": 0, ... }
///       ]
///     }
///   ]
/// }
/// ```
pub(crate) fn parse_table_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    let tables = raw
        .get("tables_result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            BaiduError::InvalidResponse("missing or invalid `tables_result` array".to_owned())
        })?;

    let mut all_text = Vec::new();

    for table in tables {
        // 从 body 收集单元格。
        if let Some(body) = table.get("body").and_then(serde_json::Value::as_array) {
            // 按行分组单元格以进行表格输出。
            let mut cells: Vec<(u64, u64, String)> = Vec::new();

            for cell in body {
                let words = cell
                    .get("words")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let row = cell
                    .get("row_start")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let col = cell
                    .get("col_start")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                cells.push((row, col, words));
            }

            // 按 (row, col) 排序并格式化为制表符分隔。
            cells.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

            let mut current_row = 0u64;
            let mut row_cells = Vec::new();
            for (row, _col, words) in &cells {
                if *row != current_row && !row_cells.is_empty() {
                    all_text.push(row_cells.join("\t"));
                    row_cells.clear();
                    current_row = *row;
                }
                row_cells.push(words.clone());
            }
            if !row_cells.is_empty() {
                all_text.push(row_cells.join("\t"));
            }

            // 若无 body 单元格，尝试 header。
            if all_text.is_empty()
                && let Some(header) = table.get("header").and_then(serde_json::Value::as_array)
            {
                for cell in header {
                    let words = cell.get("words").and_then(|v| v.as_str()).unwrap_or("");
                    all_text.push(words.to_owned());
                }
            }
        }
    }

    Ok(OcrResult {
        text: all_text.join("\n"),
        confidence: None,
        word_boxes: Vec::new(),
    })
}

/// 解析千帆 OCR 响应。
///
/// Qianfan-OCR returns a different format:
/// ```json
/// {
///   "result": {
///     "text": "extracted text"
///   }
/// }
/// ```
pub(crate) fn parse_qianfan_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    // 首先尝试 "result" 字段（千帆 OCR 格式）。
    if let Some(result) = raw.get("result")
        && let Some(text) = result.get("text").and_then(|v| v.as_str())
    {
        return Ok(OcrResult {
            text: text.to_owned(),
            confidence: None,
            word_boxes: Vec::new(),
        });
    }

    // 回退到 words_result 格式（部分千帆端点使用此格式）。
    parse_words_response(raw)
}

/// 解析办公文档识别响应。
///
/// Response format:
/// ```json
/// {
///   "results": [
///     {
///       "words_type": "print",
///       "words": [
///         { "word": "line text" }
///       ]
///     }
///   ],
///   "results_num": 1
/// }
/// ```
pub(crate) fn parse_office_doc_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    let results = raw
        .get("results")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            BaiduError::InvalidResponse("missing or invalid `results` array".to_owned())
        })?;

    let mut lines = Vec::new();

    for item in results {
        // 每个 result 有一个包含 word 对象的 "words" 数组。
        if let Some(words) = item.get("words").and_then(serde_json::Value::as_array) {
            for word_obj in words {
                if let Some(word) = word_obj.get("word").and_then(|v| v.as_str()) {
                    lines.push(word.to_owned());
                }
            }
        }
    }

    Ok(OcrResult {
        text: lines.join("\n"),
        confidence: None,
        word_boxes: Vec::new(),
    })
}

/// 解析印章识别响应。
///
/// Response format:
/// ```json
/// {
///   "result": [
///     {
///       "type": "circle",
///       "major": { "words": "Company Name", "probability": 0.99 },
///       "minor": [{ "words": "Department", "probability": 0.92 }],
///       "location": { "left": 10, "top": 20, "width": 100, "height": 100 },
///       "color": "red"
///     }
///   ],
///   "result_num": 1
/// }
/// ```
pub(crate) fn parse_seal_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    let result = raw
        .get("result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            BaiduError::InvalidResponse("missing or invalid `result` array".to_owned())
        })?;

    let mut lines = Vec::new();

    for seal in result {
        // 提取主文字。
        if let Some(words) = seal
            .get("major")
            .and_then(|m| m.get("words"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            lines.push(words.to_owned());
        }
        // 提取次文字项。
        if let Some(minor) = seal.get("minor").and_then(serde_json::Value::as_array) {
            for item in minor {
                if let Some(words) = item
                    .get("words")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    lines.push(words.to_owned());
                }
            }
        }
    }

    Ok(OcrResult {
        text: lines.join("\n"),
        confidence: None,
        word_boxes: Vec::new(),
    })
}

/// 解析二维码识别响应。
///
/// Response format:
/// ```json
/// {
///   "codes_result": [
///     {
///       "type": "QR_CODE",
///       "text": ["https://example.com"]
///     }
///   ],
///   "codes_result_num": 1
/// }
/// ```
pub(crate) fn parse_qrcode_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    let codes = raw
        .get("codes_result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            BaiduError::InvalidResponse("missing or invalid `codes_result` array".to_owned())
        })?;

    let mut lines = Vec::new();

    for code in codes {
        if let Some(texts) = code.get("text").and_then(serde_json::Value::as_array) {
            for text in texts {
                if let Some(s) = text.as_str() {
                    lines.push(s.to_owned());
                }
            }
        }
    }

    Ok(OcrResult {
        text: lines.join("\n"),
        confidence: None,
        word_boxes: Vec::new(),
    })
}

/// 解析智能结构化响应。
///
/// Response format:
/// ```json
/// {
///   "words_result": {
///     "struct_info": {
///       "group": [
///         {
///           "key": [{ "word": "Name" }],
///           "value": [{ "word": "Alice" }]
///         }
///       ]
///     }
///   }
/// }
/// ```
pub(crate) fn parse_structured_response(raw: &serde_json::Value) -> BaiduResult<OcrResult> {
    let words_result = raw
        .get("words_result")
        .ok_or_else(|| BaiduError::InvalidResponse("missing `words_result` object".to_owned()))?;

    let mut lines = Vec::new();

    // 导航：words_result -> struct_info -> group[]
    if let Some(groups) = words_result
        .get("struct_info")
        .and_then(|si| si.get("group"))
        .and_then(serde_json::Value::as_array)
    {
        for group in groups {
            let key_text = extract_struct_words(group.get("key"));
            let value_text = extract_struct_words(group.get("value"));
            if !key_text.is_empty() || !value_text.is_empty() {
                lines.push(format!("{key_text}: {value_text}"));
            }
        }
    }

    Ok(OcrResult {
        text: lines.join("\n"),
        confidence: None,
        word_boxes: Vec::new(),
    })
}

/// 从结构化字段数组中提取拼接的文字。
///
/// 数组中的每个元素是包含 `"word"` 键的对象。
fn extract_struct_words(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("word").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baidu::config::BaiduApi;
    use crate::baidu::parser::BaiduOcrParser;

    // --- OfficeDocument tests ---

    #[test]
    fn test_parse_office_doc_response() {
        let raw = serde_json::json!({
            "results": [
                {
                    "words_type": "print",
                    "words": [
                        { "word": "Invoice #12345" },
                        { "word": "Date: 2024-01-15" }
                    ]
                },
                {
                    "words_type": "handwriting",
                    "words": [
                        { "word": "Signed by Alice" }
                    ]
                }
            ],
            "results_num": 2
        });
        let parser = BaiduOcrParser::new(BaiduApi::OfficeDocument);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(
            result.text,
            "Invoice #12345\nDate: 2024-01-15\nSigned by Alice"
        );
    }

    #[test]
    fn test_parse_office_doc_empty() {
        let raw = serde_json::json!({
            "results": [],
            "results_num": 0
        });
        let parser = BaiduOcrParser::new(BaiduApi::OfficeDocument);
        let result = parser.parse(&raw).unwrap();
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_parse_office_doc_missing_results() {
        let raw = serde_json::json!({ "log_id": 123 });
        let parser = BaiduOcrParser::new(BaiduApi::OfficeDocument);
        let err = parser.parse(&raw).unwrap_err();
        assert!(matches!(err, BaiduError::InvalidResponse(_)));
    }

    // --- Seal tests ---

    #[test]
    fn test_parse_seal_response() {
        let raw = serde_json::json!({
            "result": [
                {
                    "type": "circle",
                    "major": { "words": "Beijing Company Ltd", "probability": 0.9999 },
                    "minor": [
                        { "words": "HR Department", "probability": 0.9238 }
                    ],
                    "location": { "top": 768, "left": 676, "width": 132, "height": 130 },
                    "color": "red"
                }
            ],
            "result_num": 1
        });
        let parser = BaiduOcrParser::new(BaiduApi::Seal);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(result.text, "Beijing Company Ltd\nHR Department");
    }

    #[test]
    fn test_parse_seal_multiple_seals() {
        let raw = serde_json::json!({
            "result": [
                {
                    "type": "circle",
                    "major": { "words": "Company A", "probability": 0.99 },
                    "minor": [],
                    "color": "red"
                },
                {
                    "type": "ellipse",
                    "major": { "words": "Company B", "probability": 0.98 },
                    "minor": [{ "words": "Finance", "probability": 0.95 }],
                    "color": "blue"
                }
            ],
            "result_num": 2
        });
        let parser = BaiduOcrParser::new(BaiduApi::Seal);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(result.text, "Company A\nCompany B\nFinance");
    }

    #[test]
    fn test_parse_seal_empty() {
        let raw = serde_json::json!({
            "result": [],
            "result_num": 0
        });
        let parser = BaiduOcrParser::new(BaiduApi::Seal);
        let result = parser.parse(&raw).unwrap();
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_parse_seal_missing_result() {
        let raw = serde_json::json!({ "log_id": 123 });
        let parser = BaiduOcrParser::new(BaiduApi::Seal);
        let err = parser.parse(&raw).unwrap_err();
        assert!(matches!(err, BaiduError::InvalidResponse(_)));
    }

    // --- Qrcode tests ---

    #[test]
    fn test_parse_qrcode_response() {
        let raw = serde_json::json!({
            "codes_result": [
                {
                    "type": "QR_CODE",
                    "text": ["https://example.com"]
                }
            ],
            "codes_result_num": 1
        });
        let parser = BaiduOcrParser::new(BaiduApi::Qrcode);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(result.text, "https://example.com");
    }

    #[test]
    fn test_parse_qrcode_multiple() {
        let raw = serde_json::json!({
            "codes_result": [
                {
                    "type": "QR_CODE",
                    "text": ["https://example.com", "backup-url"]
                },
                {
                    "type": "EAN_13",
                    "text": ["4901234567890"]
                }
            ],
            "codes_result_num": 2
        });
        let parser = BaiduOcrParser::new(BaiduApi::Qrcode);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(
            result.text,
            "https://example.com\nbackup-url\n4901234567890"
        );
    }

    #[test]
    fn test_parse_qrcode_empty() {
        let raw = serde_json::json!({
            "codes_result": [],
            "codes_result_num": 0
        });
        let parser = BaiduOcrParser::new(BaiduApi::Qrcode);
        let result = parser.parse(&raw).unwrap();
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_parse_qrcode_missing_codes_result() {
        let raw = serde_json::json!({ "log_id": 123 });
        let parser = BaiduOcrParser::new(BaiduApi::Qrcode);
        let err = parser.parse(&raw).unwrap_err();
        assert!(matches!(err, BaiduError::InvalidResponse(_)));
    }

    // --- Structured tests ---

    #[test]
    fn test_parse_structured_response() {
        let raw = serde_json::json!({
            "words_result": {
                "struct_info": {
                    "group": [
                        {
                            "key": [{ "word": "Name" }],
                            "value": [{ "word": "Alice" }]
                        },
                        {
                            "key": [{ "word": "Date" }],
                            "value": [{ "word": "2024" }, { "word": "-01-15" }]
                        }
                    ]
                }
            }
        });
        let parser = BaiduOcrParser::new(BaiduApi::Structured);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(result.text, "Name: Alice\nDate: 2024-01-15");
    }

    #[test]
    fn test_parse_structured_empty_groups() {
        let raw = serde_json::json!({
            "words_result": {
                "struct_info": {
                    "group": []
                }
            }
        });
        let parser = BaiduOcrParser::new(BaiduApi::Structured);
        let result = parser.parse(&raw).unwrap();
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_parse_structured_missing_words_result() {
        let raw = serde_json::json!({ "log_id": 123 });
        let parser = BaiduOcrParser::new(BaiduApi::Structured);
        let err = parser.parse(&raw).unwrap_err();
        assert!(matches!(err, BaiduError::InvalidResponse(_)));
    }

    #[test]
    fn test_parse_structured_missing_struct_info() {
        let raw = serde_json::json!({
            "words_result": {}
        });
        let parser = BaiduOcrParser::new(BaiduApi::Structured);
        let result = parser.parse(&raw).unwrap();
        // Missing struct_info is not an error, just yields empty text.
        assert!(result.text.is_empty());
    }

    // --- Handwriting and Digit use words_result, verify routing ---

    #[test]
    fn test_parse_handwriting_uses_words_result() {
        let raw = serde_json::json!({
            "words_result": [
                { "words": "hello" },
                { "words": "world" }
            ],
            "words_result_num": 2
        });
        let parser = BaiduOcrParser::new(BaiduApi::Handwriting);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(result.text, "hello\nworld");
    }

    #[test]
    fn test_parse_digit_uses_words_result() {
        let raw = serde_json::json!({
            "words_result": [
                { "words": "12345" }
            ],
            "words_result_num": 1
        });
        let parser = BaiduOcrParser::new(BaiduApi::Digit);
        let result = parser.parse(&raw).unwrap();
        assert_eq!(result.text, "12345");
    }
}
