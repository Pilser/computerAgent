//! `computer-agent`: the warm Slint chat window on a fresh [`Engine`].
//!
//! Thin binary only: it builds one shared engine handle and hands it to
//! [`frontend-slint`](computer_agent_frontend_slint)'s window loop. Policy
//! stays inside the engine; markdown/mermaid rendering stays in the
//! frontend crate (see `docs/ARCHITECTURE.md` §2).

use std::sync::Arc;

use computer_agent_engine::Engine;

fn main() -> anyhow::Result<()> {
    let engine = Arc::new(Engine::new());
    computer_agent_frontend_slint::window::run_window(engine)
}
