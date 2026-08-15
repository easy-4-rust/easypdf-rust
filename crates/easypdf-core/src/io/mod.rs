//! PDF 输入、资源限制与原子化输出基础设施。
//!
//! 本模块为 `easypdf-rust` 生态提供 I/O 原语：
//!
//! - [`PdfInput`] -- 文件路径或内存字节输入，带资源限制强制执行。
//! - [`ResourceLimits`] -- 可配置的文件大小、解压、元素数量等上限。
//! - [`AtomicFileOutput`] -- 通过临时文件 + 重命名实现崩溃安全的原子写入。
//! - [`guards`] -- 预检安全检查（解压炸弹、元素爆炸）。
//! - [`repair`] -- 自修复 PDF 打开（悬挂引用、xref 重建）。
//! - [`ssrf_guard`] -- URL 校验以防止 SSRF 攻击。

mod atomic_file_output;
pub mod guards;
mod pdf_input;
pub mod repair;
mod resource_limits;
pub mod ssrf_guard;

pub use atomic_file_output::AtomicFileOutput;
pub use pdf_input::PdfInput;
pub use resource_limits::ResourceLimits;
