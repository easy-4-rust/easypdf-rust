//! 可观测性初始化与日志工具。
//!
//! 提供两种订阅者模式：
//!
//! - [`init_logging`]: 紧凑的人类可读输出到 stderr（开发环境）。
//! - [`init_logging_json`]: 结构化 JSON 输出到 stderr（生产环境）。
//!
//! 两者均读取 `RUST_LOG` 环境变量来控制过滤级别。
//! 语法参见 [`tracing_subscriber::EnvFilter`]。
//!
//! # Examples
//!
//! ```rust
//! // 在应用入口点中：
//! easypdf_core::logging::init_logging().ok();
//! tracing::info!("application started");
//! ```

use tracing_subscriber::EnvFilter;

/// 使用紧凑的人类可读输出初始化全局 tracing 订阅者。
///
/// 适用于开发环境。读取 `RUST_LOG` 进行级别过滤
///（未设置时默认为 `info`）。输出到 stderr。
///
/// # Errors
///
/// 全局订阅者已被设置时返回错误
///（例如之前已调用 `init_logging` 或 `init_logging_json`）。
pub fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .try_init()
}

/// 使用结构化 JSON 输出初始化全局 tracing 订阅者。
///
/// 适用于生产环境。读取 `RUST_LOG` 进行级别过滤
///（未设置时默认为 `info`）。输出到 stderr。
///
/// # Errors
///
/// 全局订阅者已被设置时返回错误
///（例如之前已调用 `init_logging` 或 `init_logging_json`）。
pub fn init_logging_json() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .try_init()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_logging_does_not_panic() {
        // `try_init` may fail if a subscriber is already set (e.g. another
        // test ran first). This must not panic.
        let _ = init_logging();
    }

    #[test]
    fn init_logging_json_does_not_panic() {
        let _ = init_logging_json();
    }
}
