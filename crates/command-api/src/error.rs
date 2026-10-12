//! Error type for [`crate::dispatch`] and arg validation.

use thiserror::Error;

/// Every failure a [`crate::Command`] can report to a frontend.
#[derive(Debug, Error)]
pub enum CommandError {
    /// Caller-supplied args failed validation.
    #[error("invalid args for {command}: {reason}")]
    InvalidArgs { command: String, reason: String },

    /// Stub engine failure placeholder (real variants arrive with engine).
    #[error("dispatch failed: {0}")]
    Dispatch(String),

    /// Command is known but not wired yet.
    #[error("unsupported command: {0}")]
    Unsupported(String),

    /// JSON (de)serialization of a command or result failed.
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}
