//! Palette: Ctrl+K command runner (new thread, clear chat).
//!
//! Modal overlay plug-in: fuzzy-filtered action list, Esc dismisses.
//! Actions run through the same [`Engine`](computer_agent_engine::Engine) +
//! [`ChatState`](crate::state::ChatState) seams as their sidebar twins.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, OverlayStack, SlotId};
use crate::state::ChatState;

/// Ctrl+K palette: filterable actions.
pub struct Palette {
    query: String,
}

impl Palette {
    /// Build the palette with an empty query.
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

/// One palette action: label + effect.
fn actions() -> Vec<(&'static str, fn(&mut ChatState, &Engine))> {
    vec![
        ("new thread", |state, engine| {
            if let Ok(thread) = engine.new_thread() {
                let id = thread.id.clone();
                state.refresh_from_engine(engine);
                state.threads.select(&id);
                state.chat.clear();
                state.refresh_usage(engine, Some(&id));
            }
        }),
        ("clear chat", |state, _| {
            state.chat.clear();
        }),
    ]
}

impl Component for Palette {
    fn id(&self) -> &'static str {
        "palette"
    }

    fn slot(&self) -> SlotId {
        SlotId::Feed
    }

    fn layer(&self) -> Layer {
        Layer::Overlay(OverlayKind::Modal)
    }

    fn anchor(&self) -> Anchor {
        Anchor::Center
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        state: &mut ChatState,
        engine: &Engine,
        overlays: &mut OverlayStack,
    ) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            overlays.dismiss_modal();
            return;
        }
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("type a command…"));
            let needle = self.query.trim().to_lowercase();
            let mut run: Option<usize> = None;
            for (idx, (label, _)) in actions().iter().enumerate().filter(|(_, (label, _))| {
                needle.is_empty() || label.to_lowercase().contains(&needle)
            }) {
                if ui.selectable_label(false, *label).clicked() {
                    run = Some(idx);
                }
            }
            if let Some(idx) = run {
                actions()[idx].1(state, engine);
                self.query.clear();
                overlays.dismiss_modal();
            }
        });
    }
}
