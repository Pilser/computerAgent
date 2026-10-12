//! System-domain commands: host info, shell, and file access.

use serde::{Deserialize, Serialize};

/// Run a shell command. Full-privilege in v1 (sandbox OFF by default);
/// policy gating lands in `dispatch`/engine (phase 8), never in UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellExecArgs {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Read a file from disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileReadArgs {
    pub path: String,
    pub max_bytes: Option<u64>,
}

/// Write (create or overwrite) a file on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileWriteArgs {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub create_dirs: bool,
}

/// Host description returned for `Command::SystemInfo`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemInfoData {
    pub os: String,
    pub arch: String,
}
