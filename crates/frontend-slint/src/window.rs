//! Real Slint window: `AppWindow` from `ui/appwindow.slint`.
//!
//! [`run_window`] blocks on the Slint event loop (call it from `main`, never
//! from async code or tests). State lives on the `AppState` global; each
//! callback hands work to a worker thread that drives the [`Engine`] on a
//! single-threaded Tokio runtime, then posts results back with
//! `invoke_from_event_loop`. Policy stays inside engine, never here.
//! Markdown rendering and the Ask-approval dialog are later milestones.

use std::sync::Arc;

use anyhow::Context;
use command_api::{AgentRunArgs, Command};
use computer_agent_engine::{Engine, ThreadInfo as EngineThread, TurnEvent};
use slint::{ComponentHandle, Model, SharedString, Weak};

use crate::viewmodel::{str_vec, ui_messages, ui_thread, user_msg, vecmodel};
use crate::{AppState, AppWindow, ChatMsg};

/// Open the assistant window. Blocks until the window closes.
///
/// # Errors
///
/// [`anyhow::Error`] when the Slint backend or event loop fails.
pub fn run_window(engine: Arc<Engine>) -> anyhow::Result<()> {
    let ui = AppWindow::new().context("create AppWindow")?;
    {
        let state = ui.global::<AppState>();
        state.set_models(str_vec(Engine::chat_models()));
        state.set_current_model(engine.current_model().into());
        refresh_threads(&ui, &engine);
        match engine.list_threads(1).first() {
            Some(recent) => open_thread(&ui, &engine, &recent.id.clone()),
            None => set_status(&ui, true, "New chat to begin.".to_string()),
        }
    }
    wire(&ui, &engine);
    ui.run().context("Slint event loop")?;
    Ok(())
}

/// Attach every `AppState` callback to its engine worker.
fn wire(ui: &AppWindow, engine: &Arc<Engine>) {
    let state = ui.global::<AppState>();

    state.on_send_message({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move |text: SharedString| on_send(&weak, &engine, text)
    });
    state.on_new_thread({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move || rerun(&weak, |ui| on_new(ui, &engine))
    });
    state.on_open_thread({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move |id: SharedString| rerun(&weak, |ui| open_thread(ui, &engine, &id))
    });
    state.on_delete_thread({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move |id: SharedString| rerun(&weak, |ui| on_delete(ui, &engine, &id))
    });
    state.on_set_model({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move |id: SharedString| rerun(&weak, |ui| on_set_model(ui, &engine, &id))
    });
    state.on_cycle_model({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move || rerun(&weak, |ui| on_cycle_model(ui, &engine))
    });
    state.on_stop_run({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move || rerun(&weak, |ui| on_stop(ui, &engine))
    });
    state.on_search_changed({
        let (engine, weak) = (engine.clone(), ui.as_weak());
        move |_: SharedString| rerun(&weak, |ui| refresh_threads(ui, &engine))
    });
}

/// Run `f` on the upgraded handle (drop silently when closed).
fn rerun(weak: &Weak<AppWindow>, f: impl FnOnce(&AppWindow)) {
    if let Some(ui) = weak.upgrade() {
        f(&ui);
    }
}

/// Send the composer text through one engine turn on a worker thread.
fn on_send(weak: &Weak<AppWindow>, engine: &Arc<Engine>, text: SharedString) {
    let prompt = text.as_str().trim().to_string();
    if prompt.is_empty() {
        return;
    }
    let Some(ui) = weak.upgrade() else { return };
    let tid = ensure_thread(&ui, engine);
    let Some(tid) = tid else { return };
    push_local(&ui, user_msg(&prompt));
    set_sending(&ui, true, true, "Thinking…");
    let (weak, engine) = (weak.clone(), engine.clone());
    std::thread::spawn(move || {
        let outcome = run_turn(&engine, &tid, &prompt);
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                match outcome {
                    Ok(chunks) => {
                        open_thread(&ui, &engine, &tid);
                        set_sending(&ui, false, true, &format!("Done · {chunks} streamed."));
                    }
                    Err(err) => set_sending(&ui, false, false, &format!("error: {err}")),
                }
            }
        });
    });
}

/// Active thread id, creating a fresh thread for the first-ever send.
fn ensure_thread(ui: &AppWindow, engine: &Engine) -> Option<String> {
    let state = ui.global::<AppState>();
    let active = state.get_active_id().to_string();
    if !active.is_empty() {
        return Some(active);
    }
    match engine.new_thread() {
        Ok(t) => {
            state.set_search(SharedString::new());
            refresh_threads(ui, engine);
            state.set_active_id(t.id.clone().into());
            Some(t.id)
        }
        Err(err) => {
            set_status(&ui, false, format!("error: new thread: {err:#}"));
            None
        }
    }
}

/// One turn: subscribe first, dispatch, then drain the stub's sync events.
fn run_turn(engine: &Engine, tid: &str, prompt: &str) -> Result<usize, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("runtime: {err:#}"))?;
    let mut rx = engine.events().subscribe(64);
    let result = runtime.block_on(engine.dispatch(Command::AgentRun(AgentRunArgs {
        prompt: prompt.to_string(),
        session_id: Some(tid.to_string()),
    })));
    let mut chunks = 0;
    while let Ok(event) = rx.try_recv() {
        if matches!(event, TurnEvent::Text(_)) {
            chunks += 1;
        }
    }
    result.map(|_| chunks).map_err(|err| format!("{err:#}"))
}

