//! Approval: modal Ask-approval dialog for one gated tool call.
//!
//! Renders [`ApprovalState::pending`](crate::state::ApprovalState); Allow /
//! Deny records the answer there. Engine wiring is a TODO (see
//! [`crate::state::approval`]): nothing stages requests yet, so this dialog
//! only appears in tests / once the engine streams Ask events.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Anchor, Component, Layer, OverlayKind, OverlayStack, SlotId};
use crate::state::ChatState;

/// Ask-approval dialog: tool + input with Allow/Deny.
pub struct Approval;

impl Approval {
    /// Build the approval dialog.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Approval {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Approval {
    fn id(&self) -> &'static str {
        "approval"
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
        state: &mut ChatState,
        _engine: &Engine,
        overlays: &mut OverlayStack,
    ) {
        let Some(req) = state.approval.pending().cloned() else {
            return;
        };
        if overlays.modal() != Some("approval") {
            overlays.show_modal("approval");
        }
        ui.vertical(|ui| {
            ui.heading("Approval needed");
            ui.label(format!("tool: {}", req.tool));
            ui.label(format!("input: {}", req.input));
            ui.horizontal(|ui| {
                if ui.button("Allow").clicked() {
                    state.approval.answer(true);
                    overlays.dismiss_modal();
                }
                if ui.button("Deny").clicked() {
                    state.approval.answer(false);
                    overlays.dismiss_modal();
                }
            });
        });
    }
}
