//! Turn events: the runner's output stream.
//!
//! Slim port of the vendor `runner.rs` event flow (`AgentEvent`): tool calls
//! the model makes map to [`TurnEvent::ToolCall`] / [`TurnEvent::ToolResult`],
//! plain text to [`TurnEvent::Text`], and the finished reply to
//! [`TurnEvent::Done`].

/// One step of a turn.
#[derive(Debug, Clone, PartialEq)]
pub enum TurnEvent {
    /// Assistant text delta / message.
    Text(String),
    /// Model requested a tool call.
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
    /// A tool call finished.
    ToolResult { name: String, output: String },
    /// Turn finished with the final reply.
    Done { response: String },
}

impl TurnEvent {
    /// Build a `ToolCall` from a model tool invocation.
    pub fn tool_call(name: &str, args: serde_json::Value) -> Self {
        Self::ToolCall {
            name: name.to_string(),
            args,
        }
    }

    /// Build a `ToolResult` from a finished tool invocation.
    pub fn tool_result(name: &str, output: impl Into<String>) -> Self {
        Self::ToolResult {
            name: name.to_string(),
            output: output.into(),
        }
    }

    /// True for the terminal event.
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_call_round_trip() {
        let args = serde_json::json!({"path": "a.txt"});
        let e = TurnEvent::tool_call("read", args.clone());
        assert_eq!(
            e,
            TurnEvent::ToolCall {
                name: "read".into(),
                args
            }
        );
        assert!(!e.is_done());
    }

    #[test]
    fn done_is_terminal() {
        let e = TurnEvent::Done {
            response: "ok".into(),
        };
        assert!(e.is_done());
        assert!(matches!(
            TurnEvent::tool_result("bash", "out"),
            TurnEvent::ToolResult { .. }
        ));
    }
}
