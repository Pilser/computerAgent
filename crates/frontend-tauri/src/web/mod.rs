//! Web-view contract: the JSON shape the future TS side sends.
//!
//! Re-export only; the wire format helpers live in [`shape`]. The contract
//! is byte-for-byte the [`Command`](command-api::Command) serde form
//! (adjacently-tagged `{"type": ..., "data": ...}`), so the web view can
//! later reuse the same payloads over Tauri IPC / HTTP with no frontend
//! rewrite. No TS build in v1.

pub mod shape;

pub use shape::{command_from_json, command_to_json, COMMAND_SCHEMA_NOTE};
