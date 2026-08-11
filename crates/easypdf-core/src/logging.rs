//! Observability initialization and logging utilities.
//!
//! Provides two subscriber modes:
//!
//! - [`init_logging`]: compact human-readable output to stderr (development).
//! - [`init_logging_json`]: structured JSON output to stderr (production).
//!
//! Both read the `RUST_LOG` environment variable to control filtering.
//! See [`tracing_subscriber::EnvFilter`] for syntax.
//!
//! # Examples
//!
//! ```rust
//! // In your application entry point:
//! easypdf_core::logging::init_logging().ok();
//! tracing::info!("application started");
//! ```

use tracing_subscriber::EnvFilter;

/// Initialize the global tracing subscriber with compact human-readable output.
///
/// Intended for development. Reads `RUST_LOG` for level filtering
/// (defaults to `info` if unset). Output goes to stderr.
///
/// # Errors
///
/// Returns an error if the global subscriber has already been set
/// (e.g. by a previous call to `init_logging` or `init_logging_json`).
pub fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .try_init()
}

/// Initialize the global tracing subscriber with structured JSON output.
///
/// Intended for production. Reads `RUST_LOG` for level filtering
/// (defaults to `info` if unset). Output goes to stderr.
///
/// # Errors
///
/// Returns an error if the global subscriber has already been set
/// (e.g. by a previous call to `init_logging` or `init_logging_json`).
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
