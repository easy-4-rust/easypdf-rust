//! 写入后端选择与页面级溢出机制。
//!
//! 提供 [`WriteBackend`] 用于在内存模式和溢出到磁盘模式之间选择，
//! 以及 [`PageSpillWriter`] 用于将已完成的页面内容序列化到临时文件
//! 以限制峰值内存使用。
//!
//! # 设计
//!
//! 溢出机制以页面粒度运行：页面完成后，其操作（`Vec<WriterOp>`）
//! 和尺寸被序列化到临时文件（可选 gzip 压缩）。在 `finish()` 时，
//! 所有溢出的页面被读回并合并到最终 PDF 文档中。
//!
//! 这借鉴了 `easyexcel-rust` 的 SXSSF 溢出模式，适配了 PDF 的
//! 页面级（而非行级）内容单元。

use easypdf_core::error::{PdfError, Result};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write as _};
use std::path::PathBuf;

use crate::engine::WriterOp;

/// 单个页面内容的序列化表示，存储在溢出文件中。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SpilledPageData {
    /// 页码（从 1 开始）。
    pub page_number: usize,
    /// 页面宽度（PDF 点）。
    pub width_pt: f64,
    /// 页面高度（PDF 点）。
    pub height_pt: f64,
    /// 该页面的操作列表（引擎无关的中间表示）。
    pub ops: Vec<WriterOp>,
}

/// PDF 写入后端选择。
///
/// 控制写入器是将所有页面保留在内存中，还是将已完成的页面溢出到
/// 临时文件以限制峰值内存使用。
///
/// # Examples
///
/// ```
/// use easypdf_writer::WriteBackend;
///
/// // 根据预期页数自动选择。
/// let backend = WriteBackend::auto(50);
/// assert!(!backend.is_constant_memory());
///
/// let backend = WriteBackend::auto(200);
/// assert!(backend.is_constant_memory());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum WriteBackend {
    /// 全量内存模式（小型文档默认）。
    ///
    /// 整个 PDF 文档通过 `printpdf` 在内存中构建。
    /// 适合约 100 页以内的文档。
    #[default]
    InMemory,

    /// 页面级溢出模式，适用于大型文档。
    ///
    /// 每个已完成的页面被序列化到临时文件并从内存中释放。在 `finish()` 时，
    /// 所有溢出的页面被读回并合并。这将峰值内存限制为大约一个页面的内容
    /// 加上最终 PDF 输出缓冲区。
    Spill {
        /// Directory for spill files. `None` uses the system temporary directory.
        spill_dir: Option<PathBuf>,
        /// Whether to gzip-compress spill files (reduces disk I/O at the cost
        /// of CPU, mirroring `compress_temp_files` from easyexcel-rust).
        compress: bool,
        /// Page count threshold: spill activates only after this many pages
        /// have been finalized. Pages below the threshold stay in memory.
        threshold_pages: usize,
    },
}

impl WriteBackend {
    /// 根据估计的页数自动选择后端。
    ///
    /// 100 页及以下返回 [`InMemory`](Self::InMemory)。
    /// 超过 100 页返回启用压缩且阈值为 50 页的 [`Spill`](Self::Spill)。
    #[must_use]
    pub fn auto(estimated_pages: usize) -> Self {
        match estimated_pages {
            0..=100 => Self::InMemory,
            _ => Self::Spill {
                spill_dir: None,
                compress: true,
                threshold_pages: 50,
            },
        }
    }

    /// 当此后端使用常量内存溢出策略时返回 `true`。
    #[must_use]
    pub const fn is_constant_memory(&self) -> bool {
        matches!(self, Self::Spill { .. })
    }

    /// 创建具有常量内存语义的溢出后端（阈值 = 1）。
    ///
    /// 这是一个便捷构造函数，每个页面在完成后立即溢出。
    #[must_use]
    pub fn constant_memory() -> Self {
        Self::Spill {
            spill_dir: None,
            compress: true,
            threshold_pages: 1,
        }
    }
}

/// 页面级溢出写入器。
///
/// 管理已完成页面内容到临时文件的序列化以及完成时的反序列化。
/// 溢出文件存储在临时目录中，当 `PageSpillWriter` 被 drop 时
/// 自动清理（通过 [`tempfile::TempDir`]）。
pub(crate) struct PageSpillWriter {
    /// Temporary directory guard (cleaned up on drop).
    _temp_dir: Option<tempfile::TempDir>,
    /// Directory for spill files (either user-provided or from `_temp_dir`).
    spill_dir: PathBuf,
    /// Whether to gzip-compress spill files.
    compress: bool,
    /// Threshold: only spill when finalized page count exceeds this value.
    threshold_pages: usize,
    /// Paths of spilled page files, keyed by page number (1-based).
    spilled_pages: BTreeMap<usize, PathBuf>,
    /// Count of pages finalized so far (including both in-memory and spilled).
    finalized_count: usize,
}

impl PageSpillWriter {
    /// 创建新的溢出写入器。
    ///
    /// 如果 `spill_dir` 为 `None`，会自动创建唯一的临时目录
    /// （drop 时清理）。
    ///
    /// # Errors
    ///
    /// 当无法创建溢出目录时返回错误。
    pub fn new(spill_dir: Option<PathBuf>, compress: bool, threshold_pages: usize) -> Result<Self> {
        let (dir, temp_dir_guard) = if let Some(d) = spill_dir {
            std::fs::create_dir_all(&d)?;
            (d, None)
        } else {
            let td = tempfile::tempdir().map_err(PdfError::Io)?;
            let path = td.path().to_path_buf();
            (path, Some(td))
        };
        Ok(Self {
            _temp_dir: temp_dir_guard,
            spill_dir: dir,
            compress,
            threshold_pages,
            spilled_pages: BTreeMap::new(),
            finalized_count: 0,
        })
    }

