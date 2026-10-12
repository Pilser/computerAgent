//! `computer-agent-frontend-tauri`: Tauri shell on [`command-api`].
//!
//! The real `tauri` dependency lives behind `--features tauri-app`
//! ([`app::run_app`], `src-tauri/tauri.conf.json`, `web/index.html`) because
//! it needs system webkit this box lacks — see `docs/ARCHITECTURE.md`.
//! Frontend-callable commands live in [`commands`]; the web-view contract
//! (same `Command` JSON) lives in [`web`]. Policy is enforced inside
//! `dispatch`/engine, never here (see `docs/ARCHITECTURE.md` §2).

pub mod app;
pub mod commands;
pub mod web;

pub use commands::{agent_run, agent_run_command, system_info, system_info_command};

// `tauri::State<Arc<Engine>>` requires `Engine: Send + Sync`. Assert it here,
// on the always-compiled path, so a regression breaks fast locally instead of
// in CI under `--features tauri-app`.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<computer_agent_engine::Engine>();
};
