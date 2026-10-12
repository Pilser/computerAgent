//! Chat text -> Slint `MdBlock` models + pre-rendered diagram images.
//!
//! Call on the Slint event-loop thread only: diagram bytes become
//! [`slint::Image`]s via [`slint::Image::load_from_svg_data`], and `Image`
//! is `!Send`. Any diagram that fails to render or load falls back to a
//! fenced-code block so the message is never lost. `bridge.rs`/`app.rs`
//! are untouched (see `docs/ARCHITECTURE.md` §2).

use slint::{Image, SharedString};

use crate::markdown::{diagram::render_cached, parse_markdown, Block};
use crate::MdBlock;

/// One Slint block-model row (alias of the generated `MdBlock` struct).
pub type UiBlock = MdBlock;

/// Split `text` into view blocks plus `(diagram-id, image)` pairs for the
/// `MdView` image properties. Tables expand to one `table-head` row plus
/// one `table-row` per body row.
pub fn message_blocks(text: &str) -> (Vec<UiBlock>, Vec<(String, Image)>) {
    let mut out = Vec::new();
    let mut images = Vec::new();
    let mut diagrams = 0;
    for block in parse_markdown(text) {
        match block {
            Block::Heading { level, text } => out.push(ui_block("h", text, "", level as i32)),
            Block::Para { text } => out.push(ui_block("p", text, "", 0)),
            Block::Code { lang, code } => out.push(ui_block("code", code, &lang, 0)),
            Block::Mermaid { source } => {
                diagrams += 1;
                let id = format!("mermaid-{diagrams}");
                match render_cached(&source).and_then(|svg| Image::load_from_svg_data(&svg).ok()) {
                    Some(img) => {
                        images.push((id, img.clone()));
                        out.push(image_block(&source, img));
                    }
                    None => out.push(ui_block("code", source, "mermaid", 0)),
                }
            }
            Block::List { items } => {
                let text = items
                    .iter()
                    .map(|i| format!("• {i}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                out.push(ui_block("list", text, "", 0));
            }
            Block::Quote { text } => out.push(ui_block("quote", text, "", 0)),
            Block::Table { head, rows } => {
                if !head.is_empty() {
                    out.push(ui_block("table-head", head.join("  │  "), "", 0));
                }
                for row in rows {
                    out.push(ui_block("table-row", row.join("  │  "), "", 0));
                }
            }
        }
    }
    (out, images)
}

fn ui_block(kind: &str, text: String, lang: &str, level: i32) -> UiBlock {
    UiBlock {
        kind: SharedString::from(kind),
        text: SharedString::from(text),
        lang: SharedString::from(lang),
        level,
        img: Image::default(),
    }
}

fn image_block(source: &str, img: Image) -> UiBlock {
    UiBlock {
        kind: SharedString::from("mermaid"),
        text: SharedString::from(source),
        lang: SharedString::from("mermaid"),
        level: 0,
        img,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(blocks: &[UiBlock]) -> Vec<String> {
        blocks.iter().map(|b| b.kind.to_string()).collect()
    }

    #[test]
    fn para_maps_to_p_block() {
        let (blocks, images) = message_blocks("hello\n");
        assert_eq!(kinds(&blocks), vec!["p"]);
        assert_eq!(blocks[0].text.as_str(), "hello");
        assert!(images.is_empty());
    }

    #[test]
    fn empty_diagram_falls_back_to_code() {
        let (blocks, images) = message_blocks("```mermaid\n```\n");
        assert_eq!(kinds(&blocks), vec!["code"]);
        assert_eq!(blocks[0].lang.as_str(), "mermaid");
        assert!(images.is_empty());
    }

    #[test]
    fn table_expands_to_head_plus_rows() {
        let (blocks, _) = message_blocks("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert_eq!(kinds(&blocks), vec!["table-head", "table-row"]);
        assert!(blocks[0].text.as_str().contains('a'));
        assert!(blocks[1].text.as_str().contains('1'));
    }

    #[test]
    fn list_items_get_bullets() {
        let (blocks, _) = message_blocks("- a\n- b\n");
        assert_eq!(kinds(&blocks), vec!["list"]);
        assert_eq!(blocks[0].text.as_str(), "• a\n• b");
    }
}
