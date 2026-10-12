//! `computer-agent-frontend-slint`: Slint shell on [`command-api`].
//!
//! Real Slint dependency (`slint` + `slint-build`, UI in `ui/appwindow.slint`).
//! Window wiring lives in [`window`] ([`run_window`] opens the event loop);
//! headless dispatch proofs live in [`app`]; Slint-callback ->
//! [`Command`](command-api::Command) mapping lives in [`bridge`]. Policy is
//! enforced inside `dispatch`/engine, never here (see `docs/ARCHITECTURE.md`
//! §2).

pub mod app;
pub mod bridge;
pub mod markdown;
pub mod markdown_view;
pub mod viewmodel;
pub mod window;

pub use app::App;
pub use bridge::{callback_to_command, SlintCallback};

slint::include_modules!();
