//! PDF 表单填充 Builder。

use crate::Result;
use std::path::{Path, PathBuf};

/// 用于填充 PDF 表单模板的 Builder。
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
        let _ = data; // PdfModel trait 将用于提取字段映射
        Self {
            template_path: template_path.into(),
            fields: Vec::new(),
        }
    }

    /// 添加一个要填充的字段值。
    ///
    /// # 参数
    ///
    /// * `name` - 表单字段名称。
    /// * `value` - 要填入的值。
    #[must_use = "builder method"]
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((name.into(), value.into()));
        self
    }

    /// 添加多个字段值。
    ///
    /// # 参数
    ///
    /// * `fields` - 字段名-值对的迭代器。
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

    /// 填充表单字段并保存到输出文件。
    ///
    /// # Errors
    ///
    /// 如果模板无法读取或输出无法写入，返回错误。
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut filler = easypdf_writer::PdfTemplateFiller::open(&self.template_path)?;
        for (name, value) in &self.fields {
            filler.fill_field(name, value)?;
        }
        filler.save(output)
    }
}
