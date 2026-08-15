//! PDF 创建、读取、拆分和操作的 Builder 类型。

use std::path::{Path, PathBuf};

use easypdf_core::{
    Orientation, PageSize, PdfFont, PdfImage, PdfMetadata, PdfTable, PdfText, PdfWriteHandler,
    Result, Rotation,
};
use easypdf_reader::{PdfManipulator, PdfReader, ReadStrategy};

// ======================================================================
// PdfCreateBuilder
// ======================================================================

/// 用于创建新 PDF 文档的 Builder。
#[must_use]
pub struct PdfCreateBuilder {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    pub(crate) page_size: PageSize,
    pub(crate) orientation: Orientation,
    pub(crate) metadata: PdfMetadata,
    #[allow(dead_code)]
    pub(crate) fonts: Vec<PdfFont>,
    pub(crate) handlers: Vec<Box<dyn PdfWriteHandler>>,
}

impl PdfCreateBuilder {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            title: String::from("Untitled"),
            page_size: PageSize::A4,
            orientation: Orientation::default(),
            metadata: PdfMetadata::default(),
            fonts: Vec::new(),
            handlers: Vec::new(),
        }
    }

    /// 设置文档标题。
    #[must_use = "builder method"]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 设置默认页面大小。
    #[must_use = "builder method"]
    pub const fn page_size(mut self, size: PageSize) -> Self {
        self.page_size = size;
        self
    }

    /// 设置页面方向。
    #[must_use = "builder method"]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// 设置文档元数据。
    #[must_use = "builder method"]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// 注册一个写入处理器。
    #[must_use = "builder method"]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// 写入文本并一步完成文档创建。
    ///
    /// 这是简单单页 PDF 的便捷方法。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法写入，返回错误。
    pub fn add_text(self, content: impl Into<String>) -> PdfTextBuilder<Self> {
        PdfTextBuilder {
            parent: self,
            text: PdfText::new(content),
        }
    }

    /// 向当前页面添加表格。
    ///
    /// 使用网格线渲染表头和数据行。列宽和行高以 PDF point 为单位。
    #[must_use = "builder method"]
    pub fn add_table(self, table: &PdfTable) -> PdfTableBuilder {
        PdfTableBuilder {
            parent: self,
            table: table.clone(),
            x: 72.0,
            y: 700.0,
            col_widths: Vec::new(),
            row_height: 20.0,
            font: PdfFont::helvetica(10.0),
        }
    }

    /// 向当前页面添加图片。
    #[must_use = "builder method"]
    pub fn add_image(self, image: &PdfImage) -> PdfImageBuilder {
        PdfImageBuilder {
            parent: self,
            image: image.clone(),
            x: 72.0,
            y: 700.0,
            w: 0.0,
            h: 0.0,
        }
    }

    /// 构建写入器以进行手动逐页构建。
    ///
    /// # Errors
    ///
    /// 如果写入器无法初始化，返回错误。
    pub fn build(self) -> Result<easypdf_writer::PdfWriter> {
        let mut writer = easypdf_writer::PdfWriter::new(&self.title);
        writer = writer.metadata(self.metadata);
        for handler in self.handlers {
            writer = writer.register_handler(handler);
        }
        Ok(writer)
    }

    /// 构建、添加默认页面、写入文本并保存 -- 一步完成。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法创建或写入，返回错误。
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.path.clone();
        let page_size = self.page_size;
        let orientation = self.orientation;
        let mut writer = self.build()?;
        writer.add_page(page_size, orientation)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// ======================================================================
// PdfTextBuilder
// ======================================================================

/// 用于向 PDF 添加文本的 Builder，由 [`PdfCreateBuilder::add_text`] 返回。
#[must_use]
pub struct PdfTextBuilder<P> {
    pub(crate) parent: P,
    pub(crate) text: PdfText,
}

