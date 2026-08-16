//! 写入引擎抽象 trait。
//!
//! 定义 [`WriteEngine`] trait，将 PDF 文档操作抽象为引擎无关的接口。
//! 具体引擎（如 [`PrintpdfEngine`](super::printpdf_engine::PrintpdfEngine)）
//! 实现此 trait 以提供特定 PDF 后端的支持。

use easypdf_core::PdfMetadata;
use easypdf_core::error::Result;

use super::op::{PendingXObject, WriterOp};

/// PDF 写入引擎抽象。
///
/// 将 PDF 文档的创建、字体注册、页面构建和最终序列化抽象为统一接口。
/// `PdfWriter` 通过此 trait 委托所有与底层 PDF 库交互的操作。
pub(crate) trait WriteEngine {
    /// 注册自定义字体。
    ///
    /// 将 TTF/OTF 字体数据注册到文档，返回可后续引用的字体标识符。
    ///
    /// # 参数
    ///
    /// - `key`：字体注册键名（由调用方定义，用于在 `WriterOp` 中引用）。
    /// - `data`：字体文件的原始字节数据。
    ///
    /// # 错误
    ///
    /// 当字体数据无法解析时返回错误。
    fn register_font(&mut self, key: &str, data: &[u8]) -> Result<()>;

    /// 注册 XObject 资源（图片或 SVG）并返回引用标识符。
    ///
    /// # 错误
    ///
    /// 当资源数据无法解析或注册失败时返回错误。
    fn register_xobject(&mut self, xobject: PendingXObject) -> Result<String>;

    /// 添加一个已完成的页面。
    ///
    /// 页面的操作数据以 [`WriterOp`] 格式传入，由引擎负责转换和存储。
    ///
    /// # 参数
    ///
    /// - `width_pt`：页面宽度（PDF 点）。
    /// - `height_pt`：页面高度（PDF 点）。
    /// - `ops`：该页面的操作列表。
    fn add_page(&mut self, width_pt: f64, height_pt: f64, ops: Vec<WriterOp>);

    /// 完成文档构建，生成最终 PDF 字节。
    ///
    /// 将所有已添加的页面序列化为 PDF 格式，并应用文档元数据。
    ///
    /// # 错误
    ///
    /// 当序列化失败时返回错误。
    fn finish(&mut self, metadata: &PdfMetadata) -> Result<Vec<u8>>;
}
