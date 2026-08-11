//! PDF 操作的资源限制。

/// PDF 读取与转换过程的资源上限。
///
/// # Examples
///
/// ```
/// use easypdf_core::ResourceLimits;
///
/// let limits = ResourceLimits::new()
///     .with_max_input_bytes(100 * 1024 * 1024)
///     .with_max_decompressed_size(512 * 1024 * 1024);
/// assert_eq!(limits.max_input_bytes(), 100 * 1024 * 1024);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct ResourceLimits {
    max_input_bytes: u64,
    max_pages: usize,
    max_extracted_text_bytes: usize,
    max_decompressed_size: u64,
    max_compression_ratio: u32,
    max_element_count: usize,
}

impl ResourceLimits {
    /// 创建默认资源限制。
    ///
    /// 默认值适用于一般用途的 PDF 处理。对于不可信输入，
    /// 考虑使用 [`Self::strict`]；对于受信任的内部文档，
    /// 考虑使用 [`Self::permissive`]。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_input_bytes: 256 * 1024 * 1024,
            max_pages: 10_000,
            max_extracted_text_bytes: 128 * 1024 * 1024,
            max_decompressed_size: 2 * 1024 * 1024 * 1024,
            max_compression_ratio: 100,
            max_element_count: 5_000_000,
        }
    }

    /// 创建严格资源限制，适用于处理不可信输入。
    ///
    /// 所有限制为默认值的约 1/4，能更早拒绝恶意文件。
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_pages: 2_500,
            max_extracted_text_bytes: 32 * 1024 * 1024,
            max_decompressed_size: 512 * 1024 * 1024,
            max_compression_ratio: 50,
            max_element_count: 1_000_000,
        }
    }

    /// 创建宽松资源限制，适用于受信任的大型文档。
    ///
    /// 所有限制为默认值的约 4 倍。
    #[must_use]
    pub const fn permissive() -> Self {
        Self {
            max_input_bytes: 1024 * 1024 * 1024,
            max_pages: 100_000,
            max_extracted_text_bytes: 512 * 1024 * 1024,
            max_decompressed_size: 8 * 1024 * 1024 * 1024,
            max_compression_ratio: 400,
            max_element_count: 20_000_000,
        }
    }

    // --- Builder methods ---

    /// 设置最大输入字节数。
    #[must_use]
    pub const fn with_max_input_bytes(mut self, value: u64) -> Self {
        self.max_input_bytes = value;
        self
    }

    /// 设置最大页数。
    #[must_use]
    pub const fn with_max_pages(mut self, value: usize) -> Self {
        self.max_pages = value;
        self
    }

    /// 设置最大提取文本字节数。
    #[must_use]
    pub const fn with_max_extracted_text_bytes(mut self, value: usize) -> Self {
        self.max_extracted_text_bytes = value;
        self
    }

    /// 设置解压后最大字节数（防解压炸弹）。
    #[must_use]
    pub const fn with_max_decompressed_size(mut self, value: u64) -> Self {
        self.max_decompressed_size = value;
        self
    }

    /// 设置最大压缩比（解压后/压缩前）。
    #[must_use]
    pub const fn with_max_compression_ratio(mut self, value: u32) -> Self {
        self.max_compression_ratio = value;
        self
    }

    /// 设置最大 PDF 对象/元素数量。
    #[must_use]
    pub const fn with_max_element_count(mut self, value: usize) -> Self {
        self.max_element_count = value;
        self
    }

    // --- Getters ---

    /// 返回最大输入字节数。
    #[must_use]
    pub const fn max_input_bytes(self) -> u64 {
        self.max_input_bytes
    }

    /// 返回最大页数。
    #[must_use]
    pub const fn max_pages(self) -> usize {
        self.max_pages
    }

    /// 返回最大提取文本字节数。
    #[must_use]
    pub const fn max_extracted_text_bytes(self) -> usize {
        self.max_extracted_text_bytes
    }

    /// 返回解压后最大字节数。
    #[must_use]
    pub const fn max_decompressed_size(self) -> u64 {
        self.max_decompressed_size
    }

    /// 返回最大压缩比。
    #[must_use]
    pub const fn max_compression_ratio(self) -> u32 {
        self.max_compression_ratio
    }

    /// 返回最大 PDF 对象/元素数量。
    #[must_use]
    pub const fn max_element_count(self) -> usize {
        self.max_element_count
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_are_sensible() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_input_bytes(), 256 * 1024 * 1024);
        assert_eq!(limits.max_pages(), 10_000);
        assert_eq!(limits.max_decompressed_size(), 2 * 1024 * 1024 * 1024);
        assert_eq!(limits.max_compression_ratio(), 100);
        assert_eq!(limits.max_element_count(), 5_000_000);
    }

    #[test]
    fn strict_is_more_restrictive() {
        let default = ResourceLimits::default();
        let strict = ResourceLimits::strict();
        assert!(strict.max_input_bytes() < default.max_input_bytes());
        assert!(strict.max_pages() < default.max_pages());
        assert!(strict.max_decompressed_size() < default.max_decompressed_size());
        assert!(strict.max_compression_ratio() < default.max_compression_ratio());
        assert!(strict.max_element_count() < default.max_element_count());
    }

    #[test]
    fn permissive_is_more_relaxed() {
        let default = ResourceLimits::default();
        let permissive = ResourceLimits::permissive();
        assert!(permissive.max_input_bytes() > default.max_input_bytes());
        assert!(permissive.max_pages() > default.max_pages());
        assert!(permissive.max_decompressed_size() > default.max_decompressed_size());
        assert!(permissive.max_compression_ratio() > default.max_compression_ratio());
        assert!(permissive.max_element_count() > default.max_element_count());
    }

    #[test]
    fn builder_overrides_all_fields() {
        let limits = ResourceLimits::new()
            .with_max_input_bytes(1024)
            .with_max_pages(5)
            .with_max_extracted_text_bytes(2048)
            .with_max_decompressed_size(4096)
            .with_max_compression_ratio(10)
            .with_max_element_count(100);
        assert_eq!(limits.max_input_bytes(), 1024);
        assert_eq!(limits.max_pages(), 5);
        assert_eq!(limits.max_extracted_text_bytes(), 2048);
        assert_eq!(limits.max_decompressed_size(), 4096);
        assert_eq!(limits.max_compression_ratio(), 10);
        assert_eq!(limits.max_element_count(), 100);
    }
}
