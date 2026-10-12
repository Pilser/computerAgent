//! `command-api`: the single human->system contract.
//!
//! Both frontends (`frontend-slint`, `frontend-tauri`) talk to the engine
//! only through [`Command`] + [`dispatch`]. Policy is enforced inside
//! `dispatch`/engine, never in UI (see `docs/ARCHITECTURE.md` §2).
//!
//! Phase 1: types + stub dispatch only. No engine dependency yet.

pub mod dispatch;
pub mod error;
pub mod types;

pub use dispatch::dispatch;
pub use error::CommandError;
pub use types::{
    AgentCancelArgs, AgentRunArgs, AgentStatusArgs, Command, CommandResult, FileReadArgs,
    FileWriteArgs, MemoryHit, MemorySearchArgs, SessionCloseArgs, SessionOpenArgs, ShellExecArgs,
    SystemInfoData,
};
