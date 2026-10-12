//! Slint application shell: window wiring only, no logic.
//!
//! No `slint` crate dependency yet (shell only, GUI deps land later), so
//! [`App::run`] just proves the dispatch path: it builds an [`Engine`],
//! dispatches [`Command::SystemInfo`](command-api::Command) through
//! [`command-api::dispatch`], runs one stub turn through the engine seam,
//! and prints both results. No window is opened.

use anyhow::Context;
use command_api::CommandResult;
use computer_agent_engine::Engine;

use crate::bridge::{agent_run_command, system_info_command};

/// Thin shell around one shared [`Engine`] handle.
pub struct App {
    engine: Engine,
}

impl App {
    /// Build the shell with a production [`Engine`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    /// Build the shell against an isolated session store (tests/embeddings).
    #[must_use]
    pub fn with_store_dir(store_dir: std::path::PathBuf) -> Self {
        Self {
            engine: Engine::with_store_dir(store_dir),
        }
    }

    /// Prove the dispatch path: `SystemInfo` via `command-api`, one
    /// `AgentRun` stub turn via the engine seam, both printed to stdout.
    ///
    /// # Errors
    ///
    /// [`anyhow::Error`] when either dispatch fails.
    pub async fn run(&self) -> anyhow::Result<()> {
        let info: CommandResult = command_api::dispatch(system_info_command())
            .await
            .context("dispatch SystemInfo")?;
        println!("system-info: {info:?}");

        let run: CommandResult = self
            .engine
            .dispatch(agent_run_command("hi".to_string(), None))
            .await
            .context("engine dispatch AgentRun")?;
        println!("agent-run: {run:?}");

        assert_is_system_info(&info);
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn assert_is_system_info(result: &CommandResult) {
    assert!(
        matches!(result, CommandResult::SystemInfo(_)),
        "SystemInfo dispatch must answer SystemInfo, got {result:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke: `App::run` dispatches `SystemInfo` + `AgentRun("hi")`
    /// end-to-end without opening a window.
    #[tokio::test]
    async fn app_run_proves_dispatch_path() {
        let dir = std::path::PathBuf::from("/tmp/opencode/frontend-slint-tests/app-run");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let app = App::with_store_dir(dir.clone());
        app.run().await.expect("app run dispatches");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
