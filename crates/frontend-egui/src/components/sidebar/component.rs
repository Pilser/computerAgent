//! Sidebar: thread rows + search + new/delete, backed by the engine.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::ChatState;

/// Thread list: search box, rows (select + delete), new-thread button.
pub struct Sidebar;

impl Sidebar {
    /// Build the sidebar.
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Sidebar {
    fn id(&self) -> &'static str {
        "sidebar"
    }

    fn slot(&self) -> SlotId {
        SlotId::Sidebar
    }

    fn layer(&self) -> Layer {
        Layer::Base
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        state: &mut ChatState,
        engine: &Engine,
        _overlays: &mut OverlayStack,
    ) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("threads");
                if ui.small_button("+ new").clicked() {
                    if let Ok(thread) = engine.new_thread() {
                        let id = thread.id.clone();
                        state.refresh_from_engine(engine);
                        state.threads.select(&id);
                        if let Some(session) = engine.open_thread(&id) {
                            state.chat.load_session(&session);
                        }
                        state.refresh_usage(engine, Some(&id));
                    }
                }
            });
            ui.add(
                egui::TextEdit::singleline(&mut state.threads.search).hint_text("search threads"),
            );
            ui.separator();
            // Collect first: row clicks mutate selection + session below.
            let rows: Vec<(String, String, String)> = state
                .threads
                .filtered()
                .iter()
                .map(|r| (r.id.clone(), r.title.clone(), r.model.clone()))
                .collect();
            if rows.is_empty() {
                ui.label(egui::RichText::new("no threads").weak().small());
            }
            let mut open: Option<String> = None;
            let mut delete: Option<String> = None;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (id, title, model) in &rows {
                        let selected = state.threads.active_id.as_deref() == Some(id.as_str());
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, title).clicked() {
                                open = Some(id.clone());
                            }
                            if ui.small_button("x").clicked() {
                                delete = Some(id.clone());
                            }
                        });
                        ui.label(egui::RichText::new(model).weak().small());
                    }
                });
            if let Some(id) = delete {
                let was_active = state.threads.active_id.as_deref() == Some(id.as_str());
                let _ = engine.delete_thread(&id);
                state.threads.remove(&id);
                state.refresh_from_engine(engine);
                if was_active {
                    state.chat.clear();
                    state.refresh_usage(engine, None);
                }
            } else if let Some(id) = open {
                state.threads.select(&id);
                if let Some(session) = engine.open_thread(&id) {
                    state.chat.load_session(&session);
                }
                state.refresh_usage(engine, Some(&id));
            }
        });
    }
}
