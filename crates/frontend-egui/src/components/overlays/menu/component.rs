//! Menu: Menu overlay stub (real render in later phases).

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, OverlayStack, SlotId};
use computer_agent_engine::Engine;

use crate::state::ChatState;

/// Menu stub: draws only its id label when the stack shows it.
pub struct Menu;

impl Menu {
    /// Build the stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Menu {
    fn id(&self) -> &'static str {
        "menu"
    }

    fn slot(&self) -> SlotId {
        SlotId::Feed
    }

    fn layer(&self) -> Layer {
        Layer::Overlay(OverlayKind::Menu)
    }

    fn anchor(&self) -> Anchor {
        Anchor::Cursor
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        _state: &mut ChatState,
        _engine: &Engine,
        _overlays: &mut OverlayStack,
    ) {
        ui.label("menu (stub)");
    }
}
