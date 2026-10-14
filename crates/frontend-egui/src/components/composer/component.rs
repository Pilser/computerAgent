//! Composer: draft input + Send, wired to the pump via send intent.
//!
//! The component never touches the pump: `Send` (or Ctrl+Enter in later
//! phases) sets [`ChatState`](crate::state::ChatState) intent, the app owns
//! the `pump.send()` call.

use crate::shell::{Allocated, Component, Layer, SlotId};
use crate::state::ChatState;

/// Draft editor + send button.
pub struct Composer;

impl Composer {
    /// Build the composer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Composer {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Composer {
    fn id(&self) -> &'static str {
        "composer"
    }

    fn slot(&self) -> SlotId {
        SlotId::Composer
    }

    fn layer(&self) -> Layer {
        Layer::Base
    }

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, state: &mut ChatState) {
        ui.vertical(|ui| {
            let width = ui.available_width();
            ui.add(
                egui::TextEdit::multiline(&mut state.chat.draft)
                    .desired_rows(4)
                    .desired_width(width)
                    .hint_text("Message the agent…"),
            );
            ui.horizontal(|ui| {
                let ready = !state.chat.sending && !state.chat.draft.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Send")).clicked() {
                    state.chat.request_send();
                }
                if state.chat.sending {
                    ui.spinner();
                    ui.label("sending…");
                } else {
                    ui.label(
                        egui::RichText::new("Send runs the agent turn")
                            .weak()
                            .small(),
                    );
                }
            });
        });
    }
}
