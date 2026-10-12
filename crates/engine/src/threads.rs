//! Chat threads for the Slint frontend: list/open/new/delete + model + usage.
//!
//! Thin adapters over `session-store` storage + the `agent` provider catalog:
//! no policy, no runner here. Markdown rendering and the Ask-approval dialog
//! are later milestones (not here).

use computer_agent_agent::provider::catalog;
use computer_agent_session_store::session::storage::{
    delete_session_in, find_recent_sessions_in, load_session_in, save_session_in,
};
use computer_agent_session_store::session::Session;

use crate::error::EngineError;
use crate::Engine;

/// Default model id (always present in the provider catalog).
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
/// Context window assumed when the model is not cataloged.
pub const FALLBACK_CONTEXT_WINDOW: u64 = 128_000;
/// Upper bound the sidebar ever pulls in one shot.
pub const THREAD_LIMIT: usize = 100;

/// One sidebar row: identity + model + token/cost totals + recency.
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    pub id: String,
    pub title: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_cents: i32,
    /// Preformatted dollars (`$0.0042`); Slint formats floats poorly.
    pub cost_label: String,
    pub updated_at: String,
}

/// Context-bar snapshot for one thread.
#[derive(Debug, Clone, Copy)]
pub struct ThreadUsage {
    pub used: u64,
    pub window: u64,
    pub cost_cents: i32,
}

impl ThreadInfo {
    fn from_session(s: &Session) -> Self {
        let title = if s.name.trim().is_empty() {
            s.id.chars().take(8).collect()
        } else {
            s.name.clone()
        };
        let cost_cents = (s.total_cost * 100.0).round() as i32;
        Self {
            id: s.id.clone(),
            title,
            model: s.model.clone(),
            input_tokens: s.total_input_tokens,
            output_tokens: s.total_output_tokens,
            cost_cents,
            cost_label: format_cost(s.total_cost),
            updated_at: s.updated_at.clone(),
        }
    }
}

/// `$d.cc` with 4 decimals while sub-dollar (stub turns cost fractions).
fn format_cost(dollars: f64) -> String {
    if dollars < 1.0 {
        format!("${dollars:.4}")
    } else {
        format!("${dollars:.2}")
    }
}

impl Engine {
    /// Model ids the UI may offer (provider catalog order, 12 entries).
    #[must_use]
    pub fn chat_models() -> Vec<String> {
        catalog::MODELS.iter().map(|m| m.id.to_string()).collect()
    }

    /// Currently selected model id.
    #[must_use]
    pub fn current_model(&self) -> String {
        self.current_model.lock().expect("model lock").clone()
    }

    /// Select the model for future threads/turns. Rejects unknown ids.
    ///
    /// # Errors
    ///
    /// [`EngineError::InvalidArgs`] when `model` is not in the catalog.
    pub fn set_model(&self, model: &str) -> Result<(), EngineError> {
        let id = model.trim();
        if catalog::MODELS.iter().any(|m| m.id == id) {
            *self.current_model.lock().expect("model lock") = id.to_string();
            Ok(())
        } else {
            Err(EngineError::InvalidArgs(format!("unknown model '{id}'")))
        }
    }

    /// Most-recent threads first, at most `limit` (clamped to [`THREAD_LIMIT`]).
    #[must_use]
    pub fn list_threads(&self, limit: usize) -> Vec<ThreadInfo> {
        find_recent_sessions_in(&self.store_dir, limit.min(THREAD_LIMIT))
            .unwrap_or_default()
            .iter()
            .map(ThreadInfo::from_session)
            .collect()
    }

    /// Full session for the chat view (`None` when unknown/unreadable).
    #[must_use]
    pub fn open_thread(&self, id: &str) -> Option<Session> {
        load_session_in(&self.store_dir, id).ok().flatten()
    }

    /// Create a thread on the current model and return its sidebar row.
    ///
    /// # Errors
    ///
    /// [`EngineError::Session`] when the session JSON cannot be saved.
    pub fn new_thread(&self) -> Result<ThreadInfo, EngineError> {
        let model = self.current_model();
        let (provider, window) = catalog::MODELS
            .iter()
            .find(|m| m.id == model)
            .map(|m| (m.provider, u64::from(m.context_window)))
            .unwrap_or(("chat", FALLBACK_CONTEXT_WINDOW));
        let session = Session::new(provider, &model, window, "New chat");
        save_session_in(&self.store_dir, &session)?;
        Ok(ThreadInfo::from_session(&session))
    }

    /// Delete a thread file (missing ids are a no-op success).
    ///
    /// # Errors
    ///
    /// [`EngineError::Session`] on I/O failure.
    pub fn delete_thread(&self, id: &str) -> Result<(), EngineError> {
        delete_session_in(&self.store_dir, id)?;
        Ok(())
    }

    /// Context-bar snapshot (`None` when the thread is unknown).
    #[must_use]
    pub fn thread_usage(&self, id: &str) -> Option<ThreadUsage> {
        self.open_thread(id).map(|s| ThreadUsage {
            used: s.effective_context_tokens(),
            window: if s.context_window == 0 {
                FALLBACK_CONTEXT_WINDOW
            } else {
                s.context_window
            },
            cost_cents: (s.total_cost * 100.0).round() as i32,
        })
    }

    /// Summed spend across recent threads (sidebar total).
    #[must_use]
    pub fn total_cost(&self) -> f64 {
        find_recent_sessions_in(&self.store_dir, THREAD_LIMIT)
            .unwrap_or_default()
            .iter()
            .map(|s| s.total_cost)
            .sum()
    }

    /// Flip every in-flight cancel flag (Stop button path). The stub runner
    /// is synchronous so this is a no-op today; the async runner will poll.
    pub fn cancel_all(&self) -> usize {
        self.cancels.cancel_all()
    }
}

/// First line of the prompt, capped — titles fresh threads.
pub(crate) fn thread_title(prompt: &str) -> String {
    let first = prompt.lines().next().unwrap_or("").trim();
    let short: String = first.chars().take(42).collect();
    if first.chars().count() > 42 {
        format!("{short}…")
    } else if short.is_empty() {
        "New chat".to_string()
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::path::PathBuf::from("/tmp/opencode/engine-threads").join(name);
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp dir");
        d
    }

    #[test]
    fn catalog_lists_twelve_models_with_default_first() {
        assert_eq!(Engine::chat_models().len(), 12);
        assert!(Engine::chat_models().contains(&DEFAULT_MODEL.to_string()));
    }

    #[test]
    fn set_model_rejects_unknown_ids() {
        let e = Engine::with_store_dir(dir("model"));
        assert!(e.set_model("gpt-99").is_err());
        e.set_model(DEFAULT_MODEL).expect("known model");
        assert_eq!(e.current_model(), DEFAULT_MODEL);
    }

    #[test]
    fn new_list_delete_round_trip() {
        let e = Engine::with_store_dir(dir("roundtrip"));
        let t = e.new_thread().expect("new thread");
        assert_eq!(t.title, "New chat");
        assert_eq!(e.list_threads(10).len(), 1);
        assert!(e.open_thread(&t.id).is_some());
        assert!(e.thread_usage(&t.id).is_some());
        e.delete_thread(&t.id).expect("delete");
        assert!(e.list_threads(10).is_empty());
    }

    #[test]
    fn titles_cap_at_first_line() {
        assert_eq!(thread_title("hello"), "hello");
        assert!(thread_title(&"x".repeat(100)).chars().count() <= 43);
        assert_eq!(thread_title(""), "New chat");
    }
}
