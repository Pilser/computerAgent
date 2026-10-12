//! Provider dispatch: kind, config, payload planning.
//!
//! Slim port of vendor `provider.rs` (1562 lines) + `auth.rs`
//! (`ProviderKind`, key resolution) WITHOUT `rig` / `reqwest`: v1 makes no
//! network calls. Each vendor module builds its request payload as
//! `serde_json::Value`; [`plan`] turns a resolved config into a
//! [`StubPlanned`] (method + url + model) so the engine can log/replay.
//! Real HTTP execution arrives in a later milestone.

pub mod anthropic;
pub mod catalog;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod traits;

pub use catalog::{context_window, estimate_cost, lookup, ModelInfo};
pub use traits::{plan, ChatMessage, ProviderConfig, ProviderKind, StubPlanned};
