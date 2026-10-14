//! Component harness tests: headless logic plus image snapshots.
//!
//! Logic tests run anywhere (AccessKit tree, no renderer). Snapshot tests
//! compare against `tests/snapshots/*.png`; seed them once on a GPU box
//! with `UPDATE_SNAPSHOTS=true cargo test -p computer-agent-frontend-egui`
//! (see `egui_kittest` docs), then they pass everywhere.

use computer_agent_frontend_egui::components::composer::Composer;
use computer_agent_frontend_egui::components::feed::Feed;
use computer_agent_frontend_egui::shell::{Allocated, Component, SlotId};
use computer_agent_frontend_egui::state::ChatState;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};

fn state() -> ChatState {
    ChatState::new(vec!["model-a".to_string()], "model-a")
}

fn feed_state() -> ChatState {
    let mut s = state();
    s.chat.push_user("hello **world**");
    s.chat
        .finish_assistant("# Hi\n\nsome `code` here\n\n```rs\nlet x = 1;\n```\n\n- a\n- b\n");
    s
}

#[test]
fn composer_send_marks_request() {
    let mut s = state();
    s.chat.draft = "hello".to_string();
    let mut composer = Composer::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            composer.show(ui, Allocated::new(SlotId::Composer, ui.max_rect()), state);
        },
        s,
    );
    harness.set_size(egui::Vec2::new(800.0, 300.0));
    harness.get_by_label("Send").click();
    harness.run();
    assert!(harness.state().chat.send_requested);
}

#[test]
fn composer_blank_draft_sends_nothing() {
    let s = state();
    let mut composer = Composer::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            composer.show(ui, Allocated::new(SlotId::Composer, ui.max_rect()), state);
        },
        s,
    );
    harness.set_size(egui::Vec2::new(800.0, 300.0));
    harness.run();
    assert!(!harness.state().chat.send_requested);
}

#[test]
fn feed_renders_bubbles() {
    let mut s = feed_state();
    let mut feed = Feed::new();
    let mut harness = Harness::new_ui(|ui: &mut egui::Ui| {
        feed.show(ui, Allocated::new(SlotId::Feed, ui.max_rect()), &mut s);
    });
    harness.set_size(egui::Vec2::new(800.0, 600.0));
    harness.run();
    harness.snapshot("feed");
}

#[test]
fn composer_snapshot() {
    let mut s = state();
    s.chat.draft = "hello".to_string();
    let mut composer = Composer::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            composer.show(ui, Allocated::new(SlotId::Composer, ui.max_rect()), state);
        },
        s,
    );
    harness.set_size(egui::Vec2::new(800.0, 300.0));
    harness.run();
    harness.snapshot("composer");
}
