//! `Command` (human -> system) and `CommandResult` (system -> human).
//!
//! One domain per file; this module only declares the enums and re-exports
//! the per-domain arg/result payloads. Everything is serde-serializable and
//! versioned via the adjacently-tagged `type` field so frontends can later
//! send the same JSON over Tauri IPC / HTTP without changes.
//!
//! NOTE: `docs/ARCHITECTURE.md` §1 lists both `types.rs` and `types/mod.rs`.
//! Rust allows only one module file, so this is `types/mod.rs` (no
//! `types.rs`). Behavior is as specified.

pub mod agent_cmds;
pub mod session_cmds;
pub mod sys_cmds;

pub use agent_cmds::{AgentCancelArgs, AgentRunArgs, AgentState, AgentStatusArgs};
pub use session_cmds::{MemorySearchArgs, SessionCloseArgs, SessionOpenArgs};
pub use sys_cmds::{FileReadArgs, FileWriteArgs, ShellExecArgs, SystemInfoData};

use serde::{Deserialize, Serialize};

/// Every action a frontend may request. Policy is enforced inside
/// [`crate::dispatch`] / engine, never in UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Command {
    AgentRun(AgentRunArgs),
    AgentCancel(AgentCancelArgs),
    AgentStatus(AgentStatusArgs),
    SystemInfo,
    ShellExec(ShellExecArgs),
    FileRead(FileReadArgs),
    FileWrite(FileWriteArgs),
    SessionOpen(SessionOpenArgs),
    SessionClose(SessionCloseArgs),
    MemorySearch(MemorySearchArgs),
}

/// Stub-able outcome of a [`Command`]. The engine (phase 8) fills these with
/// real data; phase 1 returns deterministic stub values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CommandResult {
    AgentAccepted {
        run_id: String,
    },
    AgentCancelled,
    AgentStatus {
        state: AgentState,
    },
    SystemInfo(SystemInfoData),
    ShellExec {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    FileRead {
        content: String,
    },
    FileWrite {
        bytes_written: u64,
    },
    SessionOpened {
        session_id: String,
    },
    SessionClosed,
    MemorySearch {
        hits: Vec<MemoryHit>,
    },
}

/// Long-term memory reference returned by `MemorySearch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryHit {
    pub path: String,
    pub snippet: String,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(cmd: &Command) {
        let json = serde_json::to_string(cmd).expect("serialize Command");
        let back: Command = serde_json::from_str(&json).expect("deserialize Command");
        assert_eq!(&back, cmd);
    }

    #[test]
    fn all_commands_serde_round_trip() {
        round_trip(&Command::AgentRun(AgentRunArgs {
            prompt: "hello".into(),
            session_id: None,
        }));
        round_trip(&Command::AgentCancel(AgentCancelArgs {
            run_id: "r1".into(),
        }));
        round_trip(&Command::AgentStatus(AgentStatusArgs {
            run_id: "r1".into(),
        }));
        round_trip(&Command::SystemInfo);
        round_trip(&Command::ShellExec(ShellExecArgs {
            command: "ls".into(),
            args: vec!["-la".into()],
            cwd: None,
            timeout_ms: None,
        }));
        round_trip(&Command::FileRead(FileReadArgs {
            path: "/tmp/x".into(),
            max_bytes: None,
        }));
        round_trip(&Command::FileWrite(FileWriteArgs {
            path: "/tmp/x".into(),
            content: "hi".into(),
            create_dirs: false,
        }));
        round_trip(&Command::SessionOpen(SessionOpenArgs { session_id: None }));
        round_trip(&Command::SessionClose(SessionCloseArgs {
            session_id: "s1".into(),
        }));
        round_trip(&Command::MemorySearch(MemorySearchArgs {
            query: "scratchpad".into(),
            limit: None,
        }));
    }

    #[test]
    fn results_serde_round_trip() {
        let results = vec![
            CommandResult::AgentAccepted { run_id: "r".into() },
            CommandResult::AgentCancelled,
            CommandResult::AgentStatus {
                state: AgentState::Running,
            },
            CommandResult::SystemInfo(SystemInfoData {
                os: "linux".into(),
                arch: "x86_64".into(),
            }),
            CommandResult::ShellExec {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
            },
            CommandResult::FileRead {
                content: "c".into(),
            },
            CommandResult::FileWrite { bytes_written: 1 },
            CommandResult::SessionOpened {
                session_id: "s".into(),
            },
            CommandResult::SessionClosed,
            CommandResult::MemorySearch { hits: vec![] },
        ];
        for r in &results {
            let json = serde_json::to_string(r).expect("serialize result");
            let back: CommandResult = serde_json::from_str(&json).expect("deserialize result");
            assert_eq!(&back, r);
        }
    }
}
