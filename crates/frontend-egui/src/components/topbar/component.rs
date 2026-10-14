//! Topbar: brand + model picker + context usage bar + cost label + theme.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::ChatState;
use crate::theme;

/// Model combo, usage bar, cost: the whole Slint top-row checklist.
pub struct Topbar;

impl Topbar {
    /// Build the topbar.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Topbar {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Topbar {
    fn id(&self) -> &'static str {
        "topbar"
    }

    fn slot(&self) -> SlotId {
        SlotId::Topbar
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
        let (accent, _) = theme::accent(dark);
        let ink = if dark { theme::EMBER_INK } else { theme::INK };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⬢").size(18.0).color(accent));
            ui.label(
                egui::RichText::new("computerAgent")
                    .strong()
                    .size(17.0)
                    .color(ink),
            );
            let moon = if dark { "☀ light" } else { "☾ dark" };
            if theme::chip(ui, moon, dark).clicked() {
                state.dark_mode = !dark;
            }
            let mut picked = state.models.current.clone();
            let models = state.models.models.clone();
            egui::ComboBox::from_label("model")
                .selected_text(&picked)
                .show_ui(ui, |ui| {
                    for m in &models {
                        ui.selectable_value(&mut picked, m.clone(), m);
                    }
                });
            if picked != state.models.current && engine.set_model(&picked).is_ok() {
                state.models.current = picked;
            }
            ui.separator();
            ui.add(
                egui::ProgressBar::new(state.usage.fraction())
                    .fill(if dark {
                        theme::EMBER_AMBER
                    } else {
                        theme::AMBER
                    })
                    .desired_width(120.0)
                    .show_percentage(),
            );
            ui.label(egui::RichText::new(state.usage.ctx_label()).weak().small());
            ui.label(
                egui::RichText::new(state.usage.cost_label())
                    .strong()
                    .small(),
            );
        });
    }
}
