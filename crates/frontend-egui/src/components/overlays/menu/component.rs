//! Menu: message context menu (anchored overlay).
//!
//! The feed's "…" button sets [`ChatLog::menu_message`](crate::state::ChatLog)
//! and opens this overlay; Copy duplicates the message to the clipboard and
//! confirms with a toast.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, OverlayStack, SlotId};
use crate::state::ChatState;

/// Message menu: snippet + copy action for the targeted bubble.
pub struct Menu;

impl Menu {
    /// Build the menu.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Menu {
    fn id(&self) -> &'static str {
        "menu"
    }

    fn slot(&self) -> SlotId {
        SlotId::Feed
    }

    fn layer(&self) -> Layer {
        Layer::Overlay(OverlayKind::Menu)
    }

    fn anchor(&self) -> Anchor {
        Anchor::Cursor
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        state: &mut ChatState,
        _engine: &Engine,
        overlays: &mut OverlayStack,
    ) {
        let Some(idx) = state.chat.menu_message else {
            overlays.dismiss_menu();
            return;
        };
        let Some(msg) = state.chat.messages().get(idx).cloned() else {
            state.chat.menu_message = None;
            overlays.dismiss_menu();
            return;
        };
        ui.vertical(|ui| {
            let snippet: String = msg.content.chars().take(60).collect();
            ui.label(egui::RichText::new(snippet).weak().small());
            if ui.small_button("copy message").clicked() {
                ui.ctx().copy_text(msg.content.clone());
                overlays.push_toast("copied message", std::time::Duration::from_secs(2));
                state.chat.menu_message = None;
                overlays.dismiss_menu();
            }
            if ui.small_button("close").clicked() {
                state.chat.menu_message = None;
                overlays.dismiss_menu();
            }
        });
    }
}