fn on_new(ui: &AppWindow, engine: &Engine) {
    let state = ui.global::<AppState>();
    match engine.new_thread() {
        Ok(t) => {
            state.set_search(SharedString::new());
            refresh_threads(ui, engine);
            open_thread(ui, engine, &t.id);
            set_status(ui, true, "New thread.".to_string());
        }
        Err(err) => set_status(ui, false, format!("error: new thread: {err:#}")),
    }
}

fn on_delete(ui: &AppWindow, engine: &Engine, id: &str) {
    let state = ui.global::<AppState>();
    let was_active = state.get_active_id().as_str() == id;
    match engine.delete_thread(id) {
        Ok(()) => {
            state.set_search(SharedString::new());
            refresh_threads(ui, engine);
            if was_active {
                match engine.list_threads(1).first() {
                    Some(next) => open_thread(ui, engine, &next.id.clone()),
                    None => reset_view(ui, "Thread deleted."),
                }
            }
        }
        Err(err) => set_status(ui, false, format!("error: delete: {err:#}")),
    }
}

fn on_set_model(ui: &AppWindow, engine: &Engine, id: &str) {
    let state = ui.global::<AppState>();
    match engine.set_model(id) {
        Ok(()) => {
            state.set_current_model(engine.current_model().into());
            set_status(ui, true, format!("Model {}.", engine.current_model()));
        }
        Err(err) => set_status(ui, false, format!("error: {err:#}")),
    }
}

fn on_cycle_model(ui: &AppWindow, engine: &Engine) {
    let models = Engine::chat_models();
    let current = engine.current_model();
    let next = models
        .iter()
        .position(|m| *m == current)
        .and_then(|i| models.get((i + 1) % models.len()))
        .cloned()
        .unwrap_or(current);
    on_set_model(ui, engine, &next);
}

fn on_stop(ui: &AppWindow, engine: &Engine) {
    let live = engine.cancel_all();
    set_status(ui, true, format!("Stop requested · {live} in flight."));
}

/// Reload the sidebar, honoring the search box (Rust-side filter: Slint has
/// no substring match, so the model is pre-filtered here).
fn refresh_threads(ui: &AppWindow, engine: &Engine) {
    let state = ui.global::<AppState>();
    let query = state.get_search().trim().to_lowercase();
    let shown: Vec<EngineThread> = engine
        .list_threads(100)
        .into_iter()
        .filter(|t| {
            query.is_empty()
                || t.title.to_lowercase().contains(&query)
                || t.model.to_lowercase().contains(&query)
                || t.id.to_lowercase().contains(&query)
        })
        .collect();
    state.set_threads(vecmodel(shown.iter().map(ui_thread).collect()));
}

/// Load a thread into the chat view + context bar.
fn open_thread(ui: &AppWindow, engine: &Engine, id: &str) {
    let state = ui.global::<AppState>();
    match engine.open_thread(id) {
        Some(session) => {
            state.set_active_id(id.into());
            state.set_messages(vecmodel(ui_messages(&session)));
            apply_usage(ui, engine, id);
            let title: String = session.name.chars().take(40).collect();
            set_status(
                ui,
                true,
                format!(
                    "{} · {} msgs · {}",
                    title,
                    session.messages.len(),
                    session.model
                ),
            );
        }
        None => set_status(ui, false, "Thread not found.".to_string()),
    }
}

/// Empty chat view (no threads left).
fn reset_view(ui: &AppWindow, status: &str) {
    let state = ui.global::<AppState>();
    state.set_active_id(SharedString::new());
    state.set_messages(vecmodel(Vec::new()));
    state.set_ctx_used(0);
    state.set_ctx_window(128_000);
    state.set_ctx_label("0 / 128000".into());
    state.set_cost_label("$0.00".into());
    set_status(ui, true, status.to_string());
}

/// Context bar: per-thread usage + total spend.
fn apply_usage(ui: &AppWindow, engine: &Engine, id: &str) {
    let state = ui.global::<AppState>();
    if let Some(usage) = engine.thread_usage(id) {
        let used = usage.used.min(i32::MAX as u64) as i32;
        let window = usage.window.min(i32::MAX as u64) as i32;
        state.set_ctx_used(used);
        state.set_ctx_window(window);
        state.set_ctx_label(format!("{used} / {window}").into());
        state.set_cost_label(
            format!(
                "${:.2} · total ${:.4}",
                usage.cost_cents as f64 / 100.0,
                engine.total_cost()
            )
            .into(),
        );
    }
}

fn push_local(ui: &AppWindow, msg: ChatMsg) {
    let state = ui.global::<AppState>();
    let mut msgs: Vec<ChatMsg> = state.get_messages().iter().collect();
    msgs.push(msg);
    state.set_messages(vecmodel(msgs));
}

fn set_sending(ui: &AppWindow, sending: bool, ok: bool, status: &str) {
    let state = ui.global::<AppState>();
    state.set_sending(sending);
    state.set_status_ok(ok);
    state.set_status_text(status.into());
}

/// Set status text + banner flag together (banner shows on `!ok`).
fn set_status(ui: &AppWindow, ok: bool, text: String) {
    let state = ui.global::<AppState>();
    state.set_status_ok(ok);
    state.set_status_text(text.into());
}