impl PdfTextBuilder<PdfCreateBuilder> {
    /// 设置此文本的字体。
    #[must_use = "builder method"]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.text = self.text.font(font);
        self
    }

    /// 设置位置为 (x, y)，单位为 PDF point。
    #[must_use = "builder method"]
    pub fn position(self, x: f64, y: f64) -> PdfPositionedTextBuilder {
        PdfPositionedTextBuilder {
            parent: self.parent,
            text: self.text,
            x,
            y,
        }
    }

    /// 以默认位置 (100, 700) 写入文本来完成操作。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法创建或写入，返回错误。
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_text(&self.text, 100.0, 700.0)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// ======================================================================
// PdfTableBuilder
// ======================================================================

/// 用于在 PDF 中放置表格的 Builder，由 [`PdfCreateBuilder::add_table`] 返回。
#[must_use]
pub struct PdfTableBuilder {
    parent: PdfCreateBuilder,
    table: PdfTable,
    x: f64,
    y: f64,
    col_widths: Vec<f64>,
    row_height: f64,
    font: PdfFont,
}

impl PdfTableBuilder {
    /// 设置表格位置。
    #[must_use = "builder method"]
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// 设置列宽，单位为 PDF point。
    #[must_use = "builder method"]
    pub fn column_widths(mut self, widths: Vec<f64>) -> Self {
        self.col_widths = widths;
        self
    }

    /// 设置行高，单位为 PDF point。
    #[must_use = "builder method"]
    pub fn row_height(mut self, height: f64) -> Self {
        self.row_height = height;
        self
    }

    /// 设置单元格文本的字体。
    #[must_use = "builder method"]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.font = font;
        self
    }

    /// 写入表格并保存 PDF 以完成操作。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法创建或写入，返回错误。
    pub fn do_write(self) -> easypdf_core::Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        crate::write_table(
            &mut writer,
            &self.table,
            self.x,
            self.y,
            &self.col_widths,
            self.row_height,
            &self.font,
        )?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// ======================================================================
// PdfImageBuilder
// ======================================================================

/// 用于在 PDF 中放置图片的 Builder，由 [`PdfCreateBuilder::add_image`] 返回。
#[must_use]
pub struct PdfImageBuilder {
    parent: PdfCreateBuilder,
    image: PdfImage,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl PdfImageBuilder {
    /// 设置图片位置，单位为 PDF point。
    #[must_use = "builder method"]
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// 设置图片尺寸，单位为 PDF point。
    #[must_use = "builder method"]
    pub fn size(mut self, w: f64, h: f64) -> Self {
        self.w = w;
        self.h = h;
        self
    }

    /// 写入图片并保存 PDF 以完成操作。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法创建或图片无法写入，返回错误。
    pub fn do_write(self) -> easypdf_core::Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_image(&self.image, self.x, self.y, self.w, self.h)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// ======================================================================
// PdfPositionedTextBuilder
// ======================================================================

/// 带显式位置的文本 Builder。
#[must_use]
pub struct PdfPositionedTextBuilder {
    parent: PdfCreateBuilder,
    text: PdfText,
    x: f64,
    y: f64,
}

impl PdfPositionedTextBuilder {
    /// 完成操作并写入 PDF。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法创建或写入，返回错误。
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_text(&self.text, self.x, self.y)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// ======================================================================
// PdfReadBuilder
// ======================================================================

/// 用于从 PDF 中读取/提取内容的 Builder。
#[must_use]
pub struct PdfReadBuilder {
    path: PathBuf,
    pages: Option<std::ops::Range<usize>>,
    strategy: Option<ReadStrategy>,
}

impl PdfReadBuilder {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages: None,
            strategy: None,
        }
    }

    /// 限制提取到指定的页面范围（0 起始）。
    #[must_use = "builder method"]
    pub fn pages(mut self, range: std::ops::Range<usize>) -> Self {
        self.pages = Some(range);
        self
    }

    /// 设置读取策略（默认：按文件大小自动选择）。
    #[must_use = "builder method"]
    pub const fn strategy(mut self, strategy: ReadStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// 打开一个可复用的、单次解析的读取器会话。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法读取或解析，返回错误。
    pub fn open(self) -> Result<PdfReader> {
        let mut reader = match self.strategy {
            Some(strategy) => PdfReader::open_with_strategy(&self.path, strategy)?,
            None => PdfReader::open(&self.path)?,
        };
        if let Some(range) = self.pages {
            reader = reader.try_pages(range)?;
        }
        Ok(reader)
    }

    /// 提取 PDF 中的所有文本。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法读取，返回 `PdfError::Parse`。
    pub fn extract_text(self) -> Result<String> {
        self.open()?.extract_text()
    }

    /// 提取 PDF 元数据。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法读取，返回 `PdfError::Parse`。
    pub fn metadata(self) -> Result<PdfMetadata> {
        self.open()?.extract_metadata()
    }

    /// 获取页数。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法读取，返回 `PdfError::Parse`。
    pub fn page_count(self) -> Result<usize> {
        self.open()?.page_count()
    }
}

// ======================================================================
// PdfSplitBuilder
// ======================================================================

/// 用于将 PDF 拆分为单页文件的 Builder。
#[must_use]
pub struct PdfSplitBuilder {
    path: PathBuf,
    pages_per_file: usize,
}

impl PdfSplitBuilder {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages_per_file: 1,
        }
    }

