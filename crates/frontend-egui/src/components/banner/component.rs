//! Banner: oldest error/status notice with a dismiss button.
//!
//! The shell only reserves this slot while
//! [`StatusState::banner_current`](crate::state::StatusState::banner_current)
//! is `Some`. The notice paints as a danger-tinted card, not a bare label.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::ChatState;
use crate::theme;

/// Error banner: notice text + dismiss.
pub struct Banner;

impl Banner {
    /// Build the banner.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Banner {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Banner {
    fn id(&self) -> &'static str {
        "banner"
    }

    fn slot(&self) -> SlotId {
        SlotId::Banner
    }

    fn layer(&self) -> Layer {
        Layer::Base
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        state: &mut ChatState,
        _engine: &Engine,
        _overlays: &mut OverlayStack,
    ) {
        if let Some(text) = state.status.banner_current().map(str::to_string) {
            let dark = state.dark_mode;
            let (wash, edge, ink) = theme::danger(dark);
            egui::Frame {
                fill: wash,
                stroke: egui::Stroke::new(1.0, edge),
                corner_radius: egui::CornerRadius::same(theme::BUTTON_ROUNDING),
                inner_margin: egui::Margin::same(8),
                ..Default::default()
            }
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("!").strong().color(edge));
                    ui.label(egui::RichText::new(text).strong().color(ink));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if theme::chip(ui, "dismiss", dark).clicked() {
                            state.status.dismiss_banner();
                        }
                    });
                });
            });
        }
    }
}
