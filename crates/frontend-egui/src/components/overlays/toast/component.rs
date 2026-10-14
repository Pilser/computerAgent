//! Toast: Toast overlay stub (real render in later phases).

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, SlotId};
use crate::state::ChatState;

/// Toast stub: draws only its id label when the stack shows it.
pub struct Toast;

impl Toast {
    /// Build the stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Toast {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Toast {
    fn id(&self) -> &'static str {
        "toast"
    }

    fn slot(&self) -> SlotId {
        SlotId::Feed
    }

    fn layer(&self) -> Layer {
        Layer::Overlay(OverlayKind::Toast)
    }

    fn anchor(&self) -> Anchor {
        Anchor::DockBottomRight
    }

    fn show(&mut self, ui: &mut egui::Ui, _slot: Allocated, _state: &mut ChatState) {
        ui.label("toast (stub)");
    }
}
