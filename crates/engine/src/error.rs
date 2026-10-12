//! Every failure an [`crate::Engine`] turn can report.

use thiserror::Error;

/// Failures of [`crate::Engine::handle_agent_run`] / [`crate::Engine::dispatch`].
#[derive(Debug, Error)]
pub enum EngineError {
    /// Policy gate said no (`Decision::Deny`).
    #[error("run denied by policy: {0}")]
    Denied(String),

    /// Caller-supplied args failed validation (e.g. empty prompt).
    #[error("invalid args: {0}")]
    InvalidArgs(String),

    /// Session JSON load/save failed.
    #[error("session store: {0}")]
    Session(#[from] computer_agent_session_store::session::storage::StorageError),

    /// `Command` variant this engine does not execute yet.
    #[error("unsupported command: {0}")]
    Unsupported(String),

    /// Anything else (carries context, never a bare string).
    #[error("engine: {0}")]
    Internal(#[from] anyhow::Error),
}

impl EngineError {
    /// Translate into the frontend-facing [`command_api::CommandError`].
    pub fn to_command_error(&self) -> command_api::CommandError {
        match self {
            Self::InvalidArgs(reason) => command_api::CommandError::InvalidArgs {
                command: "AgentRun".to_string(),
                reason: reason.clone(),
            },
            Self::Unsupported(cmd) => command_api::CommandError::Unsupported(cmd.clone()),
            other => command_api::CommandError::Dispatch(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_to_command_error_shapes() {
        let e = EngineError::InvalidArgs("prompt must not be empty".to_string());
        assert!(matches!(
            e.to_command_error(),
            command_api::CommandError::InvalidArgs { .. }
        ));
        let e = EngineError::Unsupported("SystemInfo".to_string());
        assert!(matches!(
            e.to_command_error(),
            command_api::CommandError::Unsupported(_)
        ));
        let e = EngineError::Denied("nope".to_string());
        assert!(matches!(
            e.to_command_error(),
            command_api::CommandError::Dispatch(_)
        ));
    }
}
