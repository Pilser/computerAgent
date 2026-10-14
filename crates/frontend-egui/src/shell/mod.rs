//! Shell: owns layout, registry iteration, and the overlay stack.
//!
//! Per frame: `layout()` numbers are applied to panels FIRST (fixed
//! widths/heights, so wrapped text measures against a known number),
//! base components draw in registration order, overlays paint last.

pub mod layout;
pub mod overlay;
pub mod registry;
pub mod slot;

pub use layout::{density_scale, regions, sidebar_width, RegionMap, NARROW_BREAKPOINT};
pub use overlay::{Anchor, OverlayKind, OverlayStack};
pub use registry::{Component, Layer, Registry};
pub use slot::{policy, Allocated, SlotId, SlotPolicy};

use crate::components::register_all;
use crate::state::ChatState;

/// Layout owner: partitions the window, then runs every plug-in.
pub struct Shell {
    registry: Registry,
    overlays: OverlayStack,
}

impl Shell {
    /// Shell with every phase-1 plug-in registered.
    pub fn new() -> Self {
        let mut registry = Registry::new();
        register_all(&mut registry);
        Self {
            registry,
            overlays: OverlayStack::new(),
        }
    }

    /// Overlay stack (modal/toast/menu requests from components).
    pub fn overlays_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlays
    }

    /// One frame: panels → base slots → overlay stack.
    pub fn run(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut ChatState,
        engine: &computer_agent_engine::Engine,
    ) {
        crate::theme::apply(ui.ctx());
        let width = ui.max_rect().width();
        let narrow = width < NARROW_BREAKPOINT;
        let _density = density_scale(width);

        if state.status.banner_current().is_some() {
            egui::Panel::top("banner").resizable(false).show(ui, |ui| {
                self.show_slot(SlotId::Banner, ui, state, engine);
            });
        }
        egui::Panel::top("topbar")
            .exact_size(48.0)
            .resizable(false)
            .show(ui, |ui| {
                self.show_slot(SlotId::Topbar, ui, state, engine);
            });
        if !narrow {
            egui::Panel::left("sidebar")
                .exact_size(300.0)
                .resizable(false)
                .show(ui, |ui| {
                    self.show_slot(SlotId::Sidebar, ui, state, engine);
                });
        }
        egui::Panel::bottom("composer")
            .exact_size(132.0)
            .resizable(false)
            .show(ui, |ui| {
                self.show_slot(SlotId::Composer, ui, state, engine);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.show_slot(SlotId::Feed, ui, state, engine);
        });

        self.show_overlays(ui, state, engine);
    }

    /// Run every base component registered for `slot`.
    fn show_slot(
        &mut self,
        slot: SlotId,
        ui: &mut egui::Ui,
        state: &mut ChatState,
        engine: &computer_agent_engine::Engine,
    ) {
        let rect = ui.max_rect();
        for component in self.registry.iter_mut() {
            if component.slot() == slot && matches!(component.layer(), Layer::Base) {
                component.show(
                    ui,
                    Allocated::new(slot, rect),
                    state,
                    engine,
                    &mut self.overlays,
                );
            }
        }
    }

    /// Paint the overlay layer last: live modal + toasts + menu.
    fn show_overlays(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut ChatState,
        engine: &computer_agent_engine::Engine,
    ) {
        let modal = self.overlays.modal().map(str::to_string);
        let has_toasts = !self.overlays.toasts().is_empty();
        let rect = ui.max_rect();
        let ctx = ui.ctx().clone();
        for component in self.registry.iter_mut() {
            let id = component.id();
            match component.layer() {
                Layer::Base => {}
                Layer::Overlay(OverlayKind::Modal) => {
                    if modal.as_deref() == Some(id) {
                        egui::Window::new(id)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .collapsible(false)
                            .resizable(false)
                            .show(&ctx, |ui| {
                                component.show(
                                    ui,
                                    Allocated::new(component.slot(), rect),
                                    state,
                                    engine,
                                    &mut self.overlays,
                                );
                            });
                    }
                }
                Layer::Overlay(OverlayKind::Toast) => {
                    if has_toasts {
                        egui::Window::new(id)
                            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
                            .title_bar(false)
                            .collapsible(false)
                            .resizable(false)
                            .show(&ctx, |ui| {
                                component.show(
                                    ui,
                                    Allocated::new(component.slot(), rect),
                                    state,
                                    engine,
                                    &mut self.overlays,
                                );
                            });
                    }
                }
                Layer::Overlay(OverlayKind::Menu) => {
                    if self.overlays.menu() == Some(id) {
                        egui::Window::new(id)
                            .collapsible(false)
                            .resizable(false)
                            .show(&ctx, |ui| {
                                component.show(
                                    ui,
                                    Allocated::new(component.slot(), rect),
                                    state,
                                    engine,
                                    &mut self.overlays,
                                );
                            });
                    }
                }
            }
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}
