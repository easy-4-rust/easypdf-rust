//! PDF 表单填充 Builder。

use crate::Result;
use std::path::{Path, PathBuf};

/// Builder for filling PDF form templates.
#[must_use]
pub struct PdfFillBuilder {
    template_path: PathBuf,
    fields: Vec<(String, String)>,
}

impl PdfFillBuilder {
    pub(crate) fn new(
        template_path: impl Into<PathBuf>,
        data: &dyn easypdf_core::PdfModel,
    ) -> Self {
        let _ = data; // The PdfModel trait will be used to extract field mappings
        Self {
            template_path: template_path.into(),
            fields: Vec::new(),
        }
    }

    /// Add a field value to fill.
    #[must_use = "builder method"]
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((name.into(), value.into()));
        self
    }

    /// Add multiple field values.
    #[must_use = "builder method"]
    pub fn fields(
        mut self,
        fields: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (name, value) in fields {
            self.fields.push((name.into(), value.into()));
        }
        self
    }

    /// Fill form fields and save to the output file.
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be read or the output cannot be written.
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut filler = easypdf_writer::PdfTemplateFiller::open(&self.template_path)?;
        for (name, value) in &self.fields {
            filler.fill_field(name, value)?;
        }
        filler.save(output)
    }
}
