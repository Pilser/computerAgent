//! Sidebar: thread rows + search + new/delete, backed by the engine.
//!
//! Rows are full-width tinted cards: the active thread gets an accent wash
//! plus a terracotta brand bar on its left edge, hover fill comes from the
//! warm widget visuals. Section labels are small, muted, letterspaced.

use computer_agent_engine::Engine;

use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::ChatState;
use crate::theme;

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

/// Letterspaced muted section label (`"THREADS"` → `"T H R E A D S"`).
fn section(ui: &mut egui::Ui, text: &str, dark: bool) {
    let spaced: String = text
        .chars()
        .map(|c| {
            if c == ' ' {
                "  ".to_string()
            } else {
                format!("{c} ")
            }
        })
        .collect();
    ui.label(
        egui::RichText::new(spaced.trim_end())
            .small()
            .color(theme::soft_text(dark, false)),
    );
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
        let dark = state.dark_mode;
        let (accent, _) = theme::accent(dark);
        let ink = if dark { theme::EMBER_INK } else { theme::INK };
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Threads")
                        .strong()
                        .size(16.0)
                        .color(if dark { theme::EMBER_INK } else { theme::INK }),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::chip(ui, "+ new", dark).clicked() {
                        if let Ok(thread) = engine.new_thread() {
                            let id = thread.id.clone();
                            state.refresh_from_engine(engine);
                            state.threads.select(&id);
                            if let Some(session) = engine.open_thread(&id) {
                                state.chat.load_session(&session);
                            }
                            state.refresh_usage(engine, Some(id.as_str()));
                        }
                    }
                });
            });
            ui.add(
                egui::TextEdit::singleline(&mut state.threads.search).hint_text("Search threads…"),
            );
            ui.add_space(4.0);
            ui.separator();
            section(ui, "THREADS", dark);
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
                        let row = egui::Frame {
                            fill: if selected {
                                if dark {
                                    theme::EMBER_WASH
                                } else {
                                    theme::AMBER_WASH
                                }
                            } else {
                                egui::Color32::TRANSPARENT
                            },
                            corner_radius: egui::CornerRadius::same(theme::BUTTON_ROUNDING),
                            inner_margin: egui::Margin::same(6),
                            ..Default::default()
                        };
                        row.show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (bar, _) = ui.allocate_exact_size(
                                    egui::vec2(3.0, 32.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    bar,
                                    egui::CornerRadius::same(2),
                                    if selected {
                                        accent
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    },
                                );
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .selectable_label(
                                                selected,
                                                egui::RichText::new(title).strong().color(ink),
                                            )
                                            .clicked()
                                        {
                                            open = Some(id.clone());
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if theme::chip(ui, "x", dark).clicked() {
                                                    delete = Some(id.clone());
                                                }
                                            },
                                        );
                                    });
                                    ui.label(
                                        egui::RichText::new(model)
                                            .small()
                                            .color(theme::soft_text(dark, false)),
                                    );
                                });
                            });
                        });
                        ui.add_space(2.0);
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
                state.refresh_usage(engine, Some(id.as_str()));
            }
            ui.separator();
            let total = engine.total_cost();
            let total_label = if total < 1.0 {
                format!("Total ${total:.4}")
            } else {
                format!("Total ${total:.2}")
            };
            ui.label(
                egui::RichText::new(total_label)
                    .small()
                    .color(theme::soft_text(dark, false)),
            );
        });
    }
}
