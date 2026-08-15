//! [`PdfReader`] 的定义与核心打开/构造方法。

use std::ops::Range;
use std::path::Path;

use easypdf_core::error::{PdfError, Result};
use easypdf_core::io::guards::guard_element_explosion;
use easypdf_core::io::repair::{RepairOptions, attempt_repair, is_likely_corrupt};
use easypdf_core::{PageRange, PdfInput, ResourceLimits};

use crate::strategy::ReadStrategy;

/// 从 PDF 文档中提取内容的读取器。
///
/// 底层使用 `lopdf` crate 进行低层 PDF 解析。支持多种读取策略
/// （[`ReadStrategy`])，可根据文档大小自动选择最优解析方式。
///
/// # Examples
///
/// ```no_run
/// use easypdf_reader::PdfReader;
///
/// let text = PdfReader::open("document.pdf")?.extract_text()?;
/// # Ok::<(), easypdf_core::PdfError>(())
/// ```
pub struct PdfReader {
    /// 已解析的文档对象（[`ReadStrategy::Streaming`] 模式下为 `None`）。
    pub(super) document: Option<lopdf::Document>,
    pub(super) pages: Option<PageRange>,
    pub(super) limits: ResourceLimits,
    pub(super) strategy: ReadStrategy,
    /// 原始 PDF 字节 -- 供 Streaming 策略使用。
    pub(super) raw_bytes: Vec<u8>,
}

impl PdfReader {
    /// 打开 PDF 文件进行读取，自动选择解析策略。
    ///
    /// 根据文件大小自动选择 [`ReadStrategy`]：
    /// 5 MB 以下使用 [`Full`](ReadStrategy::Full)，
    /// 5--100 MB 使用 [`Lazy`](ReadStrategy::Lazy)，
    /// 更大的文件使用 [`Streaming`](ReadStrategy::Streaming)。
    ///
    /// # Errors
    ///
    /// 当文件无法打开或不是有效的 PDF 时，返回 `PdfError::Parse`。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file_size = std::fs::metadata(path).map_or(0, |m| m.len());
        let strategy = ReadStrategy::auto(file_size);
        Self::open_with_strategy(path, strategy)
    }

    /// 从内存字节打开 PDF，自动选择解析策略。
    ///
    /// # Errors
    ///
    /// 当字节数据不是有效的 PDF 时，返回 [`PdfError::Parse`]。
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let bytes = bytes.into();
        let file_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let strategy = ReadStrategy::auto(file_size);
        let input = PdfInput::from_bytes(bytes);
        Self::open_with_limits_and_strategy(&input, ResourceLimits::default(), strategy)
    }

    /// 使用指定的资源限制打开 PDF 输入。
    ///
    /// 文档仅解析一次并由读取器会话保留。
    ///
    /// # Errors
    ///
    /// 当输入超出限制或解析失败时返回错误。
    pub fn open_with_limits(input: &PdfInput, limits: ResourceLimits) -> Result<Self> {
        let bytes = input.read(limits)?;
        let file_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let strategy = ReadStrategy::auto(file_size);
        Self::load_from_bytes(bytes, limits, strategy)
    }

    /// 使用指定的 [`ReadStrategy`] 打开 PDF 文件。
    ///
    /// # Errors
    ///
    /// 当文件无法读取、解析或超出资源限制时返回错误。
    pub fn open_with_strategy(path: impl AsRef<Path>, strategy: ReadStrategy) -> Result<Self> {
        let input = PdfInput::from_path(path.as_ref());
        Self::open_with_limits_and_strategy(&input, ResourceLimits::default(), strategy)
    }

    /// 使用指定的修复选项和读取策略打开 PDF 文件。
    ///
    /// 如果 [`is_likely_corrupt`] 检测到损坏，将使用提供的
    /// [`RepairOptions`] 调用 [`attempt_repair`] 进行修复后再加载。
    ///
    /// # Errors
    ///
    /// 当文件无法读取、修复、解析或超出资源限制时返回错误。
    pub fn open_with_repair(
        path: impl AsRef<Path>,
        repair: RepairOptions,
        strategy: ReadStrategy,
    ) -> Result<Self> {
        let input = PdfInput::from_path(path.as_ref());

        let bytes = if is_likely_corrupt(&input) {
            attempt_repair(&input, &repair)?
        } else {
            input.read(ResourceLimits::default())?
        };

        Self::load_from_bytes(bytes, ResourceLimits::default(), strategy)
    }

    /// 使用指定的资源限制和读取策略打开 PDF 输入。
    ///
    /// # Errors
    ///
    /// 当输入超出限制或解析失败时返回错误。
    pub fn open_with_limits_and_strategy(
        input: &PdfInput,
        limits: ResourceLimits,
        strategy: ReadStrategy,
    ) -> Result<Self> {
        let bytes = input.read(limits)?;
        Self::load_from_bytes(bytes, limits, strategy)
    }

    /// 内部方法：从原始字节加载 PDF，使用给定的限制和策略。
    ///
    /// 解析前应用安全防护（元素爆炸检测）。`Streaming` 策略完全跳过
    /// `lopdf::Document` 的构建。
    fn load_from_bytes(
        bytes: Vec<u8>,
        limits: ResourceLimits,
        strategy: ReadStrategy,
    ) -> Result<Self> {
        if strategy == ReadStrategy::Streaming {
            // Streaming 模式：不需要 lopdf::Document -- 直接扫描原始字节。
            return Ok(Self {
                document: None,
                pages: None,
                limits,
                strategy,
                raw_bytes: bytes,
            });
        }

        let document = lopdf::Document::load_mem(&bytes)
            .map_err(|error| PdfError::Parse(error.to_string()))?;

        // 安全防护：元素爆炸 -- 检查对象总数。
        let element_count = document.objects.len();
        guard_element_explosion(element_count, &limits)?;

        let page_count = document.get_pages().len();
        if page_count > limits.max_pages() {
            return Err(PdfError::ResourceLimitExceeded {
                resource: "pages",
                limit: usize_to_u64_saturating(limits.max_pages()),
                actual: usize_to_u64_saturating(page_count),
            });
        }

        Ok(Self {
            document: Some(document),
            pages: None,
            limits,
            strategy,
            raw_bytes: bytes,
        })
    }

    /// 返回此读取器打开时使用的读取策略。
    #[must_use]
    pub const fn strategy(&self) -> ReadStrategy {
        self.strategy
    }

    /// 将提取范围限制为指定的页面范围（从 0 开始）。
    #[must_use]
    pub fn pages(mut self, range: Range<usize>) -> Self {
        let start = range.start;
        self.pages = Some(match PageRange::new(range) {
            Ok(pages) => pages,
            Err(_) => PageRange::empty_at(start),
        });
        self
    }

    /// 尝试将提取范围限制为经验证的从零开始的页面范围。
    ///
    /// # Errors
    ///
    /// 当范围反转（起始大于结束）时返回错误。
    pub fn try_pages(mut self, range: Range<usize>) -> Result<Self> {
        self.pages = Some(PageRange::new(range)?);
        Ok(self)
    }
}

/// 将 `usize` 转换为 `u64`，溢出时饱和到 `u64::MAX`。
pub(crate) fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
