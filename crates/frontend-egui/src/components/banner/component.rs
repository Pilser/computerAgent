//! Banner: Banner slot stub (phase 1 empty rect; real render in later phases).

use crate::shell::{Allocated, Component, Layer, SlotId};
use crate::state::ChatState;

/// Banner stub: reserves its slot, draws only its id label.
pub struct Banner;

impl Banner {
    /// Build the stub.
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

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, _state: &mut ChatState) {
        ui.label("banner (stub)");
    }
}
