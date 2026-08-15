//! 百度云 OCR API 响应解析器。
//!
//! 将百度 OCR 端点的 JSON 响应解析为标准化的
//! [`OcrResult`](easypdf_markdown::ocr::OcrResult)。处理多种响应格式：
//!
//! - **文本 API**（`words_result`）：`GeneralBasic`、`GeneralAccurate`、`WebImage`、
//!   `Handwriting`、`Digit` 等
//! - **表格 API**（`tables_result`）：`TableRecognitionV2`
//! - **办公文档**（`results`）：`OfficeDocument`
//! - **印章**（`result`）：`Seal`，含 `major`/`minor` 文字
//! - **二维码**（`codes_result`）：`Qrcode`，含 `text` 数组
//! - **结构化**（`words_result.struct_info.group`）：`Structured`，含键值对
//! - **千帆 OCR**（`result`）：千帆大模型响应
//!
//! # 错误处理
//!
//! 百度 API 在响应体中返回错误（而非 HTTP 状态码）：
//!
//! ```json
//! { "error_code": 110, "error_msg": "Access token invalid" }
//! ```
//!
//! 解析器检查这些字段并返回
//! [`BaiduError::Api`](crate::baidu::config::BaiduError::Api)。

mod core;
mod parsers;
#[cfg(test)]
mod tests;

pub use core::BaiduOcrParser;
