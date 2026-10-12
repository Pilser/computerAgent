//! Agent + tool errors.

/// Top-level agent error.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("tool error: {0}")]
    Tool(#[from] ToolError),
    #[error("policy error: {0}")]
    Policy(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("runner error: {0}")]
    Runner(String),
}

/// Per-tool-call failure. `Denied` is a policy verdict, `Msg` is anything
/// else (I/O, timeout, misuse). Kept stringly like the vendor port source.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("{0}")]
    Msg(String),
    #[error("permission denied: {0}")]
    Denied(String),
}

impl From<std::io::Error> for ToolError {
    fn from(e: std::io::Error) -> Self {
        ToolError::Msg(e.to_string())
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(e: serde_json::Error) -> Self {
        ToolError::Msg(e.to_string())
    }
}

impl From<computer_agent_sys_tools::error::SysError> for ToolError {
    fn from(e: computer_agent_sys_tools::error::SysError) -> Self {
        ToolError::Msg(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_error_from_io() {
        let e = ToolError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(matches!(e, ToolError::Msg(_)));
    }

    #[test]
    fn agent_error_wraps_tool() {
        let e = AgentError::from(ToolError::Denied("no".into()));
        assert!(e.to_string().contains("permission denied"));
    }
}
