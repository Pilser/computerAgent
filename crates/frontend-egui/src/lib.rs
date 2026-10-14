//! egui frontend: immediate-mode shell + slot-isolated components.
//!
//! Same engine, same `command-api` seams, same warm theme as the Slint
//! frontend (which keeps shipping until screenshot parity). See
//! `ARCHITECTURE.md` for the shell/slot/overlay contract.
//!
//! Re-export only; see [`app`], [`components`], [`markdown`], [`shell`],
//! [`state`], [`stream`], [`theme`].

pub mod app;
pub mod components;
pub mod markdown;
pub mod shell;
pub mod state;
pub mod stream;
pub mod theme;

pub use app::ComputerAgentApp;
