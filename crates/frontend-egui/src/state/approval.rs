//! Ask-approval state: one pending tool request + the last answer.
//!
//! TODO (engine wiring): nothing produces [`ApprovalRequest`]s yet —
//! [`Engine::handle_agent_run`](computer_agent_engine::Engine) treats
//! `Decision::Ask` as allow and there is no `Command::Approve/Deny` in
//! `command-api`. The approval overlay renders this state and records
//! answers here; once the engine streams Ask events, drain
//! `last_answer` there and delete this note.

/// One tool call awaiting a human decision.
#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalRequest {
    /// Policy tool name (e.g. `"bash"`).
    pub tool: String,
    /// The gated input (command line, path, …).
    pub input: String,
}

impl ApprovalRequest {
    /// Build a request from its parts.
    pub fn new(tool: &str, input: &str) -> Self {
        Self {
            tool: tool.to_string(),
            input: input.to_string(),
        }
    }
}

/// Pending Ask plus the most recent Allow/Deny answer.
#[derive(Debug, Default)]
pub struct ApprovalState {
    pending: Option<ApprovalRequest>,
    /// Last answer (`true` = allow); consumed by the engine wiring (TODO).
    pub last_answer: Option<bool>,
}

impl ApprovalState {
    /// Empty: nothing pending, no answer yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage a request (replaces any live one).
    pub fn request(&mut self, tool: &str, input: &str) {
        self.pending = Some(ApprovalRequest::new(tool, input));
    }

    /// The live request, if any.
    #[must_use]
    pub fn pending(&self) -> Option<&ApprovalRequest> {
        self.pending.as_ref()
    }

    /// Answer the pending request; `false` when nothing was pending.
    pub fn answer(&mut self, allow: bool) -> bool {
        if self.pending.is_some() {
            self.pending = None;
            self.last_answer = Some(allow);
            true
        } else {
            false
        }
    }
}
