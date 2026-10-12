//! Policy gate: `Decision` verdicts + the [`Policy`] trait.
//!
//! Frontends render `Ask`; enforcement lives in
//! [`Policy`](crate::PermissionChecker) / `command-api::dispatch`, never in UI.

use serde::{Deserialize, Serialize};

/// Verdict for one tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Ask,
    Deny(String),
}

/// Rule outcome before mode resolution and doom-loop handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Ask,
    Deny,
}

/// Permission mode. Port of vendor `permission::SecurityMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityMode {
    Standard,
    Restrictive,
    ReadOnly,
    PlanWrite,
    Guarded,
    Yolo,
}

impl SecurityMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "standard" => Some(Self::Standard),
            "restrictive" => Some(Self::Restrictive),
            "readonly" => Some(Self::ReadOnly),
            "planwrite" => Some(Self::PlanWrite),
            "guarded" => Some(Self::Guarded),
            "yolo" => Some(Self::Yolo),
            _ => None,
        }
    }
}

impl std::fmt::Display for SecurityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Standard => "standard",
            Self::Restrictive => "restrictive",
            Self::ReadOnly => "readonly",
            Self::PlanWrite => "planwrite",
            Self::Guarded => "guarded",
            Self::Yolo => "yolo",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("policy I/O: {0}")]
    Io(String),
}

/// Object-safe gate trait. `PermissionChecker` is the v1 implementor.
pub trait Policy {
    fn check(&mut self, tool: &str, input: &str) -> Decision;
    fn check_path(&mut self, tool: &str, path: &str) -> Decision;
    fn allow_session(&mut self, tool: &str, pattern: &str);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_round_trip() {
        for (s, m) in [
            ("standard", SecurityMode::Standard),
            ("restrictive", SecurityMode::Restrictive),
            ("readonly", SecurityMode::ReadOnly),
            ("planwrite", SecurityMode::PlanWrite),
            ("guarded", SecurityMode::Guarded),
            ("yolo", SecurityMode::Yolo),
        ] {
            assert_eq!(SecurityMode::from_str(s), Some(m));
            assert_eq!(m.to_string(), s);
        }
        assert_eq!(SecurityMode::from_str("nope"), None);
    }

    #[test]
    fn action_semantics() {
        let a = Action::Ask;
        assert_eq!(a, Action::Ask);
        assert_ne!(Action::Allow, Action::Deny);
        let d = Decision::Deny("Blocked by deny rule".to_string());
        assert!(matches!(d, Decision::Deny(_)));
    }
}
