//! Feed: virtualized message bubbles with markdown + copy buttons.
//!
//! The log renders newest-last inside a stick-to-bottom scroll area (egui
//! keeps the view pinned unless the user scrolled up). Only the trailing
//! window of messages is drawn so giant threads stay cheap; uniform
//! `show_rows` virtualization is future work (variable-height bubbles need
//! measured rects first).

use egui_commonmark::CommonMarkCache;

use crate::markdown::show_message;
use crate::shell::{Allocated, Component, Layer, OverlayStack, SlotId};
use crate::state::{ChatState, Message};
use computer_agent_engine::Engine;
use computer_agent_session_store::session::MessageRole;

/// How many trailing messages the feed draws (older ones are summarized).
pub const FEED_WINDOW: usize = 200;

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
                let visible: Vec<Message> = self.visible(state.chat.messages()).to_vec();
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
                    ui.label(egui::RichText::new("No messages yet — say hi below.").weak());
                }
                for msg in &visible {
                    show_message_bubble(ui, msg, &mut self.md_cache, overlays);
                    ui.add_space(8.0);
                }
            });
    }
}

/// One bubble: role header with copy button, then the markdown body.
fn show_message_bubble(
    ui: &mut egui::Ui,
    msg: &Message,
    md_cache: &mut CommonMarkCache,
    overlays: &mut OverlayStack,
) {
    let (role, mark) = match msg.role {
        MessageRole::User => ("you", "↑"),
        MessageRole::Assistant => ("assistant", "↓"),
        _ => ("tool", "⚙"),
    };
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{mark} {role}"))
                .strong()
                .small(),
        );
        ui.label(
            egui::RichText::new(format!("{} tok", msg.tokens))
                .weak()
                .small(),
        );
        if ui.small_button("copy").clicked() {
            ui.ctx().copy_text(msg.content.clone());
            overlays.push_toast("copied message", std::time::Duration::from_secs(2));
        }
    });
    show_message(ui, &msg.content, md_cache);
}
