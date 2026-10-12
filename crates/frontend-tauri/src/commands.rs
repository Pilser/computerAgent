//! Tauri commands: same [`Command`](command-api::Command) enum as Slint.
//!
//! Pure builders first (`agent_run_command`, `system_info_command`), then thin
//! async wrappers over the real seams — [`command-api::dispatch`] for
//! `SystemInfo`, the [`Engine`](computer_agent_engine::Engine) seam from
//! phase 8 for `AgentRun`. (The `#[tauri::command]` attributes land with the
//! Tauri dependency; the signatures here already match that shape.)

use command_api::{AgentRunArgs, Command, CommandResult};
use computer_agent_engine::Engine;

/// Build the `AgentRun` command for `agent_run(prompt)`.
#[must_use]
pub fn agent_run_command(prompt: String, session_id: Option<String>) -> Command {
    Command::AgentRun(AgentRunArgs { prompt, session_id })
}

/// Build the `SystemInfo` command for `system_info()`.
#[must_use]
pub fn system_info_command() -> Command {
    Command::SystemInfo
}

/// Tauri command `agent_run`: one stub turn through the engine seam.
///
/// # Errors
///
/// [`anyhow::Error`] wrapping [`computer_agent_engine::EngineError`].
pub async fn agent_run(
    engine: &Engine,
    prompt: String,
    session_id: Option<String>,
) -> anyhow::Result<CommandResult> {
    Ok(engine
        .dispatch(agent_run_command(prompt, session_id))
        .await?)
}

/// Tauri command `system_info` through [`command-api::dispatch`] (stub).
///
/// # Errors
///
/// Forwards [`command-api::CommandError`].
pub async fn system_info() -> Result<CommandResult, command_api::CommandError> {
    command_api::dispatch(system_info_command()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_run_builder_carries_prompt() {
        assert_eq!(
            agent_run_command("hi".to_string(), None),
            Command::AgentRun(AgentRunArgs {
                prompt: "hi".to_string(),
                session_id: None,
            })
        );
    }

    #[test]
    fn system_info_builder_is_unit_variant() {
        assert_eq!(system_info_command(), Command::SystemInfo);
    }

    /// Smoke: `system_info()` end-to-end through `command-api::dispatch`.
    #[tokio::test]
    async fn smoke_system_info_command() {
        let result = system_info().await.expect("SystemInfo stub");
        assert!(matches!(result, CommandResult::SystemInfo(_)));
    }

    /// Smoke: `agent_run("hi")` end-to-end through the engine seam. No
    /// window is opened; the session store is isolated under `/tmp/opencode`.
    #[tokio::test]
    async fn smoke_agent_run_command() {
        let dir = std::path::PathBuf::from("/tmp/opencode/frontend-tauri-tests/smoke");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let engine = Engine::with_store_dir(dir.clone());

        let result = agent_run(&engine, "hi".to_string(), None)
            .await
            .expect("stub run");
        assert!(matches!(result, CommandResult::AgentAccepted { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
