//! Response parser for Baidu Cloud OCR API.
//!
//! Parses the JSON response from Baidu OCR endpoints into a standardized
//! [`OcrResult`]. Handles multiple response formats:
//!
//! - **Text APIs** (`words_result`): `GeneralBasic`, `GeneralAccurate`, `WebImage`,
//!   `Handwriting`, `Digit`, etc.
//! - **Table API** (`tables_result`): `TableRecognitionV2`
//! - **Office Document** (`results`): `OfficeDocument`
//! - **Seal** (`result`): `Seal` with `major`/`minor` words
//! - **QR Code** (`codes_result`): `Qrcode` with `text` arrays
//! - **Structured** (`words_result.struct_info.group`): `Structured` with key-value pairs
//! - **Qianfan-OCR** (`result`): Qianfan large model response
//!
//! # Error Handling
//!
//! Baidu APIs return errors in the response body (not HTTP status codes):
//!
//! ```json
//! { "error_code": 110, "error_msg": "Access token invalid" }
//! ```
//!
//! The parser checks for these fields and returns [`BaiduError::Api`].

mod core;
mod parsers;
#[cfg(test)]
mod tests;

pub use core::BaiduOcrParser;
