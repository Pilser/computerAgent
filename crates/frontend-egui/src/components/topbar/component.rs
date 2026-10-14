//! Topbar: model picker + context usage bar + cost label.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::ChatState;

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
        ui.horizontal(|ui| {
            ui.heading("computerAgent");
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
            ui.add(egui::ProgressBar::new(state.usage.fraction()).show_percentage());
            ui.label(egui::RichText::new(state.usage.ctx_label()).weak().small());
            ui.label(
                egui::RichText::new(state.usage.cost_label())
                    .strong()
                    .small(),
            );
        });
    }
}
