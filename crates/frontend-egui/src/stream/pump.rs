//! Stream pump: worker thread with its own tokio runtime.
//!
//! The UI thread never awaits: it `send()`s an [`AgentRun`](command_api::Command)
//! request and `drain()`s [`PumpEvent`]s once per frame, calling
//! `request_repaint()` ONLY while streaming.

use std::sync::Arc;
use std::thread::JoinHandle;

use command_api::{AgentRunArgs, Command};
use computer_agent_engine::{Engine, TurnEvent};
use tokio::sync::mpsc;

/// One UI-digestible stream outcome from the worker thread.
#[derive(Debug, Clone, PartialEq)]
pub enum PumpEvent {
    /// Assistant text delta.
    Chunk(String),
    /// Turn finished with the final reply.
    Done(String),
    /// Turn failed (policy deny, session I/O, …).
    Failed(String),
}

/// Work item for the pump thread.
#[derive(Debug, Clone)]
struct PumpRequest {
    prompt: String,
    session_id: Option<String>,
}

/// Worker-thread pump: owns the request/event channels and the thread.
pub struct Pump {
    req_tx: mpsc::UnboundedSender<PumpRequest>,
    ev_rx: mpsc::UnboundedReceiver<PumpEvent>,
    _worker: JoinHandle<()>,
}

impl Pump {
    /// Spawn the worker thread around a shared [`Engine`].
    pub fn new(engine: Arc<Engine>) -> Self {
        let (req_tx, mut req_rx) = mpsc::unbounded_channel::<PumpRequest>();
        let (ev_tx, ev_rx) = mpsc::unbounded_channel::<PumpEvent>();
        let worker = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("egui pump runtime");
            rt.block_on(async move {
                while let Some(req) = req_rx.recv().await {
                    run_one(&engine, &ev_tx, req).await;
                }
            });
        });
        Self {
            req_tx,
            ev_rx,
            _worker: worker,
        }
    }

    /// Queue a run (sync, never blocks the UI thread).
    pub fn send(&self, prompt: String, session_id: Option<String>) {
        let _ = self.req_tx.send(PumpRequest { prompt, session_id });
    }

    /// Collect every pending event (call once per frame from `update`).
    pub fn drain(&mut self) -> Vec<PumpEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.ev_rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

/// One turn: subscribe BEFORE dispatch, forward hub events, then settle.
async fn run_one(engine: &Engine, ev_tx: &mpsc::UnboundedSender<PumpEvent>, req: PumpRequest) {
    let mut sub = engine.events().subscribe(64);
    let result = engine
        .dispatch(Command::AgentRun(AgentRunArgs {
            prompt: req.prompt,
            session_id: req.session_id,
        }))
        .await;
    let mut saw_done = false;
    while let Ok(ev) = sub.try_recv() {
        let pump_ev = match ev {
            TurnEvent::Text(text) => PumpEvent::Chunk(text),
            TurnEvent::Done { response } => {
                saw_done = true;
                PumpEvent::Done(response)
            }
            TurnEvent::ToolCall { name, .. } => PumpEvent::Chunk(format!("⚙ {name}…\n")),
            TurnEvent::ToolResult { name, .. } => PumpEvent::Chunk(format!("⚙ {name} done\n")),
        };
        if ev_tx.send(pump_ev).is_err() {
            return;
        }
    }
    match result {
        Ok(_) => {
            if !saw_done {
                let _ = ev_tx.send(PumpEvent::Done(String::new()));
            }
        }
        Err(err) => {
            let _ = ev_tx.send(PumpEvent::Failed(err.to_string()));
        }
    }
}
