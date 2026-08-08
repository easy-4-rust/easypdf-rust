//! PDF 输入、资源限制与原子输出基础设施。

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

mod atomic_file_output;
mod pdf_input;
mod resource_limits;

pub use atomic_file_output::AtomicFileOutput;
pub use pdf_input::PdfInput;
pub use resource_limits::ResourceLimits;
