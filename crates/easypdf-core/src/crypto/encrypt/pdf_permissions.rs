//! 加密 PDF 的权限标志。

/// 加密 PDF 的权限标志，定义于 ISO 32000-1 表 22。
///
/// 这些标志控制持有*用户密码*的用户可执行的操作。
/// 所有者密码绕过所有权限检查。
///
/// # Examples
///
/// ```
/// use easypdf_core::crypto::PdfPermissions;
///
/// let perms = PdfPermissions::PRINT | PdfPermissions::COPY;
/// assert!(perms.contains(PdfPermissions::PRINT));
/// assert!(!perms.contains(PdfPermissions::MODIFY));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PdfPermissions(u32);

bitflags::bitflags! {
    impl PdfPermissions: u32 {
        /// 打印文档。
        const PRINT = 1 << 2;
        /// 修改内容（除非受其他标志控制）。
        const MODIFY = 1 << 3;
        /// 复制或提取文本与图形。
        const COPY = 1 << 4;
        /// 添加或修改文本注释及填写交互式表单字段。
        const ADD_ANNOTATIONS = 1 << 5;
        /// 填写已有的交互式表单字段。
        const FILL_FORMS = 1 << 8;
        /// 为辅助功能提取文本。
        const EXTRACT = 1 << 9;
        /// 组装文档（插入、旋转、删除页面）。
        const ASSEMBLE = 1 << 10;
        /// 高质量打印。
        const HIGH_QUALITY_PRINT = 1 << 11;
    }
}

impl PdfPermissions {
    /// 按规范修正保留位后转换为 lopdf 权限类型。
    pub(crate) fn to_lopdf(self) -> lopdf::Permissions {
        let mut bits: u64 = u64::from(self.bits());
        // PDF 规范要求某些保留位必须设为 1。
        bits |= 0b11 << 6;
        bits |= 0b1111 << 12;
        bits |= 0xFFFF << 16;
        bits |= 0xFFFF_FFFF << 32;
        lopdf::Permissions::from_bits_retain(bits)
    }
}
