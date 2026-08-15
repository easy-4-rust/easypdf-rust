//! OCR 引擎实现。
//!
//! 默认提供模拟引擎，可选 feature 门控的后端：
//!
//! - [`MockOcrEngine`] -- 返回固定文本（始终可用，用于测试）
//! - `OcrsEngine` -- 基于 `ocrs` crate 的纯 Rust OCR（feature `ocrs`）
//! - `LlmOcrEngine` -- 基于 `rig-core` 的 LLM Vision API（feature `llm`）
//! - `DeepSeekOcrEngine` -- 基于 `rig-core` 的 DeepSeek-OCR-2（feature `llm`）

mod mock;

pub use mock::MockOcrEngine;

#[cfg(feature = "ocrs")]
mod ocrs_backend;

#[cfg(feature = "ocrs")]
pub use ocrs_backend::OcrsEngine;

#[cfg(feature = "llm")]
mod llm_backend;

#[cfg(feature = "llm")]
pub use llm_backend::{DeepSeekOcrEngine, LlmOcrEngine};
