//! Policy gate: allow / ask / deny verdicts over tool calls.
//!
//! Enforcement lives in `command-api::dispatch` / engine, never in UI.
//! Frontends only render [`Decision::Ask`] and return the user's answer.

pub mod checker;
pub mod decision;
pub mod defaults;
pub mod doom_loop;
pub mod pattern;
pub mod store;

pub use checker::PermissionChecker;
pub use decision::{Action, Decision, Policy, PolicyError, SecurityMode};
pub use doom_loop::DoomLoop;
pub use pattern::Pattern;
pub use store::SessionAllowlist;
