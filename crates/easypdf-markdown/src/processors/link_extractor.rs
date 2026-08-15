//! 链接提取处理器。

use easypdf_core::PdfInput;
use easypdf_core::Result;
use easypdf_core::{PdfBlock, PdfDocumentModel, SourceLocation};

use crate::{MarkdownProcessorCapabilities, MarkdownWarning, PdfMarkdownProcessor};

/// 链接提取处理器（Heuristic 级别）。
///
/// 扫描文档中的 [`PdfBlock::Paragraph`] 块，检测内嵌的 URL
/// 并将其转换为 [`PdfBlock::Link`] 块。
///
/// 当前实现识别以 `http://` 或 `https://` 开头的 URL。
///
/// TODO: 后续增强——从 PDF 注解（Annotation）中提取 URI，
/// 支持相对链接与邮件链接（`mailto:`）。
///
/// # Examples
///
/// ```
/// use easypdf_markdown::processors::LinkExtractorProcessor;
/// use easypdf_markdown::PdfMarkdownProcessor;
///
/// let proc = LinkExtractorProcessor;
/// assert!(proc.capabilities().link());
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct LinkExtractorProcessor;

impl PdfMarkdownProcessor for LinkExtractorProcessor {
    fn capabilities(&self) -> MarkdownProcessorCapabilities {
        MarkdownProcessorCapabilities::new().with_link()
    }

    fn process(
        &self,
        _input: &PdfInput,
        document: PdfDocumentModel,
    ) -> Result<(PdfDocumentModel, Vec<MarkdownWarning>)> {
        let mut new_pages = Vec::with_capacity(document.page_count());
        for page in document.pages() {
            let mut new_blocks = Vec::new();
            for block in page.blocks() {
                match block {
                    PdfBlock::Paragraph { text, source } => {
                        let links = extract_links(text, *source);
                        if links.is_empty() {
                            new_blocks.push(block.clone());
                        } else {
                            new_blocks.extend(links);
                        }
                    }
                    other => new_blocks.push(other.clone()),
                }
            }
            let mut new_page = easypdf_core::PdfPageModel::new(page.index());
            if let (Some(w), Some(h)) = (page.width_pt(), page.height_pt()) {
                new_page = new_page.with_dimensions(w, h);
            }
            new_page = new_page.with_rotation(page.rotation());
            for block in new_blocks {
                new_page = new_page.with_block(block);
            }
            new_pages.push(new_page);
        }
        Ok((
            PdfDocumentModel::new(document.metadata().clone(), new_pages),
            Vec::new(),
        ))
    }
}

/// 从文本中提取 URL，生成 `PdfBlock::Link` 或保留原始段落。
fn extract_links(text: &str, source: SourceLocation) -> Vec<PdfBlock> {
    let mut blocks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if let Some(pos) = find_url_start(remaining) {
            // 前面的文本作为段落。
            if pos > 0 {
                let before = remaining[..pos].trim();
                if !before.is_empty() {
                    blocks.push(PdfBlock::paragraph(before, source));
                }
            }
            // 提取 URL。
            let url_end = find_url_end(&remaining[pos..]);
            let url = &remaining[pos..pos + url_end];
            blocks.push(PdfBlock::link(url, url, source));
            remaining = &remaining[pos + url_end..];
        } else {
            // 没有更多 URL，剩余文本作为段落。
            let trimmed = remaining.trim();
            if !trimmed.is_empty() {
                blocks.push(PdfBlock::paragraph(trimmed, source));
            }
            break;
        }
    }

    blocks
}

/// 查找文本中第一个 URL 的起始位置。
fn find_url_start(text: &str) -> Option<usize> {
    // 查找 http:// 或 https://
    if let Some(pos) = text.find("https://") {
        return Some(pos);
    }
    text.find("http://")
}

/// 从 URL 起始位置查找 URL 的结束位置。
fn find_url_end(text: &str) -> usize {
    // URL 在空白字符、括号、引号处结束。
    text.chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, ')' | ']' | '>' | '"' | '\''))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypdf_core::PdfPageModel;
    use easypdf_core::{PageIndex, PdfMetadata};

    #[test]
    fn capabilities_include_link() {
        let proc = LinkExtractorProcessor;
        assert!(proc.capabilities().link());
    }

    #[test]
    fn extracts_url_from_paragraph() {
        let proc = LinkExtractorProcessor;
        let loc = SourceLocation::new(PageIndex::new(0), 1.0);
        let page = PdfPageModel::new(PageIndex::new(0)).with_block(PdfBlock::paragraph(
            "Visit https://example.com for more",
            loc,
        ));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, warnings) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        assert!(warnings.is_empty());
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        // Should have: paragraph "Visit", link "https://example.com", paragraph "for more"
        assert!(blocks.len() >= 2);
        // Check that a Link block exists
        let has_link = blocks
            .iter()
            .any(|(_, b)| matches!(b, PdfBlock::Link { .. }));
        assert!(has_link, "expected at least one Link block");
    }

    #[test]
    fn no_url_keeps_paragraph() {
        let proc = LinkExtractorProcessor;
        let loc = SourceLocation::new(PageIndex::new(0), 1.0);
        let page = PdfPageModel::new(PageIndex::new(0))
            .with_block(PdfBlock::paragraph("No links here", loc));
        let doc = PdfDocumentModel::new(PdfMetadata::default(), vec![page]);
        let (result, _) = proc.process(&PdfInput::from_bytes(vec![]), doc).unwrap();
        let blocks: Vec<_> = result.iter_all_blocks().collect();
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0].1, PdfBlock::Paragraph { .. }));
    }

    #[test]
    fn find_url_start_finds_https() {
        assert_eq!(find_url_start("go to https://x.com now"), Some(6));
    }

    #[test]
    fn find_url_start_finds_http() {
        assert_eq!(find_url_start("see http://x.com"), Some(4));
    }

    #[test]
    fn find_url_end_stops_at_space() {
        assert_eq!(find_url_end("https://x.com more"), 13);
    }

    #[test]
    fn find_url_end_stops_at_paren() {
        assert_eq!(find_url_end("https://x.com)"), 13);
    }
}
