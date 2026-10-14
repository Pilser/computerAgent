//! Single-turn orchestration: policy -> assemble -> runner -> persist.
//!
//! Slim port of the `_vendor/embedded-agent` engine turn shape (`run_string`
//! minus TUI/slash/LLM): [`Engine::handle_agent_run`] gates the prompt on
//! policy, assembles `system + memory inject + session recap` via
//! [`computer_agent_agent::prompt::assemble`], runs the stub runner, fans
//! events out over [`StreamHub`](super::stream::StreamHub), persists the
//! user + assistant messages via `session-store`, and answers
//! [`CommandResult::AgentAccepted`](command_api::CommandResult).
//!
//! [`Engine::dispatch`] is the `command-api` seam for `AgentRun` (plus
//! `AgentCancel` via the [`CancelMap`](super::cancel::CancelMap)); every
//! other [`Command`](command_api::Command) stays a stub in `command-api`
//! until its phase wires it here.

use std::path::PathBuf;
use std::sync::Mutex;

use command_api::{Command, CommandResult};
use computer_agent_agent::prompt::{assemble, system_prompt};
use computer_agent_agent::runner::{run_turn_stub, TurnEvent, TurnRequest};
use computer_agent_agent::tools::ToolRegistry;
use computer_agent_policy::{Decision, PermissionChecker, SecurityMode};
use computer_agent_session_store::memory::Mem;
use computer_agent_session_store::session::storage::{data_dir, load_session_in, save_session_in};
use computer_agent_session_store::session::{MessageRole, Session};

use super::cancel::CancelMap;
use super::stream::StreamHub;
use crate::error::EngineError;
use crate::threads::{thread_title, DEFAULT_MODEL};

/// Policy tool name gated by [`Engine::handle_agent_run`].
pub const AGENT_RUN_TOOL: &str = "agent_run";

/// Session recap depth (messages) and per-message snippet width.
const RECAP_KEEP: usize = 10;
const RECAP_SNIP: usize = 200;

/// Turn orchestrator. Owns policy + store dir + memory + fan-out + cancels.
pub struct Engine {
    policy: Mutex<PermissionChecker>,
    pub(crate) store_dir: PathBuf,
    memory: Mem,
    events: StreamHub,
    pub(crate) cancels: CancelMap,
    pub(crate) current_model: Mutex<String>,
}

impl Engine {
    /// Production constructor: standard policy, real data dir, live memory.
    pub fn new() -> Self {
        Self {
            policy: Mutex::new(PermissionChecker::new(SecurityMode::Standard)),
            store_dir: data_dir(),
            memory: Mem::open(),
            events: StreamHub::default(),
            cancels: CancelMap::default(),
            current_model: Mutex::new(DEFAULT_MODEL.to_string()),
        }
    }

    /// Test/embedding constructor: isolated session store dir.
    pub fn with_store_dir(store_dir: PathBuf) -> Self {
        Self {
            store_dir,
            ..Self::new()
        }
    }

    /// Replace the policy gate (tests install deny rules here).
    pub fn with_policy(mut self, checker: PermissionChecker) -> Self {
        self.policy = Mutex::new(checker);
        self
    }

    /// Replace the memory handle (tests point it at a temp root).
    pub fn with_memory(mut self, memory: Mem) -> Self {
        self.memory = memory;
        self
    }

    /// Fan-out hub: frontends subscribe before dispatching a run.
    pub fn events(&self) -> &StreamHub {
        &self.events
    }

    /// Request cancellation of an in-flight run (see [`CancelMap::cancel`]).
    pub fn cancel_run(&self, run_id: &str) -> bool {
        self.cancels.cancel(run_id)
    }

