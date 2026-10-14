//! Markdown bridge: fence splitter + egui renderers + diagram cache.
//!
//! Re-export only; see [`blocks`], [`render`], [`diagram`].

pub mod blocks;
pub mod diagram;
pub mod render;

pub use blocks::{parse_markdown, Block};
pub use diagram::{image_uri, render_cached, render_mermaid};
pub use render::{show_block, show_message};
