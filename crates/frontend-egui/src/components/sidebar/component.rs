//! Sidebar: Sidebar slot stub (phase 1 empty rect; real render in later phases).

use crate::shell::{Allocated, Component, Layer, SlotId};
use crate::state::ChatState;

/// Sidebar stub: reserves its slot, draws only its id label.
pub struct Sidebar;

impl Sidebar {
    /// Build the stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Sidebar {
    fn id(&self) -> &'static str {
        "sidebar"
    }

    fn slot(&self) -> SlotId {
        SlotId::Sidebar
    }

    fn layer(&self) -> Layer {
        Layer::Base
    }

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, _state: &mut ChatState) {
        ui.label("sidebar (stub)");
    }
}
