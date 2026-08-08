//! PDF 操作的资源限制。

/// PDF 读取与转换过程的资源上限。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct ResourceLimits {
    max_input_bytes: u64,
    max_pages: usize,
    max_extracted_text_bytes: usize,
}

impl ResourceLimits {
    /// 创建默认资源限制。
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_input_bytes: 256 * 1024 * 1024,
            max_pages: 10_000,
            max_extracted_text_bytes: 128 * 1024 * 1024,
        }
    }

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
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}
