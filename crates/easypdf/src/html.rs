//! HTML 与 Markdown 输入支持。

#[cfg(feature = "html")]
use std::path::Path;

#[cfg(feature = "html")]
use easypdf_core::PdfError;

/// Convert basic Markdown to HTML.
pub(crate) fn markdown_to_html(md: &str) -> String {
    let mut html = String::from("<html><body>\n");
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            html.push_str("<br/>\n");
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            use std::fmt::Write;
            let _ = write!(html, "<h3>{rest}</h3>\n");
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            use std::fmt::Write;
            let _ = write!(html, "<h2>{rest}</h2>\n");
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            use std::fmt::Write;
            let _ = write!(html, "<h1>{rest}</h1>\n");
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            use std::fmt::Write;
            let _ = write!(html, "<li>{rest}</li>\n");
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            use std::fmt::Write;
            let _ = write!(html, "<blockquote>{rest}</blockquote>\n");
        } else {
            let processed = process_inline_formatting(trimmed);
            use std::fmt::Write;
            let _ = write!(html, "<p>{processed}</p>\n");
        }
    }
    html.push_str("</body></html>");
    html
}

/// Process inline **bold** and *italic* Markdown.
fn process_inline_formatting(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            result.push_str("<b>");
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str("</b>");
            if i + 1 < chars.len() {
                i += 2;
            }
        } else if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] != '*' {
            result.push_str("<i>");
            i += 1;
            while i < chars.len() && chars[i] != '*' {
                result.push(chars[i]);
                i += 1;
            }
            result.push_str("</i>");
            if i < chars.len() {
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Builder for HTML-to-PDF conversion (requires `html` feature).
#[cfg(feature = "html")]
#[must_use]
pub struct HtmlToPdfBuilder {
    html: String,
    title: String,
}

#[cfg(feature = "html")]
impl HtmlToPdfBuilder {
    pub(crate) fn new(html: &str) -> Self {
        Self {
            html: html.to_string(),
            title: "HTML Document".into(),
        }
    }

    /// Set the document title.
    #[must_use = "builder method"]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Render HTML to PDF and save.
    ///
    /// # Errors
    ///
    /// Returns an error if Chromium is not available or rendering fails.
    pub fn save(self, output: impl AsRef<Path>) -> crate::Result<()> {
        use std::collections::BTreeMap;
        let mut warnings = Vec::new();
        let images = BTreeMap::new();
        let fonts = BTreeMap::new();
        let options = printpdf::GeneratePdfOptions::default();
        let doc =
            printpdf::PdfDocument::from_html(&self.html, &images, &fonts, &options, &mut warnings)
                .map_err(|e| PdfError::Other(format!("HTML render error: {e}")))?;
        let file = std::fs::File::create(output)?;
        let mut buf = std::io::BufWriter::new(file);
        let save_opts = printpdf::PdfSaveOptions::default();
        doc.save_writer(&mut buf, &save_opts, &mut warnings);
        Ok(())
    }
}

// ======================================================================
