//! Component harness tests: headless logic plus image snapshots.
//!
//! Logic tests run anywhere (AccessKit tree, no renderer). Snapshot tests
//! compare against `tests/snapshots/*.png`; seed them once on a GPU box
//! with `UPDATE_SNAPSHOTS=true cargo test -p computer-agent-frontend-egui`
//! (see `egui_kittest` docs), then they pass everywhere.

use computer_agent_engine::Engine;
use computer_agent_frontend_egui::components::banner::Banner;
use computer_agent_frontend_egui::components::composer::Composer;
use computer_agent_frontend_egui::components::feed::Feed;
use computer_agent_frontend_egui::components::overlays::approval::Approval;
use computer_agent_frontend_egui::components::overlays::menu::Menu;
use computer_agent_frontend_egui::components::overlays::palette::Palette;
use computer_agent_frontend_egui::components::overlays::toast::Toast;
use computer_agent_frontend_egui::components::sidebar::Sidebar;
use computer_agent_frontend_egui::components::topbar::Topbar;
use computer_agent_frontend_egui::shell::{Allocated, Component, OverlayStack, Shell, SlotId};
use computer_agent_frontend_egui::state::{ChatState, ThreadRow, UsageState};
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
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            composer.show(
                ui,
                Allocated::new(SlotId::Composer, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
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
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            composer.show(
                ui,
                Allocated::new(SlotId::Composer, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
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
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui(|ui: &mut egui::Ui| {
        feed.show(
            ui,
            Allocated::new(SlotId::Feed, ui.max_rect()),
            &mut s,
            &engine,
            &mut overlays,
        );
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
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            composer.show(
                ui,
                Allocated::new(SlotId::Composer, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(800.0, 300.0));
    harness.run();
    harness.snapshot("composer");
}

#[test]
fn sidebar_snapshot() {
    let mut s = state();
    s.threads.set_rows(vec![
        ThreadRow::new("id-1", "Deploy checklist", "model-a"),
        ThreadRow::new("id-2", "weekend notes", "model-a"),
    ]);
    s.threads.select("id-1");
    let mut sidebar = Sidebar::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            sidebar.show(
                ui,
                Allocated::new(SlotId::Sidebar, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(300.0, 600.0));
    harness.run();
    harness.snapshot("sidebar");
}

#[test]
fn topbar_snapshot() {
    let mut s = state();
    s.usage = UsageState::new(64_000, 128_000, 42);
    let mut topbar = Topbar::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            topbar.show(
                ui,
                Allocated::new(SlotId::Topbar, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(800.0, 48.0));
    harness.run();
    harness.snapshot("topbar");
}

#[test]
fn banner_snapshot() {
    let mut s = state();
    s.status.set_err("boom");
    let mut banner = Banner::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            banner.show(
                ui,
                Allocated::new(SlotId::Banner, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(800.0, 96.0));
    harness.run();
    harness.snapshot("banner");
}

#[test]
fn toast_snapshot() {
    let s = state();
    let mut toast = Toast::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    overlays.push_toast("copied message", std::time::Duration::from_secs(60));
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            toast.show(
                ui,
                Allocated::new(SlotId::Feed, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(400.0, 200.0));
    harness.run();
    harness.snapshot("toast");
}

#[test]
fn approval_snapshot() {
    let mut s = state();
    s.approval.request("bash", "rm -rf /tmp/x");
    let mut approval = Approval::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    overlays.show_modal("approval");
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            approval.show(
                ui,
                Allocated::new(SlotId::Feed, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(500.0, 300.0));
    harness.run();
    harness.snapshot("approval");
}

#[test]
fn palette_snapshot() {
    let s = state();
    let mut palette = Palette::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    overlays.show_modal("palette");
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            palette.show(
                ui,
                Allocated::new(SlotId::Feed, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(500.0, 300.0));
    harness.run();
    harness.snapshot("palette");
}

#[test]
fn menu_snapshot() {
    let mut s = state();
    s.chat.push_user("hello world, copy me");
    s.chat.menu_message = Some(0);
    let mut menu = Menu::new();
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut overlays = OverlayStack::new();
    overlays.show_menu("menu");
    let mut harness = Harness::new_ui_state(
        |ui: &mut egui::Ui, state: &mut ChatState| {
            menu.show(
                ui,
                Allocated::new(SlotId::Feed, ui.max_rect()),
                state,
                &engine,
                &mut overlays,
            );
        },
        s,
    );
    harness.set_size(egui::Vec2::new(400.0, 200.0));
    harness.run();
    harness.snapshot("menu");
}

#[test]
fn shell_snapshot() {
    let mut s = feed_state();
    s.threads
        .set_rows(vec![ThreadRow::new("id-1", "Deploy checklist", "model-a")]);
    s.threads.select("id-1");
    s.usage = UsageState::new(1_000, 128_000, 3);
    let engine = Engine::with_store_dir(std::path::PathBuf::from("/tmp/opencode/egui-snapshots"));
    let mut shell = Shell::new();
    let mut harness = Harness::new_ui(|ui: &mut egui::Ui| {
        shell.run(ui, &mut s, &engine);
    });
    harness.set_size(egui::Vec2::new(1280.0, 800.0));
    harness.run();
    harness.snapshot("shell");
}