    /// 设置每个拆分文件的页数（默认：1）。
    #[must_use = "builder method"]
    pub const fn every_n_pages(mut self, n: usize) -> Self {
        self.pages_per_file = n;
        self
    }

    /// 将 PDF 拆分并保存页面到目录。
    ///
    /// 文件命名为 `page_001.pdf`、`page_002.pdf` 等。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法读取或拆分文件无法写入，返回错误。
    pub fn save_to_dir(self, output_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let manipulator = PdfManipulator::open(&self.path)?;
        let total_pages = manipulator.page_count();
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;

        let mut output_paths = Vec::new();
        let mut start = 0;

        while start < total_pages {
            let end = std::cmp::min(start + self.pages_per_file, total_pages);
            let mut chunk = manipulator.extract_pages(start..end)?;
            let filename = format!("page_{:03}.pdf", start / self.pages_per_file + 1);
            let output_path = output_dir.join(&filename);
            chunk.save(&output_path)?;
            output_paths.push(output_path);
            start = end;
        }

        Ok(output_paths)
    }
}

// ======================================================================
// PdfManipulateBuilder
// ======================================================================

/// 用于 PDF 操作（旋转、重排、水印）的 Builder。
#[must_use]
pub struct PdfManipulateBuilder {
    path: PathBuf,
    rotations: Vec<(usize, Rotation)>,
    order: Option<Vec<usize>>,
}

impl PdfManipulateBuilder {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            rotations: Vec::new(),
            order: None,
        }
    }

    /// 旋转指定页面（1 起始索引）。
    #[must_use = "builder method"]
    pub fn rotate_page(mut self, page_number: usize, rotation: Rotation) -> Self {
        self.rotations.push((page_number, rotation));
        self
    }

    /// 旋转所有页面。
    #[must_use = "builder method"]
    pub fn rotate_all(self, rotation: Rotation) -> Self {
        self.rotate(rotation)
    }

    /// 旋转所有页面（builder 链别名）。
    #[must_use = "builder method"]
    pub fn rotate(mut self, rotation: Rotation) -> Self {
        self.rotations.push((0, rotation)); // 0 表示"所有页面"
        self
    }

    /// 按给定排列重排页面（0 起始）。
    #[must_use = "builder method"]
    pub fn reorder_pages(mut self, order: &[usize]) -> Self {
        self.order = Some(order.to_vec());
        self
    }

    /// 应用所有操作并保存到输出文件。
    ///
    /// # Errors
    ///
    /// 如果 PDF 无法读取或保存，返回错误。
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut manipulator = PdfManipulator::open(&self.path)?;

        for (page_num, rotation) in &self.rotations {
            if *page_num == 0 {
                // 应用到所有页面
                let count = manipulator.page_count();
                for p in 1..=count {
                    manipulator.rotate_page(p, *rotation)?;
                }
            } else {
                manipulator.rotate_page(*page_num, *rotation)?;
            }
        }

        if let Some(order) = &self.order {
            manipulator.reorder_pages(order)?;
        }

        manipulator.save(output)
    }
}