    /// 尝试溢出已完成页面的数据。
    ///
    /// 如果已完成的页面数尚未超过阈值，此操作为空操作，
    /// 调用方应将页面数据保留在内存中。
    ///
    /// # Errors
    ///
    /// 当序列化或文件 I/O 失败时返回错误。
    pub fn maybe_spill(&mut self, page_data: &SpilledPageData) -> Result<Option<()>> {
        self.finalized_count += 1;
        if self.finalized_count <= self.threshold_pages {
            return Ok(None);
        }

        let file_name = format!(
            "page-{:06}.json{}",
            page_data.page_number,
            if self.compress { ".gz" } else { "" }
        );
        let file_path = self.spill_dir.join(&file_name);

        let serialized = serde_json::to_vec(page_data)
            .map_err(|e| PdfError::Other(format!("Spill serialization failed: {e}")))?;

        if self.compress {
            let file = File::create(&file_path)?;
            let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::fast());
            encoder.write_all(&serialized)?;
            encoder.try_finish()?;
        } else {
            std::fs::write(&file_path, &serialized)?;
        }

        self.spilled_pages.insert(page_data.page_number, file_path);
        Ok(Some(()))
    }

    /// 收集所有溢出的页面，按页码顺序返回。
    ///
    /// # Errors
    ///
    /// 当任何溢出文件无法读取或反序列化时返回错误。
    pub fn collect_all(&self) -> Result<Vec<SpilledPageData>> {
        let mut pages = Vec::with_capacity(self.spilled_pages.len());
        for path in self.spilled_pages.values() {
            let data = if self.compress {
                let file = File::open(path)?;
                let mut decoder = GzDecoder::new(BufReader::new(file));
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                buf
            } else {
                std::fs::read(path)?
            };
            let page: SpilledPageData = serde_json::from_slice(&data)
                .map_err(|e| PdfError::Other(format!("Spill deserialization failed: {e}")))?;
            pages.push(page);
        }
        Ok(pages)
    }

    /// 返回已溢出到磁盘的页面数量。
    #[must_use]
    pub fn spilled_count(&self) -> usize {
        self.spilled_pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_backend_auto_small() {
        let b = WriteBackend::auto(10);
        assert_eq!(b, WriteBackend::InMemory);
        assert!(!b.is_constant_memory());
    }

    #[test]
    fn write_backend_auto_large() {
        let b = WriteBackend::auto(200);
        assert!(b.is_constant_memory());
        assert!(matches!(
            b,
            WriteBackend::Spill {
                compress: true,
                threshold_pages: 50,
                ..
            }
        ));
    }

    #[test]
    fn write_backend_constant_memory() {
        let b = WriteBackend::constant_memory();
        assert!(b.is_constant_memory());
        if let WriteBackend::Spill {
            threshold_pages, ..
        } = b
        {
            assert_eq!(threshold_pages, 1);
        } else {
            panic!("expected Spill");
        }
    }

    #[test]
    fn write_backend_default_is_in_memory() {
        assert_eq!(WriteBackend::default(), WriteBackend::InMemory);
    }

    #[test]
    fn spill_writer_below_threshold_is_noop() {
        let mut sw = PageSpillWriter::new(None, false, 5).unwrap();
        let data = SpilledPageData {
            page_number: 1,
            width_pt: 595.0,
            height_pt: 842.0,
            ops: vec![],
        };
        assert!(sw.maybe_spill(&data).unwrap().is_none());
        assert_eq!(sw.spilled_count(), 0);
    }

    #[test]
    fn spill_writer_above_threshold_writes_file() {
        let mut sw = PageSpillWriter::new(None, false, 2).unwrap();
        for i in 1..=3 {
            let data = SpilledPageData {
                page_number: i,
                width_pt: 595.0,
                height_pt: 842.0,
                ops: vec![],
            };
            sw.maybe_spill(&data).unwrap();
        }
        // Pages 3 should be spilled (1 and 2 are at or below threshold).
        assert_eq!(sw.spilled_count(), 1);
    }

    #[test]
    fn spill_writer_roundtrip_with_compression() {
        let mut sw = PageSpillWriter::new(None, true, 0).unwrap();
        for i in 1..=3 {
            let data = SpilledPageData {
                page_number: i,
                width_pt: 595.0 * i as f64,
                height_pt: 842.0,
                ops: vec![],
            };
            sw.maybe_spill(&data).unwrap();
        }
        let collected = sw.collect_all().unwrap();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].page_number, 1);
        assert_eq!(collected[2].page_number, 3);
    }

    #[test]
    fn spill_writer_cleans_up_on_drop() {
        let spill_path;
        {
            let mut sw = PageSpillWriter::new(None, false, 0).unwrap();
            spill_path = sw.spill_dir.clone();
            let data = SpilledPageData {
                page_number: 1,
                width_pt: 595.0,
                height_pt: 842.0,
                ops: vec![],
            };
            sw.maybe_spill(&data).unwrap();
            assert_eq!(sw.spilled_count(), 1);
        }
        // After drop, the temp directory should be cleaned up.
        assert!(
            !spill_path.exists(),
            "spill directory should be cleaned up after drop"
        );
    }
}
