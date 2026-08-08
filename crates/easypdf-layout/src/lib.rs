//! 与 PDF 写入后端解耦的自动流式布局。

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]

mod direction;
mod flow_layout;
mod layout_sink;

pub use direction::Direction;
pub use flow_layout::FlowLayout;
pub use layout_sink::LayoutSink;
