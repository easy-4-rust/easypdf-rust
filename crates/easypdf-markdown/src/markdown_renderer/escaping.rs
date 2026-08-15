//! Markdown 文本转义与规范化辅助函数。

/// 规范化文本：逐行 trim 后用换行符连接。
pub(super) fn normalize_text(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n")
}

/// 转义 Markdown 特殊字符（`\`、`*`、`_`、`[`、`]`）。
pub(super) fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// 转义 URL 目标中的特殊字符（空格和括号）。
pub(super) fn escape_target(target: &str) -> String {
    target.replace(' ', "%20").replace(')', "%29")
}
