//! `Component` trait + ordered inventory. One `register()` line per
//! plug-in; the shell iterates in order, base slots first.

use super::overlay::{Anchor, OverlayKind, OverlayStack};
use super::slot::{Allocated, SlotId};
use crate::state::ChatState;

/// Which paint layer a component belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Shell slot, rect fixed before drawing.
    Base,
    /// Window-space overlay, painted last (§3 of `ARCHITECTURE.md`).
    Overlay(OverlayKind),
}

/// One shell plug-in. NOTE: `show` takes `&mut ChatState` (narrow slices
/// in later phases) plus the shared [`Engine`](computer_agent_engine::Engine)
/// for thread/model actions and the [`OverlayStack`] for toast/modal/menu
/// requests; the `ARCHITECTURE.md` §2 sketch omits data flow because it
/// describes the layout contract only.
pub trait Component {
    /// Stable id (`"feed"`, `"approval"`, …) for overlay routing.
    fn id(&self) -> &'static str;
    /// Which shell region owns this component.
    fn slot(&self) -> SlotId;
    /// Base or overlay (plus overlay kind).
    fn layer(&self) -> Layer;
    /// Overlay anchor; base components ignore it.
    fn anchor(&self) -> Anchor {
        Anchor::Center
    }
    /// Draw inside `slot` (already clipped to the pre-allocated rect).
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        slot: Allocated,
        state: &mut ChatState,
        engine: &computer_agent_engine::Engine,
        overlays: &mut OverlayStack,
    );
}

/// Ordered plug-in inventory; registration order = paint order.
#[derive(Default)]
pub struct Registry {
    components: Vec<Box<dyn Component>>,
}

impl Registry {
    /// Empty inventory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one plug-in (one line per component in `register_all`).
    pub fn register(&mut self, component: impl Component + 'static) {
        self.components.push(Box::new(component));
    }

    /// Mutable pass over every plug-in, registration order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Component>> {
        self.components.iter_mut()
    }

    /// How many plug-ins are registered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// True when no plug-in is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}
