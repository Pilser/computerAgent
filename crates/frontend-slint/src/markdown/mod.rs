//! Markdown message bodies: pulldown-cmark parse ([`blocks`]) + mermaid
//! SVG rendering ([`diagram`], cached by hash).

pub mod blocks;
pub mod diagram;

pub use blocks::{parse_markdown, Block};
pub use diagram::{render_cached, render_mermaid};
