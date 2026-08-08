//! Markdown 输出配置档。

/// Markdown 输出配置档。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownProfile {
    /// 标准 GitHub Flavored Markdown。
    #[default]
    Gfm,
    /// 面向大模型分块与引用的输出，显式保留页标题。
    Llm,
    /// 仅保留可读文本，不生成表格等 Markdown 结构。
    Plain,
}
