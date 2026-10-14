//! [`Block`] -> egui widgets.
//!
//! Prose ([`Block::Para`]) goes through `egui_commonmark` (inline markup,
//! links, inline code); every structural block gets a dedicated renderer:
//! headings, code frames with copy buttons, mermaid SVG images (falling
//! back to code blocks), quotes, and grid tables.

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use super::blocks::{parse_markdown, Block};
use super::diagram::{image_uri, render_cached};

/// Render one message body: split into [`Block`]s, then draw each.
pub fn show_message(ui: &mut egui::Ui, text: &str, md_cache: &mut CommonMarkCache) {
    let blocks = parse_markdown(text);
    if blocks.is_empty() {
        ui.label(text);
        return;
    }
    for block in &blocks {
        show_block(ui, block, md_cache);
    }
}

/// Draw one [`Block`] inside the feed.
pub fn show_block(ui: &mut egui::Ui, block: &Block, md_cache: &mut CommonMarkCache) {
    match block {
        Block::Heading { level, text } => {
            if *level <= 1 {
                ui.heading(text);
            } else {
                ui.label(egui::RichText::new(text).strong());
            }
        }
        Block::Para { text } => {
            CommonMarkViewer::new().show(ui, md_cache, text);
        }
        Block::Code { lang, code } => show_code(ui, lang, code),
        Block::Mermaid { source } => match render_cached(source) {
            Some(svg) => {
                ui.add(
                    egui::Image::from_bytes(image_uri(source), svg)
                        .alt_text("mermaid diagram")
                        .max_width(ui.available_width())
                        .shrink_to_fit(),
                );
            }
            None => show_code(ui, "mermaid", source),
        },
        Block::List { items } => {
            for item in items {
                ui.horizontal(|ui| {
                    ui.label("•");
                    ui.label(item);
                });
            }
        }
        Block::Quote { text } => {
            ui.horizontal(|ui| {
                ui.separator();
                ui.label(egui::RichText::new(text).italics());
            });
        }
        Block::Table { head, rows } => {
            egui::Grid::new(egui::Id::new(("chat-table", head.join("|"))))
                .striped(true)
                .show(ui, |ui| {
                    for cell in head {
                        ui.label(egui::RichText::new(cell).strong());
                    }
                    ui.end_row();
                    for row in rows {
                        for cell in row {
                            ui.label(cell);
                        }
                        ui.end_row();
                    }
                });
        }
    }
}

/// Monospace frame with language tag + copy button.
fn show_code(ui: &mut egui::Ui, lang: &str, code: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(lang_label(lang)).weak().small());
            if ui.small_button("copy").clicked() {
                ui.ctx().copy_text(code.to_string());
            }
        });
        ui.label(egui::RichText::new(code).monospace());
    });
}

fn lang_label(lang: &str) -> &str {
    if lang.trim().is_empty() {
        "code"
    } else {
        lang
    }
}
