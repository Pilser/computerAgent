//! Feed: virtualized message bubbles with markdown + copy buttons.
//!
//! The log renders newest-last inside a stick-to-bottom scroll area (egui
//! keeps the view pinned unless the user scrolled up). Only the trailing
//! window of messages is drawn so giant threads stay cheap; uniform
//! `show_rows` virtualization is future work (variable-height bubbles need
//! measured rects first).
//!
//! Bubble pattern: user messages hug the right in terracotta, assistant
//! messages hug the left on the surface wash, both capped at ~72% of the
//! feed width. Colors come from [`theme`](crate::theme) helpers so dark
//! mode flips for free.

use egui_commonmark::CommonMarkCache;

use crate::markdown::show_message;
use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::{ChatState, Message};
use crate::theme;
use computer_agent_engine::Engine;
use computer_agent_session_store::session::MessageRole;

/// How many trailing messages the feed draws (older ones are summarized).
pub const FEED_WINDOW: usize = 200;

/// Share of the feed width one bubble may occupy at most.
const BUBBLE_WIDTH: f32 = 0.72;

/// Message feed: bubbles + markdown + per-message copy.
pub struct Feed {
    md_cache: CommonMarkCache,
}

impl Feed {
    /// Build the feed with a fresh markdown cache.
    pub fn new() -> Self {
        Self {
            md_cache: CommonMarkCache::default(),
        }
    }

    /// Messages actually drawn (trailing window).
    fn visible<'a>(&self, messages: &'a [Message]) -> &'a [Message] {
        let skip = messages.len().saturating_sub(FEED_WINDOW);
        &messages[skip..]
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

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _slot: Allocated,
        state: &mut ChatState,
        _engine: &Engine,
        overlays: &mut OverlayStack,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let total = state.chat.messages().len();
                let base = total.saturating_sub(FEED_WINDOW);
                let visible: Vec<(usize, Message)> = self
                    .visible(state.chat.messages())
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(k, m)| (base + k, m))
                    .collect();
                if visible.len() < total {
                    ui.label(
                        egui::RichText::new(format!(
                            "… {} earlier messages",
                            total - visible.len()
                        ))
                        .weak()
                        .small(),
                    );
                    ui.separator();
                }
                if visible.is_empty() {
                    ui.add_space(24.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("No messages yet — say hi below.")
                                .weak()
                                .size(16.0),
                        );
                    });
                }
                for (idx, msg) in visible {
                    show_message_bubble(ui, idx, &msg, &mut self.md_cache, overlays, state);
                    ui.add_space(8.0);
                }
            });
    }
}

/// One bubble: role header with copy + menu chips, markdown body, token
/// footnote. User right/terracotta, assistant left/surface.
fn show_message_bubble(
    ui: &mut egui::Ui,
    idx: usize,
    msg: &Message,
    md_cache: &mut CommonMarkCache,
    overlays: &mut OverlayStack,
    state: &mut ChatState,
) {
    let dark = state.dark_mode;
    let is_user = msg.role == MessageRole::User;
    let (fill, text) = theme::bubble(dark, is_user);
    let footnote = theme::soft_text(dark, is_user);
    let edge = theme::hairline(dark);
    let align = if is_user {
        egui::Align::Max
    } else {
        egui::Align::Min
    };
    let max_w = (ui.available_width() * BUBBLE_WIDTH).max(220.0);
    ui.with_layout(egui::Layout::top_down(align), |ui| {
        egui::Frame {
            fill,
            stroke: egui::Stroke::new(1.0, edge),
            corner_radius: egui::CornerRadius::same(theme::FRAME_ROUNDING),
            inner_margin: egui::Margin::same(10),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.set_max_width(max_w);
            ui.visuals_mut().override_text_color = Some(text);
            let role = match msg.role {
                MessageRole::User => "you",
                MessageRole::Assistant => "assistant",
                _ => "tool",
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(role).strong().small().color(text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::chip(ui, "…", dark).clicked() {
                        state.chat.menu_message = Some(idx);
                        overlays.show_menu("menu");
                    }
                    if theme::chip(ui, "copy", dark).clicked() {
                        ui.ctx().copy_text(msg.content.clone());
                        overlays.push_toast("copied message", std::time::Duration::from_secs(2));
                    }
                });
            });
            ui.separator();
            show_message(ui, &msg.content, md_cache);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("{} tok", msg.tokens))
                    .small()
                    .color(footnote),
            );
        });
    });
}
