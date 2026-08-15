//! Metadata types for PDF documents.

/// PDF document-level metadata.
///
/// Maps to the `/Info` dictionary in a PDF file.
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Keywords associated with the document.
    pub keywords: Option<String>,
    /// The application that created the original document.
    pub creator: Option<String>,
    /// The application that produced this PDF (filled automatically).
    pub producer: Option<String>,
}

impl PdfMetadata {
    /// Create new metadata with all fields empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the author.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set keywords (comma-separated).
    #[must_use]
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// Generate XMP metadata XML string for PDF/A compatibility.
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

/// A single bookmark / outline entry.
#[derive(Debug, Clone)]
pub struct PdfBookmark {
    /// The display title.
    pub title: String,
    /// Target page number (1-based).
    pub page: usize,
    /// Child bookmarks (for hierarchical outlines).
    pub children: Vec<PdfBookmark>,
}

impl PdfBookmark {
    /// Create a new top-level bookmark.
    #[must_use]
    pub fn new(title: impl Into<String>, page: usize) -> Self {
        Self {
            title: title.into(),
            page,
            children: Vec::new(),
        }
    }

    /// Add a child bookmark.
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
