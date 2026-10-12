//! Slint-callback -> [`Command`](command-api::Command) mapping.
//!
//! Pure functions only: turning a UI callback into a `Command` performs no
//! policy check and no I/O. The thin async wrappers below hand the built
//! `Command` to the real seams — [`command-api::dispatch`] for `SystemInfo`,
//! the [`Engine`](computer_agent_engine::Engine) seam from phase 8 for
//! `AgentRun` (the engine rejects every other variant with `Unsupported`).

use command_api::{AgentRunArgs, Command, CommandResult};
use computer_agent_engine::Engine;

/// UI callbacks the (future) Slint views will emit.
#[derive(Debug, Clone, PartialEq)]
pub enum SlintCallback {
    /// `on_send(prompt)` in the chat view.
    Send {
        prompt: String,
        session_id: Option<String>,
    },
    /// `on_info()` in the status view.
    Info,
}

/// Map one [`SlintCallback`] to its [`Command`]. Pure: no I/O, no policy.
#[must_use]
pub fn callback_to_command(callback: SlintCallback) -> Command {
    match callback {
        SlintCallback::Send { prompt, session_id } => agent_run_command(prompt, session_id),
        SlintCallback::Info => system_info_command(),
    }
}

/// Build the `AgentRun` command for `on_send`.
#[must_use]
pub fn agent_run_command(prompt: String, session_id: Option<String>) -> Command {
    Command::AgentRun(AgentRunArgs { prompt, session_id })
}

/// Build the `SystemInfo` command for `on_info`.
#[must_use]
pub fn system_info_command() -> Command {
    Command::SystemInfo
}

/// Run one agent turn through the engine seam (phase 8).
///
/// # Errors
///
/// Forwards [`computer_agent_engine::EngineError`] (empty prompt, deny, ...).
pub async fn run_agent(
    engine: &Engine,
    prompt: &str,
    session_id: Option<&str>,
) -> Result<CommandResult, computer_agent_engine::EngineError> {
    engine
        .dispatch(Command::AgentRun(AgentRunArgs {
            prompt: prompt.to_string(),
            session_id: session_id.map(str::to_string),
        }))
        .await
}

/// Fetch system info through [`command-api::dispatch`] (stub in v1).
///
/// # Errors
///
/// Forwards [`command-api::CommandError`].
pub async fn fetch_system_info() -> Result<CommandResult, command_api::CommandError> {
    command_api::dispatch(Command::SystemInfo).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_send_maps_to_agent_run() {
        let cmd = callback_to_command(SlintCallback::Send {
            prompt: "hi".to_string(),
            session_id: None,
        });
        assert_eq!(
            cmd,
            Command::AgentRun(AgentRunArgs {
                prompt: "hi".to_string(),
                session_id: None,
            })
        );
    }

    #[test]
    fn on_info_maps_to_system_info() {
        assert_eq!(
            callback_to_command(SlintCallback::Info),
            Command::SystemInfo
        );
    }

    /// Smoke: `SystemInfo` end-to-end through `command-api::dispatch`.
    #[tokio::test]
    async fn smoke_system_info_dispatch() {
        let result = command_api::dispatch(system_info_command())
            .await
            .expect("SystemInfo stub");
        assert!(matches!(result, CommandResult::SystemInfo(_)));
    }

    /// Smoke: `AgentRun("hi")` end-to-end through the engine seam. No
    /// window is opened; the session store is isolated under `/tmp/opencode`.
    #[tokio::test]
    async fn smoke_agent_run_engine_seam() {
        let dir = std::path::PathBuf::from("/tmp/opencode/frontend-slint-tests/smoke");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let engine = Engine::with_store_dir(dir.clone());

        let result = run_agent(&engine, "hi", None).await.expect("stub run");
        assert!(matches!(result, CommandResult::AgentAccepted { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
