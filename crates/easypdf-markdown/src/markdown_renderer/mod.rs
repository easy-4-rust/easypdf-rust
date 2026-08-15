//! 确定性 Markdown 渲染器。

mod escaping;
mod list;
mod renderer;
mod table;

pub use renderer::MarkdownRenderer;

#[cfg(test)]
#[allow(
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::single_char_pattern,
    clippy::unnecessary_wraps
)]
mod tests;
