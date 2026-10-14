//! Feed: Feed slot stub (phase 1 empty rect; real render in later phases).

use crate::shell::{Allocated, Component, Layer, SlotId};
use crate::state::ChatState;

/// Feed stub: reserves its slot, draws only its id label.
pub struct Feed;

impl Feed {
    /// Build the stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Feed {
    fn id(&self) -> &'static str {
        "feed"
    }

    fn slot(&self) -> SlotId {
        SlotId::Feed
    }

    fn layer(&self) -> Layer {
        Layer::Base
    }

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, _state: &mut ChatState) {
        ui.label("feed (stub)");
    }
}
