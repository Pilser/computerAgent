//! Slint view-model mapping: engine/session values -> generated UI types.
//!
//! Pure functions only (no policy, no I/O); [`super::window`] calls these to
//! fill `AppState` arrays.

use std::rc::Rc;

use computer_agent_engine::ThreadInfo as EngineThread;
use computer_agent_session_store::session::{MessageRole, Session};
use slint::{ModelRc, SharedString, VecModel};

use crate::{ChatMsg, ThreadInfo};

/// Fresh user bubble with an estimated-token footnote.
pub fn user_msg(prompt: &str) -> ChatMsg {
    let est = Session::estimate_tokens(prompt);
    ChatMsg {
        role: "user".into(),
        text: prompt.into(),
        input_tokens: clamp_tokens(est),
        output_tokens: 0,
        tokens: format!("↑ {est} tok").into(),
    }
}

/// Whole session history, oldest first, for the bubble list.
pub fn ui_messages(session: &Session) -> Vec<ChatMsg> {
    session
        .messages
        .iter()
        .map(|m| {
            let est = clamp_tokens(m.estimated_tokens);
            let (role, input_tokens, output_tokens, tokens) = match m.role {
                MessageRole::User => ("user", est, 0, format!("↑ {est} tok")),
                MessageRole::Assistant => ("assistant", 0, est, format!("↓ {est} tok")),
                _ => ("tool", est, 0, format!("⚙ {est} tok")),
            };
            ChatMsg {
                role: role.into(),
                text: m.content.clone().into(),
                input_tokens,
                output_tokens,
                tokens: tokens.into(),
            }
        })
        .collect()
}

/// Sidebar row for one engine thread.
pub fn ui_thread(t: &EngineThread) -> ThreadInfo {
    ThreadInfo {
        id: t.id.clone().into(),
        title: t.title.clone().into(),
        model: t.model.clone().into(),
        input_tokens: clamp_tokens(t.input_tokens),
        output_tokens: clamp_tokens(t.output_tokens),
        cost_cents: t.cost_cents,
        cost: t.cost_label.clone().into(),
        stats: format!("↑{} ↓{} tok · {}", t.input_tokens, t.output_tokens, t.id).into(),
        updated_at: t.updated_at.clone().into(),
    }
}

/// `[string]` Slint model from Rust strings.
pub fn str_vec(items: Vec<String>) -> ModelRc<SharedString> {
    vecmodel(items.into_iter().map(SharedString::from).collect())
}

/// Slint array model from a Rust vec.
pub fn vecmodel<T: 'static + Clone>(items: Vec<T>) -> ModelRc<T> {
    Rc::new(VecModel::from(items)).into()
}

fn clamp_tokens(tokens: u64) -> i32 {
    tokens.min(i32::MAX as u64) as i32
}
