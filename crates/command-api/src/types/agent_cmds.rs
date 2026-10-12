//! Agent-domain commands: start, cancel, and poll an LLM agent run.

use serde::{Deserialize, Serialize};

/// Start (or continue, when `session_id` is set) an agent run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRunArgs {
    pub prompt: String,
    pub session_id: Option<String>,
}

/// Cancel a running agent run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCancelArgs {
    pub run_id: String,
}

/// Poll the state of an agent run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStatusArgs {
    pub run_id: String,
}

/// Lifecycle state reported by `AgentStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Queued,
    Running,
    Cancelled,
    Done,
}
