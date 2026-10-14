//! Banner: oldest error/status notice with a dismiss button.
//!
//! The shell only reserves this slot while
//! [`StatusState::banner_current`](crate::state::StatusState::banner_current)
//! is `Some`.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::ChatState;

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
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(text).strong());
                if ui.small_button("dismiss").clicked() {
                    state.status.dismiss_banner();
                }
            });
        }
    }
}
