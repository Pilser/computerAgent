//! Toast: transient notices ("copied", …) in a bottom-right window.
//!
//! Producers push via [`OverlayStack::push_toast`](crate::shell::OverlayStack);
//! the shell shows this component while any toast is live.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, OverlayStack, SlotId};
use crate::state::ChatState;

/// Toast overlay: draws every live notice.
pub struct Toast;

impl Toast {
    /// Build the toast overlay.
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

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        _state: &mut ChatState,
        _engine: &Engine,
        overlays: &mut OverlayStack,
    ) {
        for text in overlays.toasts() {
            ui.label(&text);
        }
    }
}
