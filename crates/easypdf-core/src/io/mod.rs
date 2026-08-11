//! PDF input, resource limits, and atomic output infrastructure.
//!
//! This module provides the I/O primitives for the `easypdf-rust` ecosystem:
//!
//! - [`PdfInput`] -- file-path or in-memory byte input with resource-limit enforcement.
//! - [`ResourceLimits`] -- configurable caps on file size, decompression, element count, etc.
//! - [`AtomicFileOutput`] -- crash-safe atomic file writes via temp-file + rename.
//! - [`guards`] -- pre-flight security checks (decompression bomb, element explosion).
//! - [`repair`] -- self-healing PDF open (dangling refs, xref rebuild).
//! - [`ssrf_guard`] -- URL validation to prevent SSRF attacks.

mod atomic_file_output;
pub mod guards;
mod pdf_input;
pub mod repair;
mod resource_limits;
pub mod ssrf_guard;

pub use atomic_file_output::AtomicFileOutput;
pub use pdf_input::PdfInput;
pub use resource_limits::ResourceLimits;
