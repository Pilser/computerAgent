//! Real Slint window: `AppWindow` from `ui/appwindow.slint`.
//!
//! [`run_window`] blocks on the Slint event loop (call it from `main`, never
//! from async code or tests). The `send` callback hands the prompt to a worker
//! thread that drives [`run_agent`](crate::bridge::run_agent) on a
//! single-threaded Tokio runtime, then posts the result back with
//! `invoke_from_event_loop`. Policy stays inside engine, never here.

use std::sync::Arc;

use anyhow::Context;
use computer_agent_engine::Engine;

slint::include_modules!();

/// Open the assistant window. Blocks until the window closes.
///
/// # Errors
///
/// [`anyhow::Error`] when the Slint backend fails to start or the event loop
/// returns an error.
pub fn run_window(engine: Arc<Engine>) -> anyhow::Result<()> {
    let ui = AppWindow::new().context("create AppWindow")?;
    let weak = ui.as_weak();

    ui.on_send(move |prompt| {
        let weak = weak.clone();
        let engine = engine.clone();
        std::thread::spawn(move || {
            let text = run_prompt(&engine, &prompt);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_result(text.into());
                }
            });
        });
    });

    ui.run().context("Slint event loop")?;
    Ok(())
}

fn run_prompt(engine: &Engine, prompt: &str) -> String {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => return format!("error: runtime: {err:#}"),
    };
    match runtime.block_on(crate::bridge::run_agent(engine, prompt, None)) {
        Ok(result) => format!("{result:?}"),
        Err(err) => format!("error: {err:#}"),
    }
}
