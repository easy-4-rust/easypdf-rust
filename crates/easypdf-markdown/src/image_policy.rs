//! PDF 图片转换策略。

use std::path::PathBuf;

/// PDF 图片转换策略。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImagePolicy {
    /// 忽略图片。
    #[default]
    Ignore,
    /// 保留已有图片引用。
    Reference,
    /// 将图片提取到指定目录。
    ExtractTo(PathBuf),
}
