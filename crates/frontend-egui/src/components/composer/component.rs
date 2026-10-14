//! Composer: draft input + Send, wired to the pump via send intent.
//!
//! The component never touches the pump: `Send` sets [`ChatState`](crate::state::ChatState)
//! intent, the app owns the `pump.send()` call. While a turn is in flight the
//! Send button swaps to Stop (which flips the flag off and cancels the run).

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::theme;
use computer_agent_engine::Engine;

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

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        state: &mut ChatState,
        engine: &Engine,
        _overlays: &mut OverlayStack,
    ) {
        let dark = state.dark_mode;
        ui.vertical(|ui| {
            let width = ui.available_width();
            let frame = egui::Frame {
                fill: if dark {
                    theme::EMBER_SURFACE
                } else {
                    theme::PANEL
                },
                stroke: egui::Stroke::new(1.0, theme::hairline(dark)),
                corner_radius: egui::CornerRadius::same(theme::FRAME_ROUNDING),
                inner_margin: egui::Margin::same(10),
                ..Default::default()
            };
            ui.add(
                egui::TextEdit::multiline(&mut state.chat.draft)
                    .desired_rows(3)
                    .desired_width(width)
                    .hint_text("Message the agent…")
                    .frame(frame),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if state.chat.sending {
                    let stop = egui::Button::new(
                        egui::RichText::new("■ Stop").strong().color(theme::PAPER),
                    )
                    .fill(theme::DANGER)
                    .corner_radius(egui::CornerRadius::same(theme::BUTTON_ROUNDING))
                    .min_size(egui::vec2(110.0, 32.0));
                    if ui.add(stop).clicked() {
                        state.chat.sending = false;
                        engine.cancel_all();
                    }
                    ui.spinner();
                    ui.label(
                        egui::RichText::new("sending…")
                            .small()
                            .color(theme::soft_text(dark, false)),
                    );
                } else {
                    let ready = !state.chat.sending && !state.chat.draft.trim().is_empty();
                    let (fill, text) = theme::accent(dark);
                    let send = egui::Button::new(egui::RichText::new("Send").strong().color(text))
                        .fill(fill)
                        .corner_radius(egui::CornerRadius::same(theme::BUTTON_ROUNDING))
                        .min_size(egui::vec2(110.0, 32.0));
                    if ui.add_enabled(ready, send).clicked() {
                        state.chat.request_send();
                    }
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
