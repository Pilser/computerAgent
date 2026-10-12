//! `computer-agent-frontend-slint`: thin Slint shell on [`command-api`].
//!
//! No Slint dependency yet (GUI deps land later). Window wiring lives in
//! [`app`]; Slint-callback -> [`Command`](command-api::Command) mapping lives
//! in [`bridge`]. Policy is enforced inside `dispatch`/engine, never here
//! (see `docs/ARCHITECTURE.md` §2).

pub mod app;
pub mod bridge;

pub use app::App;
pub use bridge::{callback_to_command, SlintCallback};
