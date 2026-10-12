//! Engine: single-turn orchestration (`command-api` -> `agent`).
//!
//! Slim port of the `_vendor/embedded-agent` engine turn shape (prompt
//! assemble -> runner -> session persist), without the TUI/LLM backends.
//! [`Engine::handle_agent_run`] is the whole pipeline; [`Engine::dispatch`]
//! is the `command-api` seam for `AgentRun`/`AgentCancel` (see
//! `docs/ARCHITECTURE.md` §2). Policy is enforced here, never in UI.

pub mod error;
pub mod threads;
pub mod turns;

pub use error::EngineError;
pub use threads::{ThreadInfo, ThreadUsage, DEFAULT_MODEL, FALLBACK_CONTEXT_WINDOW, THREAD_LIMIT};
pub use turns::{CancelMap, CancelToken, Engine, StreamHub, TurnEvent, AGENT_RUN_TOOL};
