//! Session/memory-domain commands: open/close sessions, search memory.

use serde::{Deserialize, Serialize};

/// Open a session. `None` creates a new one; `Some(id)` resumes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionOpenArgs {
    pub session_id: Option<String>,
}

/// Close (persist + unload) a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCloseArgs {
    pub session_id: String,
}

/// Full-text search over long-term memory (`MEMORY.md`, `notes/*.md`, …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySearchArgs {
    pub query: String,
    pub limit: Option<u32>,
}
