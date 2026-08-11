//! Markdown 转换警告。

use easypdf_core::PageIndex;

/// 不阻止转换完成的结构化警告。
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownWarning {
    /// 页面没有可提取的原生文本。
    EmptyPage {
        /// 零基页索引。
        page_index: PageIndex,
    },
    /// 当前后端尚不能执行表格检测。
    TableDetectionUnavailable,
    /// 请求了图片提取，但当前读取后端尚不能提取图片资产。
    ImageExtractionUnavailable,
    /// 请求了 OCR，但没有启用 OCR 实现。
    OcrUnavailable {
        /// 零基页索引。
        page_index: PageIndex,
    },
    /// 管道中某个处理器执行失败（非 `fail_fast` 模式下收集）。
    ProcessorFailed {
        /// 错误描述。
        message: String,
    },
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;
    use easypdf_core::PageIndex;

    #[test]
    fn empty_page_variant() {
        let w = MarkdownWarning::EmptyPage {
            page_index: PageIndex::new(0),
        };
        let dbg = format!("{:?}", w);
        assert!(dbg.contains("EmptyPage"));
    }

    #[test]
    fn table_detection_unavailable_variant() {
        let w = MarkdownWarning::TableDetectionUnavailable;
        assert_eq!(format!("{:?}", w), "TableDetectionUnavailable");
    }

    #[test]
    fn image_extraction_unavailable_variant() {
        let w = MarkdownWarning::ImageExtractionUnavailable;
        assert_eq!(format!("{:?}", w), "ImageExtractionUnavailable");
    }

    #[test]
    fn ocr_unavailable_variant() {
        let w = MarkdownWarning::OcrUnavailable {
            page_index: PageIndex::new(2),
        };
        let dbg = format!("{:?}", w);
        assert!(dbg.contains("OcrUnavailable"));
    }

    #[test]
    fn processor_failed_variant() {
        let w = MarkdownWarning::ProcessorFailed {
            message: "test error".to_string(),
        };
        let dbg = format!("{:?}", w);
        assert!(dbg.contains("ProcessorFailed"));
        assert!(dbg.contains("test error"));
    }

    #[test]
    fn clone_preserves() {
        let w = MarkdownWarning::EmptyPage {
            page_index: PageIndex::new(1),
        };
        let cloned = w.clone();
        assert_eq!(w, cloned);
    }

    #[test]
    fn partial_eq_works() {
        let a = MarkdownWarning::TableDetectionUnavailable;
        let b = MarkdownWarning::TableDetectionUnavailable;
        let c = MarkdownWarning::ImageExtractionUnavailable;
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
