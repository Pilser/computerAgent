//! LLM-callable agent tools (model -> system).
//!
//! Slimmed port of vendor `agent/tools/` (`read`, `write`, `bash` only in
//! v1). Each tool gates on [`computer_agent_policy::PermissionChecker`]
//! BEFORE touching [`computer_agent_sys_tools`]. No `rig` dependency in v1:
//! the [`tools::Tool`] trait here is ours.
//!
//! Deferred: `edit`, `grep`, `find_files`, `list_dir`, `todo` (later phases)
//! and `memory_tools` (phase 4 bridge comes with engine wiring later).

pub mod error;
pub mod prompt;
pub mod provider;
pub mod runner;
pub mod tools;

pub use error::{AgentError, ToolError};
