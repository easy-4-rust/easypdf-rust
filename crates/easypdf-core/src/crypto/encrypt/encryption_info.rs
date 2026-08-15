//! 加密 PDF 的元数据信息。

use super::PdfEncryptionAlgorithm;

/// 加密 PDF 中关于加密方案的元数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptionInfo {
    /// 加密算法族。
    pub algorithm: PdfEncryptionAlgorithm,
    /// 加密字典中的 `/V` 值。
    pub version: i64,
    /// 加密字典中的 `/R` 值。
    pub revision: i64,
    /// `/Length` 值（比特），如果存在。
    pub key_length_bits: Option<u16>,
}
