//! 文档会话管理。

use std::path::{Path, PathBuf};
use std::time::Instant;

use easypdf_core::Rotation;
use easypdf_reader::{PdfManipulator, PdfReader};
use tracing::{debug, info};

use super::error::{ResidentError, Result};
use super::protocol::{OpenMode, PageRange, PdfMetadataDto, SessionId};

/// 单个已打开的文档会话。
///
/// 包装 [`PdfReader`]（只读）或 [`PdfManipulator`]（读写），
/// 在会话生命周期内将解析后的 PDF 保持在内存中。
pub struct DocumentSession {
    /// 唯一会话标识符。
    pub id: SessionId,
    /// 此会话打开的文件路径。
    pub path: PathBuf,
    /// 打开模式。
    pub mode: OpenMode,
    /// 读取器（始终存在）。
    reader: Option<PdfReader>,
    /// 操作器（仅在 `ReadWrite` 模式下存在）。
    manipulator: Option<PdfManipulator>,
    /// 会话创建时间。
    pub opened_at: Instant,
    /// 最后访问此会话的时间。
    pub last_accessed: Instant,
    /// 文档是否有未保存的修改。
    pub dirty: bool,
    /// 自适应自动保存的保存耗时 EMA（秒）。`None` = 尚无采样。
    pub save_ema_secs: Option<f64>,
    /// 当前自动保存间隔（自适应模式）。
    pub autosave_interval: Option<std::time::Duration>,
}

impl DocumentSession {
    /// 打开一个新的文档会话。
    ///
    /// # Errors
    ///
    /// 如果文件无法打开或解析，返回 [`ResidentError::Pdf`]。
    pub fn open(id: SessionId, path: &Path, mode: OpenMode) -> Result<Self> {
        info!(session_id = id, path = %path.display(), ?mode, "opening document session");
        let now = Instant::now();
        let reader = Some(PdfReader::open(path)?);
        let manipulator = match mode {
            OpenMode::ReadWrite => Some(PdfManipulator::open(path)?),
            OpenMode::ReadOnly => None,
        };

        Ok(Self {
            id,
            path: path.to_path_buf(),
            mode,
            reader,
            manipulator,
            opened_at: now,
            last_accessed: now,
            dirty: false,
            save_ema_secs: None,
            autosave_interval: None,
        })
    }

    /// 从指定页面范围提取文本。
    ///
    /// # Errors
    ///
    /// 如果文本提取失败，返回 [`ResidentError::Pdf`]。
    pub fn extract_text(&mut self, pages: Option<&PageRange>) -> Result<String> {
        debug!(session_id = self.id, ?pages, "extracting text");
        self.touch();
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ResidentError::Protocol("no reader available".into()))?;

        if let Some(range) = pages {
            let count = reader.page_count()?;
            let end = range.end.unwrap_or(count).min(count);
            if range.start >= end {
                return Ok(String::new());
            }
            // 页面范围提取：可用时使用逐页提取，
            // 目前回退到全文提取。
            let _ = (range.start, end);
        }

        // 委托给全文提取（覆盖上述过滤后的所有页面）。
        Ok(reader.extract_text()?)
    }

    /// 获取总页数。
    ///
    /// # Errors
    ///
    /// 如果无法确定页数，返回 [`ResidentError::Pdf`]。
    pub fn page_count(&mut self) -> Result<usize> {
        debug!(session_id = self.id, "getting page count");
        self.touch();
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ResidentError::Protocol("no reader available".into()))?;
        Ok(reader.page_count()?)
    }

    /// 提取文档元数据。
    ///
    /// # Errors
    ///
    /// 如果元数据提取失败，返回 [`ResidentError::Pdf`]。
    pub fn extract_metadata(&mut self) -> Result<PdfMetadataDto> {
        self.touch();
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| ResidentError::Protocol("no reader available".into()))?;
        let meta = reader.extract_metadata()?;
        Ok(PdfMetadataDto {
            title: meta.title,
            author: meta.author,
            subject: meta.subject,
            keywords: meta.keywords,
            creator: meta.creator,
            producer: meta.producer,
        })
    }

    /// 旋转某个页面。
    ///
    /// # Errors
    ///
    /// - 如果页面无效，返回 [`ResidentError::Pdf`]。
    /// - 如果会话是只读的，返回 [`ResidentError::Server`]。
    pub fn rotate_page(&mut self, page: usize, degrees: u16) -> Result<()> {
        info!(session_id = self.id, page, degrees, "rotating page");
        self.touch();
        if self.mode == OpenMode::ReadOnly {
            return Err(ResidentError::Server {
                code: "READ_ONLY".into(),
                message: "cannot modify a read-only session".into(),
            });
        }
        let rotation = match degrees {
            0 => Rotation::None,
            90 => Rotation::Clockwise90,
            180 => Rotation::Clockwise180,
            270 => Rotation::Clockwise270,
            _ => {
                return Err(ResidentError::Server {
                    code: "INVALID_ROTATION".into(),
                    message: format!("rotation must be 0, 90, 180, or 270, got {degrees}"),
                });
            }
        };
        let manip = self
            .manipulator
            .as_mut()
            .ok_or_else(|| ResidentError::Protocol("no manipulator available".into()))?;
        manip.rotate_page(page, rotation)?;
        self.dirty = true;
        Ok(())
    }

    /// 保存文档。
    ///
    /// # Errors
    ///
    /// - 如果保存失败，返回 [`ResidentError::Pdf`]。
    /// - 如果会话是只读的，返回 [`ResidentError::Server`]。
    pub fn save(&mut self, path: Option<&Path>) -> Result<PathBuf> {
        info!(session_id = self.id, ?path, "saving document");
        self.touch();
        if self.mode == OpenMode::ReadOnly {
            return Err(ResidentError::Server {
                code: "READ_ONLY".into(),
                message: "cannot save a read-only session".into(),
            });
        }
        let manip = self
            .manipulator
            .take()
            .ok_or_else(|| ResidentError::Protocol("no manipulator available".into()))?;

        let save_path = path.unwrap_or(&self.path);
        let save_path_buf = save_path.to_path_buf();
        let start = Instant::now();
        manip.save(save_path)?;
        let elapsed = start.elapsed();
        self.dirty = false;

        // 记录保存耗时用于自适应自动保存
        self.record_save_duration(elapsed);

        // 重新打开操作器以保持会话存活
        self.manipulator = Some(PdfManipulator::open(&self.path)?);
        // 同时重新打开读取器以反映已保存的状态
        self.reader = Some(PdfReader::open(&self.path)?);

        Ok(save_path_buf)
    }

    /// 文档是否有未保存的更改。
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 更新最后访问时间戳。
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// 记录保存耗时用于自适应自动保存 EMA。
    pub fn record_save_duration(&mut self, elapsed: std::time::Duration) {
        const ALPHA: f64 = 0.3;
        const MULTIPLIER: f64 = 4.0;
        const MIN_SECS: f64 = 10.0;
        const MAX_SECS: f64 = 300.0;

        let sample = elapsed.as_secs_f64();
        let ema = match self.save_ema_secs {
            Some(prev) => ALPHA * sample + (1.0 - ALPHA) * prev,
            None => sample,
        };
        self.save_ema_secs = Some(ema);

        let interval_secs = (MULTIPLIER * ema).clamp(MIN_SECS, MAX_SECS);
        self.autosave_interval = Some(std::time::Duration::from_secs_f64(interval_secs));
    }
}
