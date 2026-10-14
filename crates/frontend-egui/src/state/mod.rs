//! All UI state in one place (egui-free, tested without egui boot).
//!
//! Narrow slices (`&chat`, `&threads`, …) flow into components; only this
//! facade talks to the [`Engine`](computer_agent_engine::Engine).

pub mod chat;
pub mod models;
pub mod status;
pub mod threads;
pub mod usage;

pub use chat::{ChatLog, Message};
pub use models::ModelState;
pub use status::StatusState;
pub use threads::{ThreadList, ThreadRow};
pub use usage::UsageState;

use computer_agent_engine::Engine;

/// Facade over every state slice: chat, threads, models, usage, status.
#[derive(Debug)]
pub struct ChatState {
    /// Bubbles + draft + sending flag.
    pub chat: ChatLog,
    /// Sidebar rows + selection + search.
    pub threads: ThreadList,
    /// Model catalog + selection.
    pub models: ModelState,
    /// Context bar + cost for the active thread.
    pub usage: UsageState,
    /// Status line + banner queue.
    pub status: StatusState,
}

impl ChatState {
    /// Fresh state from an engine catalog snapshot.
    pub fn new(models: Vec<String>, current: &str) -> Self {
        Self {
            chat: ChatLog::new(),
            threads: ThreadList::new(),
            models: ModelState::new(models, current),
            usage: UsageState::default(),
            status: StatusState::new(),
        }
    }

    /// Pull sidebar rows + model selection from the engine.
    pub fn refresh_from_engine(&mut self, engine: &Engine) {
        self.threads.set_rows(
            engine
                .list_threads(computer_agent_engine::THREAD_LIMIT)
                .iter()
                .map(|t| ThreadRow::new(&t.id, &t.title, &t.model))
                .collect(),
        );
        let current = engine.current_model();
        self.models.models = Engine::chat_models();
        self.models.current = current;
    }

    /// Pull the context bar + cost for `thread_id` (`None` clears to zero).
    pub fn refresh_usage(&mut self, engine: &Engine, thread_id: Option<&str>) {
        match thread_id.and_then(|id| engine.thread_usage(id)) {
            Some(u) => {
                self.usage = UsageState::new(u.used, u.window, u.cost_cents);
            }
            None => self.usage = UsageState::default(),
        }
    }
}
