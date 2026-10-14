//! Palette: Modal overlay stub (real render in later phases).

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, OverlayStack, SlotId};
use computer_agent_engine::Engine;

use crate::state::ChatState;

/// Palette stub: draws only its id label when the stack shows it.
pub struct Palette;

impl Palette {
    /// Build the stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Palette {
    fn id(&self) -> &'static str {
        "palette"
    }

    fn slot(&self) -> SlotId {
        SlotId::Feed
    }

    fn layer(&self) -> Layer {
        Layer::Overlay(OverlayKind::Modal)
    }

    fn anchor(&self) -> Anchor {
        Anchor::Center
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        _state: &mut ChatState,
        _engine: &Engine,
        _overlays: &mut OverlayStack,
    ) {
        ui.label("palette (stub)");
    }
}
