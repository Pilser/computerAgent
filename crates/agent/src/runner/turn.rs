//! Single-turn stub runner.
//!
//! Stands in for vendor `agent/runner.rs` (`spawn_agent` + streaming loop).
//! Real LLM execution arrives in a later milestone; v1 returns a
//! deterministic [`TurnEvent`] sequence so the engine wiring (phase 8) can be
//! built and tested now.

use super::events::TurnEvent;

/// Input to one turn: the assembled prompt plus a snapshot summary of the
/// session and a cap on tool rounds.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    /// Fully assembled prompt (see `prompt::assemble`).
    pub prompt: String,
    /// Short recap of the session so far (for logging, not the prompt).
    pub session_summary: Option<String>,
    /// Max tool-call rounds the real runner may perform.
    pub max_tool_rounds: u32,
}

impl TurnRequest {
    pub fn new(prompt: impl Into<String>, max_tool_rounds: u32) -> Self {
        Self {
            prompt: prompt.into(),
            session_summary: None,
            max_tool_rounds,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.session_summary = Some(summary.into());
        self
    }
}

/// Deterministic stub: emits `Text` (echo of the prompt) then `Done`.
/// Never touches the network; safe in tests.
pub fn run_turn_stub(req: &TurnRequest) -> Vec<TurnEvent> {
    let reply = format!("stub reply to: {}", req.prompt);
    vec![
        TurnEvent::Text(reply.clone()),
        TurnEvent::Done { response: reply },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_emits_text_then_done() {
        let req = TurnRequest::new("hello", 4).with_summary("empty session");
        let events = run_turn_stub(&req);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], TurnEvent::Text(t) if t.contains("hello")));
        assert!(matches!(&events[1], TurnEvent::Done { .. }));
        assert!(events[1].is_done());
    }

    #[test]
    fn stub_without_summary() {
        let req = TurnRequest::new("hi", 1);
        assert_eq!(req.session_summary, None);
        let events = run_turn_stub(&req);
        assert_eq!(events.len(), 2);
    }
}
