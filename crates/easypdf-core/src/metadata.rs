//! PDF 文档的元数据类型。

/// PDF 文档级元数据。
///
/// 映射到 PDF 文件中的 `/Info` 字典。
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    /// 文档标题。
    pub title: Option<String>,
    /// 文档作者。
    pub author: Option<String>,
    /// 文档主题。
    pub subject: Option<String>,
    /// 与文档关联的关键词。
    pub keywords: Option<String>,
    /// 创建原始文档的应用程序。
    pub creator: Option<String>,
    /// 生成此 PDF 的应用程序（自动填充）。
    pub producer: Option<String>,
}

impl PdfMetadata {
    /// 创建所有字段为空的元数据。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置标题。
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 设置作者。
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// 设置主题。
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// 设置关键词（逗号分隔）。
    #[must_use]
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// 生成用于 PDF/A 兼容性的 XMP 元数据 XML 字符串。
    #[must_use]
    pub fn to_xmp(&self) -> String {
        let title = self.title.as_deref().unwrap_or("");
        let author = self.author.as_deref().unwrap_or("");
        let subject = self.subject.as_deref().unwrap_or("");
        let keywords = self.keywords.as_deref().unwrap_or("");
        format!(
            r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
   xmlns:dc="http://purl.org/dc/elements/1.1/">
   <dc:title><rdf:Alt><rdf:li xml:lang="x-default">{title}</rdf:li></rdf:Alt></dc:title>
   <dc:creator><rdf:Seq><rdf:li>{author}</rdf:li></rdf:Seq></dc:creator>
   <dc:description><rdf:Alt><rdf:li xml:lang="x-default">{subject}</rdf:li></rdf:Alt></dc:description>
   <dc:subject><rdf:Bag><rdf:li>{keywords}</rdf:li></rdf:Bag></dc:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
        )
    }
}

/// 单个书签/大纲条目。
#[derive(Debug, Clone)]
pub struct PdfBookmark {
    /// 显示标题。
    pub title: String,
    /// 目标页码（一基）。
    pub page: usize,
    /// 子书签（用于层级大纲）。
    pub children: Vec<PdfBookmark>,
}

impl PdfBookmark {
    /// 创建新的顶层书签。
    #[must_use]
    pub fn new(title: impl Into<String>, page: usize) -> Self {
        Self {
            title: title.into(),
            page,
            children: Vec::new(),
        }
    }

    /// 添加子书签。
    #[must_use]
    pub fn child(mut self, child: PdfBookmark) -> Self {
        self.children.push(child);
        self
    }
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn default_metadata() {
        let meta = PdfMetadata::default();
        assert!(meta.title.is_none());
        assert!(meta.author.is_none());
        assert!(meta.subject.is_none());
        assert!(meta.keywords.is_none());
        assert!(meta.creator.is_none());
        assert!(meta.producer.is_none());
    }

    #[test]
    fn new_is_default() {
        let meta = PdfMetadata::new();
        assert!(meta.title.is_none());
    }

    #[test]
    fn title_builder() {
        let meta = PdfMetadata::default().title("My Doc");
        assert_eq!(meta.title.as_deref(), Some("My Doc"));
    }

    #[test]
    fn author_builder() {
        let meta = PdfMetadata::default().author("John");
        assert_eq!(meta.author.as_deref(), Some("John"));
    }

    #[test]
    fn subject_builder() {
        let meta = PdfMetadata::default().subject("Test");
        assert_eq!(meta.subject.as_deref(), Some("Test"));
    }

    #[test]
    fn keywords_builder() {
        let meta = PdfMetadata::default().keywords("a,b,c");
        assert_eq!(meta.keywords.as_deref(), Some("a,b,c"));
    }

    #[test]
    fn to_xmp_includes_fields() {
        let meta = PdfMetadata::default()
            .title("Title")
            .author("Author")
            .subject("Subject")
            .keywords("kw");
        let xmp = meta.to_xmp();
        assert!(xmp.contains("Title"));
        assert!(xmp.contains("Author"));
        assert!(xmp.contains("Subject"));
        assert!(xmp.contains("kw"));
    }

    #[test]
    fn to_xmp_empty_fields() {
        let meta = PdfMetadata::default();
        let xmp = meta.to_xmp();
        assert!(xmp.contains("xmpmeta"));
    }

    #[test]
    fn debug_format() {
        let meta = PdfMetadata::default();
        let dbg = format!("{:?}", meta);
        assert!(dbg.contains("PdfMetadata"));
    }

    #[test]
    fn clone_preserves() {
        let meta = PdfMetadata::default().title("Test");
        let cloned = meta.clone();
        assert_eq!(meta.title, cloned.title);
    }

    #[test]
    fn bookmark_new() {
        let bm = PdfBookmark::new("Chapter 1", 1);
        assert_eq!(bm.title, "Chapter 1");
        assert_eq!(bm.page, 1);
        assert!(bm.children.is_empty());
    }

    #[test]
    fn bookmark_with_child() {
        let bm = PdfBookmark::new("Root", 1).child(PdfBookmark::new("Child", 2));
        assert_eq!(bm.children.len(), 1);
        assert_eq!(bm.children[0].title, "Child");
    }

    #[test]
    fn bookmark_nested_children() {
        let bm = PdfBookmark::new("Root", 1)
            .child(PdfBookmark::new("L1", 2).child(PdfBookmark::new("L2", 3)));
        assert_eq!(bm.children.len(), 1);
        assert_eq!(bm.children[0].children.len(), 1);
    }

    #[test]
    fn bookmark_debug() {
        let bm = PdfBookmark::new("Test", 1);
        let dbg = format!("{:?}", bm);
        assert!(dbg.contains("PdfBookmark"));
    }
}
