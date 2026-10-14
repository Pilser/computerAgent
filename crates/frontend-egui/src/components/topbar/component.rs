//! Topbar: Topbar slot stub (phase 1 empty rect; real render in later phases).

use crate::shell::{Allocated, Component, Layer, SlotId};
use crate::state::ChatState;

/// Topbar stub: reserves its slot, draws only its id label.
pub struct Topbar;

impl Topbar {
    /// Build the stub.
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

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, _state: &mut ChatState) {
        ui.label("topbar (stub)");
    }
}