    /// One agent turn: policy -> assemble -> stub runner -> persist.
    ///
    /// # Errors
    ///
    /// [`EngineError::InvalidArgs`] on an empty prompt,
    /// [`EngineError::Denied`] when policy denies `agent_run`,
    /// [`EngineError::Session`] when the session cannot load/save.
    pub async fn handle_agent_run(
        &self,
        prompt: &str,
        session_id: Option<&str>,
    ) -> Result<CommandResult, EngineError> {
        if prompt.trim().is_empty() {
            return Err(EngineError::InvalidArgs(
                "prompt must not be empty".to_string(),
            ));
        }
        let decision = self
            .policy
            .lock()
            .expect("policy lock")
            .check(AGENT_RUN_TOOL, prompt);
        if let Decision::Deny(reason) = decision {
            return Err(EngineError::Denied(reason));
        }

        let mut session = self.open_session(session_id)?;
        if session.messages.is_empty() {
            session.name = thread_title(prompt);
        }
        if session.model.trim().is_empty() || session.model == "stub" {
            session.model.clone_from(&self.current_model());
        }
        let recap = session_recap(&session);
        let registry = ToolRegistry::default_coding();
        let full_prompt = assemble(
            &system_prompt(&registry),
            self.memory.context_block().as_deref(),
            &recap,
        );

        let run_id = uuid::Uuid::new_v4().to_string();
        self.cancels.track(&run_id);
        // NOTE: the stub echoes TurnRequest.prompt as its reply, so it must
        // receive the USER text only. The assembled system+memory+recap prompt
        // (`_full_prompt`) is built for the real LLM milestone and must never
        // be persisted as chat content (it once leaked into the UI as a wall
        // of instructions). Real runner takes `_full_prompt` back here.
        let _full_prompt = full_prompt;
        let req = TurnRequest::new(prompt.to_string(), 4).with_summary(recap);
        let mut response = String::new();
        for event in run_turn_stub(&req) {
            if let TurnEvent::Done { response: done } = &event {
                response.clone_from(done);
            }
            self.events.publish(event);
        }
        self.cancels.finish(&run_id);

        session.add_message(MessageRole::User, prompt);
        session.add_message(MessageRole::Assistant, &response);
        save_session_in(&self.store_dir, &session)?;

        Ok(CommandResult::AgentAccepted { run_id })
    }

    /// `command-api` seam: `AgentRun`/`AgentCancel` execute here; every other
    /// variant is out of scope until its phase wires it (returns
    /// [`EngineError::Unsupported`]).
    ///
    /// # Errors
    ///
    /// Forwards [`Engine::handle_agent_run`] failures, or `Unsupported`.
    pub async fn dispatch(&self, cmd: Command) -> Result<CommandResult, EngineError> {
        match cmd {
            Command::AgentRun(args) => {
                self.handle_agent_run(&args.prompt, args.session_id.as_deref())
                    .await
            }
            Command::AgentCancel(args) => {
                self.cancel_run(&args.run_id);
                Ok(CommandResult::AgentCancelled)
            }
            other => Err(EngineError::Unsupported(command_name(&other).to_string())),
        }
    }

    fn open_session(&self, session_id: Option<&str>) -> Result<Session, EngineError> {
        if let Some(id) = session_id.filter(|id| !id.trim().is_empty()) {
            if let Some(session) = load_session_in(&self.store_dir, id)? {
                return Ok(session);
            }
            let mut fresh = Session::new("stub", "stub", 128_000, id);
            fresh.id = id.to_string();
            return Ok(fresh);
        }
        Ok(Session::new("stub", "stub", 128_000, "agent"))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Last [`RECAP_KEEP`] messages as `role: snippet` lines (empty when fresh).
fn session_recap(session: &Session) -> String {
    let start = session.messages.len().saturating_sub(RECAP_KEEP);
    session.messages[start..]
        .iter()
        .map(|m| {
            let role = match m.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
                MessageRole::ToolCall => "tool_call",
                MessageRole::ToolResult => "tool_result",
            };
            let snippet: String = m.content.chars().take(RECAP_SNIP).collect();
            format!("{role}: {snippet}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Variant name for [`EngineError::Unsupported`] (no payload leakage).
fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::AgentRun(_) => "AgentRun",
        Command::AgentCancel(_) => "AgentCancel",
        Command::AgentStatus(_) => "AgentStatus",
        Command::SystemInfo => "SystemInfo",
        Command::ShellExec(_) => "ShellExec",
        Command::FileRead(_) => "FileRead",
        Command::FileWrite(_) => "FileWrite",
        Command::SessionOpen(_) => "SessionOpen",
        Command::SessionClose(_) => "SessionClose",
        Command::MemorySearch(_) => "MemorySearch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recap_is_empty_for_fresh_session() {
        let session = Session::new("stub", "stub", 128_000, "agent");
        assert!(session_recap(&session).is_empty());
    }

    #[test]
    fn recap_lists_roles_in_order() {
        let mut session = Session::new("stub", "stub", 128_000, "agent");
        session.add_message(MessageRole::User, "do it");
        session.add_message(MessageRole::Assistant, "done");
        let recap = session_recap(&session);
        assert!(recap.contains("user: do it"));
        assert!(recap.contains("assistant: done"));
        assert!(recap.find("user:").unwrap() < recap.find("assistant:").unwrap());
    }

    #[test]
    fn every_command_has_a_name() {
        assert_eq!(command_name(&Command::SystemInfo), "SystemInfo");
        assert_eq!(
            command_name(&Command::AgentRun(command_api::AgentRunArgs {
                prompt: "hi".to_string(),
                session_id: None,
            })),
            "AgentRun"
        );
    }
}
