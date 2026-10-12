//! Async entry point for every frontend action.
//!
//! Phase 1: stub only — matches on [`Command`] and returns a deterministic
//! [`CommandResult`] per variant. No engine dependency yet (wired in phase 8,
//! with policy enforced here/inside-engine, never in UI).

use crate::error::CommandError;
use crate::types::{AgentState, Command, CommandResult, MemoryHit, SystemInfoData};

/// Handle one [`Command`], returning its stub [`CommandResult`].
///
/// # Errors
///
/// Currently infallible for well-formed commands; returns [`CommandError`]
/// once validation/policy/engine wiring lands (phases 2–8).
pub async fn dispatch(cmd: Command) -> Result<CommandResult, CommandError> {
    match cmd {
        Command::AgentRun(args) => {
            // TODO(phase8): route through `computer_agent_engine::Engine::dispatch`
            // (policy + session persist live there). This stub must NOT grow an
            // engine dep: engine already deps command-api, so the call would cycle.
            if args.prompt.trim().is_empty() {
                return Err(CommandError::InvalidArgs {
                    command: "AgentRun".to_string(),
                    reason: "prompt must not be empty".to_string(),
                });
            }
            Ok(CommandResult::AgentAccepted {
                run_id: "stub-run-id".to_string(),
            })
        }
        Command::AgentCancel(_) => Ok(CommandResult::AgentCancelled),
        Command::AgentStatus(_) => Ok(CommandResult::AgentStatus {
            state: AgentState::Queued,
        }),
        Command::SystemInfo => Ok(CommandResult::SystemInfo(SystemInfoData {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        })),
        Command::ShellExec(_) => Ok(CommandResult::ShellExec {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }),
        Command::FileRead(_) => Ok(CommandResult::FileRead {
            content: String::new(),
        }),
        Command::FileWrite(_) => Ok(CommandResult::FileWrite { bytes_written: 0 }),
        Command::SessionOpen(args) => Ok(CommandResult::SessionOpened {
            session_id: args
                .session_id
                .unwrap_or_else(|| "stub-session".to_string()),
        }),
        Command::SessionClose(_) => Ok(CommandResult::SessionClosed),
        Command::MemorySearch(_) => Ok(CommandResult::MemorySearch {
            hits: vec![stub_hit()],
        }),
    }
}

fn stub_hit() -> MemoryHit {
    MemoryHit {
        path: "memory/stub.md".to_string(),
        snippet: "stub result (no session-store yet)".to_string(),
        score: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AgentCancelArgs, AgentRunArgs, AgentStatusArgs, FileReadArgs, FileWriteArgs,
        MemorySearchArgs, SessionCloseArgs, SessionOpenArgs, ShellExecArgs,
    };

    /// Every `Command` variant must dispatch to its stub result.
    #[tokio::test]
    async fn dispatch_stub_covers_every_variant() {
        let cases: Vec<(Command, fn(&CommandResult))> = vec![
            (
                Command::AgentRun(AgentRunArgs {
                    prompt: "hi".into(),
                    session_id: None,
                }),
                |r| assert!(matches!(r, CommandResult::AgentAccepted { .. })),
            ),
            (
                Command::AgentCancel(AgentCancelArgs { run_id: "r".into() }),
                |r| assert!(matches!(r, CommandResult::AgentCancelled)),
            ),
            (
                Command::AgentStatus(AgentStatusArgs { run_id: "r".into() }),
                |r| assert!(matches!(r, CommandResult::AgentStatus { .. })),
            ),
            (Command::SystemInfo, |r| {
                assert!(matches!(r, CommandResult::SystemInfo(_)))
            }),
            (
                Command::ShellExec(ShellExecArgs {
                    command: "echo".into(),
                    args: vec![],
                    cwd: None,
                    timeout_ms: None,
                }),
                |r| assert!(matches!(r, CommandResult::ShellExec { .. })),
            ),
            (
                Command::FileRead(FileReadArgs {
                    path: "/tmp/x".into(),
                    max_bytes: None,
                }),
                |r| assert!(matches!(r, CommandResult::FileRead { .. })),
            ),
            (
                Command::FileWrite(FileWriteArgs {
                    path: "/tmp/x".into(),
                    content: "hi".into(),
                    create_dirs: false,
                }),
                |r| assert!(matches!(r, CommandResult::FileWrite { .. })),
            ),
            (
                Command::SessionOpen(SessionOpenArgs { session_id: None }),
                |r| assert!(matches!(r, CommandResult::SessionOpened { .. })),
            ),
            (
                Command::SessionClose(SessionCloseArgs {
                    session_id: "s".into(),
                }),
                |r| assert!(matches!(r, CommandResult::SessionClosed)),
            ),
            (
                Command::MemorySearch(MemorySearchArgs {
                    query: "q".into(),
                    limit: None,
                }),
                |r| assert!(matches!(r, CommandResult::MemorySearch { .. })),
            ),
        ];
        for (cmd, check) in cases {
            let result = dispatch(cmd).await.expect("stub dispatch must succeed");
            check(&result);
        }
    }

    #[tokio::test]
    async fn dispatch_rejects_empty_prompt() {
        let err = dispatch(Command::AgentRun(AgentRunArgs {
            prompt: "   ".into(),
            session_id: None,
        }))
        .await
        .expect_err("empty prompt must fail");
        assert!(matches!(err, CommandError::InvalidArgs { .. }));
    }
}
