//! 流式扫描结果 [`StreamScanResult`] 的定义。

/// 流式扫描一次的结果。
#[derive(Debug, Clone, Default)]
pub(crate) struct StreamScanResult {
    /// 检测到的页面数量（启发式：统计 `/Type /Page` 条目）。
    pub pages_scanned: usize,
    /// 已处理的流对象数量。
    pub streams_processed: usize,
    /// 是否提取到了文本。
    pub text_extracted: bool,
}
