//! State math without egui boot: append/filter/usage rollups.

use computer_agent_frontend_egui::state::{ChatState, ThreadRow, UsageState};

fn state() -> ChatState {
    ChatState::new(
        vec!["claude-sonnet-4-6".to_string(), "gpt-4o".to_string()],
        "claude-sonnet-4-6",
    )
}

#[test]
fn user_and_streaming_round_trip() {
    let mut s = state();
    s.chat.push_user("hello");
    assert!(s.chat.draft.is_empty());
    assert_eq!(s.chat.messages().len(), 1);
    s.chat.begin_assistant();
    assert!(s.chat.sending);
    s.chat.append_chunk("hi ");
    s.chat.append_chunk("there");
    s.chat.finish_assistant("hi there");
    assert!(!s.chat.sending);
    assert_eq!(s.chat.messages().len(), 2);
    assert_eq!(s.chat.messages()[1].content, "hi there");
}

#[test]
fn search_filters_titles_case_insensitively() {
    let mut s = state();
    s.threads.set_rows(vec![
        ThreadRow::new("a", "Deploy checklist", "m"),
        ThreadRow::new("b", "weekend notes", "m"),
    ]);
    assert_eq!(s.threads.filtered().len(), 2);
    s.threads.search = "DEPLOY".to_string();
    let hits = s.threads.filtered();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "a");
    s.threads.remove("a");
    assert_eq!(s.threads.rows().len(), 1);
    s.threads.select("b");
    assert_eq!(s.threads.active_id.as_deref(), Some("b"));
    s.threads.remove("b");
    assert_eq!(s.threads.active_id, None);
}

#[test]
fn unknown_model_rejected() {
    let mut s = state();
    assert!(s.models.set_current("gpt-4o"));
    assert!(!s.models.set_current("nope-99"));
    assert_eq!(s.models.current, "gpt-4o");
}

#[test]
fn usage_fraction_clamps_and_labels() {
    let u = UsageState::new(64_000, 128_000, 42);
    assert!((u.fraction() - 0.5).abs() < f32::EPSILON);
    assert_eq!(u.ctx_label(), "64000 / 128000 tok");
    assert_eq!(u.cost_label(), "$0.4200");
    assert_eq!(UsageState::default().fraction(), 0.0);
    assert_eq!(UsageState::new(200, 100, 150).fraction(), 1.0);
    assert_eq!(UsageState::new(0, 0, 150).cost_label(), "$1.50");
}

#[test]
fn send_request_round_trip() {
    let mut s = state();
    s.chat.request_send();
    assert!(s.chat.take_send_request().is_none());
    s.chat.draft = "hi".to_string();
    s.chat.request_send();
    assert_eq!(s.chat.take_send_request().as_deref(), Some("hi"));
    assert!(s.chat.sending);
    assert_eq!(s.chat.messages().len(), 2);
    assert!(s.chat.take_send_request().is_none());
}

#[test]
fn errors_surface_as_banners_until_dismissed() {
    let mut s = state();
    s.status.set_err("boom");
    assert!(!s.status.ok);
    assert_eq!(s.status.banner_current(), Some("boom"));
    assert!(s.status.dismiss_banner());
    assert_eq!(s.status.banner_current(), None);
    assert!(!s.status.dismiss_banner());
    s.status.set_ok("ready");
    assert!(s.status.ok);
}
