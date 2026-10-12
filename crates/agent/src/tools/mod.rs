//! LLM-callable tools: one file per tool plus a schema registry.
//!
//! NOTE (phase 6 scope): `memory_tools` is intentionally absent. The
//! `session-store` memory bridge ships with engine wiring in a later phase.
//! `edit`/`grep`/`find_files`/`list_dir`/`todo` are likewise deferred.

pub mod bash;
pub mod read;
pub mod registry;
pub mod write;

pub use bash::{BashArgs, BashTool};
pub use read::{ReadArgs, ReadTool};
pub use registry::ToolRegistry;
pub use write::{WriteArgs, WriteTool};

use std::sync::{Arc, Mutex};

use computer_agent_policy::{Decision, PermissionChecker};
use serde::de::DeserializeOwned;

use crate::error::ToolError;

/// Shareable policy gate. Tools lock it briefly per call (`check` /
/// `check_path` take `&mut self`); the guard is never held across I/O.
pub type SharedPolicy = Arc<Mutex<PermissionChecker>>;

/// Minimal tool contract (ours, no `rig` in v1).
pub trait Tool: Send + Sync {
    const NAME: &'static str;
    type Args: DeserializeOwned + Send;
    fn description(&self) -> String;
    fn parameters(&self) -> serde_json::Value;
    async fn call(&self, args: Self::Args) -> Result<String, ToolError>;
}

/// Gate a path-scoped tool (`read`, `write`) on policy. Must run BEFORE any
/// `sys-tools` call. `Ask` has no UI channel in v1 (approval dialogs arrive
/// with engine `stream.rs` in phase 8), so it is a hard block here.
pub(crate) fn check_policy_path(
    policy: &Option<SharedPolicy>,
    tool: &str,
    path: &str,
) -> Result<(), ToolError> {
    let Some(p) = policy else {
        return Ok(());
    };
    match p
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .check_path(tool, path)
    {
        Decision::Allow => Ok(()),
        Decision::Ask => Err(ToolError::Denied(format!(
            "{tool} {path} requires approval (non-interactive session)"
        ))),
        Decision::Deny(reason) => Err(ToolError::Denied(reason)),
    }
}

/// Gate a command-scoped tool (`bash`) on policy. Same Ask rule as above.
pub(crate) fn check_policy(
    policy: &Option<SharedPolicy>,
    tool: &str,
    input: &str,
) -> Result<(), ToolError> {
    let Some(p) = policy else {
        return Ok(());
    };
    match p
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .check(tool, input)
    {
        Decision::Allow => Ok(()),
        Decision::Ask => Err(ToolError::Denied(format!(
            "{tool} requires approval (non-interactive session)"
        ))),
        Decision::Deny(reason) => Err(ToolError::Denied(reason)),
    }
}

/// `~` expansion. Mirrors vendor `fs::expand_tilde` via the policy helper so
/// tools and the checker agree on paths.
pub(crate) fn expand_tilde(path: &str) -> String {
    computer_agent_policy::pattern::expand_tilde(path)
}

/// Build an empty allow-by-default gate for tests and direct use.
pub fn open_policy() -> SharedPolicy {
    use computer_agent_policy::decision::SecurityMode;
    Arc::new(Mutex::new(PermissionChecker::new(SecurityMode::Yolo)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use computer_agent_policy::decision::{Action, SecurityMode};

    #[test]
    fn open_policy_allows() {
        let p = open_policy();
        assert!(check_policy(&Some(p.clone()), "bash", "echo hi").is_ok());
        assert!(check_policy_path(&Some(p), "read", "/tmp/x").is_ok());
    }

    #[test]
    fn deny_rule_blocks_before_io() {
        let mut c = PermissionChecker::new(SecurityMode::Standard);
        c.deny("read", "/blocked/**");
        let p = Arc::new(Mutex::new(c));
        let err = check_policy_path(&Some(p), "read", "/blocked/x.txt").unwrap_err();
        assert!(matches!(err, ToolError::Denied(_)));
    }

    #[test]
    fn ask_blocks_without_ui_channel() {
        let mut c = PermissionChecker::new(SecurityMode::Standard);
        c.rule("bash", "deploy *", false, Action::Ask);
        let p = Arc::new(Mutex::new(c));
        let err = check_policy(&Some(p), "bash", "deploy prod").unwrap_err();
        assert!(matches!(err, ToolError::Denied(_)));
    }

    #[test]
    fn no_policy_is_open() {
        assert!(check_policy(&None, "bash", "anything").is_ok());
        assert!(check_policy_path(&None, "read", "/anywhere").is_ok());
    }
}
