//! Runtime layer for easypdf: MCP server + resident daemon.

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::uninlined_format_args, clippy::manual_string_new)]

#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "resident")]
pub mod resident;
