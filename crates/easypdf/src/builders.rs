//! Builder types for PDF creation, reading, splitting, and manipulation.

use std::path::{Path, PathBuf};

use easypdf_core::{
    Orientation, PageSize, PdfFont, PdfImage, PdfMetadata, PdfTable, PdfText, PdfWriteHandler,
    Result, Rotation,
};
use easypdf_reader::{PdfManipulator, PdfReader, ReadStrategy};

// ======================================================================
// PdfCreateBuilder
// ======================================================================

/// Builder for creating new PDF documents.
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

    /// Set the document title.
    #[must_use = "builder method"]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the default page size.
    #[must_use = "builder method"]
    pub const fn page_size(mut self, size: PageSize) -> Self {
        self.page_size = size;
        self
    }

    /// Set the page orientation.
    #[must_use = "builder method"]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set document metadata.
    #[must_use = "builder method"]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Register a write handler.
    #[must_use = "builder method"]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Write text and finalize the document in one call.
    ///
    /// This is a convenience method for simple single-page PDFs.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be written.
    pub fn add_text(self, content: impl Into<String>) -> PdfTextBuilder<Self> {
        PdfTextBuilder {
            parent: self,
            text: PdfText::new(content),
        }
    }

    /// Add a table to the current page.
    ///
    /// Renders headers and data rows with grid lines. Column widths and row height
    /// are specified in PDF points.
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

    /// Add an image to the current page.
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

    /// Build the writer for manual page-by-page construction.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be initialized.
    pub fn build(self) -> Result<easypdf_writer::PdfWriter> {
        let mut writer = easypdf_writer::PdfWriter::new(&self.title);
        writer = writer.metadata(self.metadata);
        for handler in self.handlers {
            writer = writer.register_handler(handler);
        }
        Ok(writer)
    }

    /// Build, add a default page, write text, and save -- all in one call.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
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

/// Builder for adding text to a PDF, returned by [`PdfCreateBuilder::add_text`].
#[must_use]
pub struct PdfTextBuilder<P> {
    pub(crate) parent: P,
    pub(crate) text: PdfText,
}

impl PdfTextBuilder<PdfCreateBuilder> {
    /// Set the font for this text.
    #[must_use = "builder method"]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.text = self.text.font(font);
        self
    }

    /// Set the position as (x, y) in PDF points.
    #[must_use = "builder method"]
    pub fn position(self, x: f64, y: f64) -> PdfPositionedTextBuilder {
        PdfPositionedTextBuilder {
            parent: self.parent,
            text: self.text,
            x,
            y,
        }
    }

    /// Finalize by writing the text at the default position (100, 700).
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
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

/// Builder for table placement within a PDF, returned by [`PdfCreateBuilder::add_table`].
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
    /// Set the table position.
    #[must_use = "builder method"]
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Set column widths in PDF points.
    #[must_use = "builder method"]
    pub fn column_widths(mut self, widths: Vec<f64>) -> Self {
        self.col_widths = widths;
        self
    }

    /// Set row height in PDF points.
    #[must_use = "builder method"]
    pub fn row_height(mut self, height: f64) -> Self {
        self.row_height = height;
        self
    }

    /// Set the font for cell text.
    #[must_use = "builder method"]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.font = font;
        self
    }

    /// Finalize by writing the table and saving the PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
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

/// Builder for image placement within a PDF, returned by [`PdfCreateBuilder::add_image`].
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
    /// Set the image position in PDF points.
    #[must_use = "builder method"]
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Set the image dimensions in PDF points.
    #[must_use = "builder method"]
    pub fn size(mut self, w: f64, h: f64) -> Self {
        self.w = w;
        self.h = h;
        self
    }

    /// Finalize by writing the image and saving the PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or the image cannot be written.
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

/// Builder for text with an explicit position.
#[must_use]
pub struct PdfPositionedTextBuilder {
    parent: PdfCreateBuilder,
    text: PdfText,
    x: f64,
    y: f64,
}

impl PdfPositionedTextBuilder {
    /// Finalize and write the PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
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

/// Builder for reading/extracting content from PDFs.
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

    /// Limit extraction to a specific page range (0-based).
    #[must_use = "builder method"]
    pub fn pages(mut self, range: std::ops::Range<usize>) -> Self {
        self.pages = Some(range);
        self
    }

    /// Set the reading strategy (default: auto-selected by file size).
    #[must_use = "builder method"]
    pub const fn strategy(mut self, strategy: ReadStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Open a reusable, single-parse reader session.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or parsed.
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

    /// Extract all text from the PDF.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn extract_text(self) -> Result<String> {
        self.open()?.extract_text()
    }

    /// Extract PDF metadata.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn metadata(self) -> Result<PdfMetadata> {
        self.open()?.extract_metadata()
    }

    /// Get the number of pages.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn page_count(self) -> Result<usize> {
        self.open()?.page_count()
    }
}

// ======================================================================
// PdfSplitBuilder
// ======================================================================

/// Builder for splitting a PDF into individual pages.
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

    /// Set the number of pages per split file (default: 1).
    #[must_use = "builder method"]
    pub const fn every_n_pages(mut self, n: usize) -> Self {
        self.pages_per_file = n;
        self
    }

    /// Split the PDF and save pages to a directory.
    ///
    /// Files are named `page_001.pdf`, `page_002.pdf`, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or split files cannot be written.
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

/// Builder for PDF manipulation operations (rotate, reorder, watermark).
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

    /// Rotate a specific page (1-based index).
    #[must_use = "builder method"]
    pub fn rotate_page(mut self, page_number: usize, rotation: Rotation) -> Self {
        self.rotations.push((page_number, rotation));
        self
    }

    /// Rotate all pages.
    #[must_use = "builder method"]
    pub fn rotate_all(self, rotation: Rotation) -> Self {
        // This will be applied inside save() by iterating all pages
        self.rotate(rotation)
    }

    /// Rotate all pages (alias for builder chain).
    #[must_use = "builder method"]
    pub fn rotate(mut self, rotation: Rotation) -> Self {
        self.rotations.push((0, rotation)); // 0 means "all pages"
        self
    }

    /// Reorder pages according to the given permutation (0-based).
    #[must_use = "builder method"]
    pub fn reorder_pages(mut self, order: &[usize]) -> Self {
        self.order = Some(order.to_vec());
        self
    }

    /// Apply all manipulations and save to the output file.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or saved.
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut manipulator = PdfManipulator::open(&self.path)?;

        for (page_num, rotation) in &self.rotations {
            if *page_num == 0 {
                // Apply to all pages
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
