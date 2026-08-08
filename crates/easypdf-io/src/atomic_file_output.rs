//! 同目录临时文件与原子替换输出。

use std::io::Write;
use std::path::{Path, PathBuf};

use easypdf_core::{PdfError, Result};

/// 将完整结果先写入同目录临时文件，再原子替换目标文件。
#[derive(Clone, Debug)]
pub struct AtomicFileOutput {
    target: PathBuf,
}

impl AtomicFileOutput {
    /// 创建原子文件输出目标。
    #[must_use]
    pub fn new(target: impl Into<PathBuf>) -> Self {
        Self {
            target: target.into(),
        }
    }

    /// 返回最终目标路径。
    #[must_use]
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// 原子写入完整字节内容。
    ///
    /// # Errors
    ///
    /// 创建目录、写入、同步或替换失败时返回错误。
    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        let parent = self.target.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".easypdf-")
            .tempfile_in(parent)?;
        temporary.write_all(bytes)?;
        temporary.as_file_mut().sync_all()?;
        temporary
            .persist(&self.target)
            .map_err(|error| PdfError::Io(error.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AtomicFileOutput;

    #[test]
    fn replaces_target_only_after_complete_write() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let target = directory.path().join("result.md");
        std::fs::write(&target, "old").expect("seed output");

        AtomicFileOutput::new(&target)
            .write(b"new")
            .expect("atomic output");

        assert_eq!(std::fs::read_to_string(target).expect("read output"), "new");
    }
}
