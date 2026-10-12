//! System prompt + prompt assembly.

pub mod assemble;
pub mod system;

pub use assemble::assemble;
pub use system::{system_prompt, tool_line, ROLE};
