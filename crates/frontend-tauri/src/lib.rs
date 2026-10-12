//! `computer-agent-frontend-tauri`: thin Tauri shell on [`command-api`].
//!
//! No `tauri` crate dependency yet (shell only, GUI deps land later).
//! Frontend-callable commands live in [`commands`]; the web-view contract
//! (same `Command` JSON, no TS build in v1) lives in [`web`]. Policy is
//! enforced inside `dispatch`/engine, never here
//! (see `docs/ARCHITECTURE.md` §2).

pub mod commands;
pub mod web;

pub use commands::{agent_run, agent_run_command, system_info, system_info_command};
