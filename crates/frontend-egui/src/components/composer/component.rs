//! Composer: Composer slot stub (phase 1 empty rect; real render in later phases).

use crate::shell::{Allocated, Component, Layer, SlotId};
use crate::state::ChatState;

/// Composer stub: reserves its slot, draws only its id label.
pub struct Composer;

impl Composer {
    /// Build the stub.
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

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, _state: &mut ChatState) {
        ui.label("composer (stub)");
    }
}
