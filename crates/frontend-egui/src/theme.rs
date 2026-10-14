//! Warm theme: paper background, terracotta accents, amber selection.
//!
//! The ONLY styling entry point: [`apply`] runs at the start of EVERY frame
//! (see `Shell::run`), so components never set colors themselves — they read
//! ambient visuals plus the small helpers below (`bubble`, `soft_text`,
//! `hairline`, …). Dark mode is one `bool` on [`ChatState`](crate::state::ChatState)
//! flipping the full ember palette; there is no half-themed state.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Visuals};

/// Window wash (light).
pub const PAPER: Color32 = Color32::from_rgb(0xFA, 0xF5, 0xEC);
/// Panel fill, a breath lighter than paper (light).
pub const PANEL: Color32 = Color32::from_rgb(0xFF, 0xFD, 0xF8);
/// Assistant bubbles, code blocks, idle chips (light).
pub const SURFACE: Color32 = Color32::from_rgb(0xF1, 0xE7, 0xD3);
/// Idle chip fill, one step up from surface (light).
pub const CHIP: Color32 = Color32::from_rgb(0xEA, 0xDC, 0xC2);
/// Primary accent: user bubbles, Send, active rows (light).
pub const TERRACOTTA: Color32 = Color32::from_rgb(0xC4, 0x62, 0x2D);
/// Pressed/open accent (light).
pub const TERRA_DEEP: Color32 = Color32::from_rgb(0x9A, 0x4A, 0x22);
/// Secondary accent: hovers, selection, usage bar (light).
pub const AMBER: Color32 = Color32::from_rgb(0xB9, 0x8A, 0x2F);
/// Hover/selection wash (light).
pub const AMBER_WASH: Color32 = Color32::from_rgb(0xF3, 0xE2, 0xBE);
/// Body text on paper (light).
pub const INK: Color32 = Color32::from_rgb(0x2E, 0x26, 0x20);
/// Footnotes, labels, placeholders (light).
pub const INK_SOFT: Color32 = Color32::from_rgb(0x6F, 0x60, 0x50);
/// Borders, separators, chip strokes (light).
pub const HAIRLINE: Color32 = Color32::from_rgb(0xDE, 0xCF, 0xB4);
/// Errors, Stop, banner edge (both themes).
pub const DANGER: Color32 = Color32::from_rgb(0xB3, 0x26, 0x1E);
/// Banner wash (light).
pub const DANGER_WASH: Color32 = Color32::from_rgb(0xF8, 0xDD, 0xD4);

/// Window wash (dark ember).
pub const EMBER_BG: Color32 = Color32::from_rgb(0x21, 0x1A, 0x13);
/// Panel fill (dark ember).
pub const EMBER_PANEL: Color32 = Color32::from_rgb(0x2B, 0x22, 0x19);
/// Bubbles, code blocks, idle chips (dark ember).
pub const EMBER_SURFACE: Color32 = Color32::from_rgb(0x3B, 0x2F, 0x23);
/// Idle chip fill (dark ember).
pub const EMBER_CHIP: Color32 = Color32::from_rgb(0x4A, 0x3B, 0x2C);
/// Primary accent (dark ember).
pub const EMBER_TERRA: Color32 = Color32::from_rgb(0xD9, 0x7B, 0x4A);
/// Pressed/open accent (dark ember).
pub const EMBER_DEEP: Color32 = Color32::from_rgb(0xA8, 0x55, 0x30);
/// Secondary accent (dark ember).
pub const EMBER_AMBER: Color32 = Color32::from_rgb(0xD9, 0xA4, 0x41);
/// Hover/selection wash (dark ember).
pub const EMBER_WASH: Color32 = Color32::from_rgb(0x54, 0x42, 0x26);
/// Body text (dark ember).
pub const EMBER_INK: Color32 = Color32::from_rgb(0xF2, 0xE8, 0xD8);
/// Footnotes, labels (dark ember).
pub const EMBER_SOFT: Color32 = Color32::from_rgb(0xB8, 0xA4, 0x8C);
/// Borders, separators (dark ember).
pub const EMBER_LINE: Color32 = Color32::from_rgb(0x57, 0x47, 0x36);
/// Banner wash (dark ember).
pub const EMBER_DANGER_WASH: Color32 = Color32::from_rgb(0x4A, 0x23, 0x1C);

/// Button corner radius.
pub const BUTTON_ROUNDING: u8 = 8;
/// Card/bubble/panel corner radius.
pub const FRAME_ROUNDING: u8 = 10;

/// Full warm [`Visuals`]: washes, widget states, selection, no shadows.
#[must_use]
pub fn visuals(dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let (bg, panel, surface, chip, terra, deep, amber, wash, ink, line);
    if dark {
        (
            bg, panel, surface, chip, terra, deep, amber, wash, ink, line,
        ) = (
            EMBER_BG,
            EMBER_PANEL,
            EMBER_SURFACE,
            EMBER_CHIP,
            EMBER_TERRA,
            EMBER_DEEP,
            EMBER_AMBER,
            EMBER_WASH,
            EMBER_INK,
            EMBER_LINE,
        );
    } else {
        (
            bg, panel, surface, chip, terra, deep, amber, wash, ink, line,
        ) = (
            PAPER, PANEL, SURFACE, CHIP, TERRACOTTA, TERRA_DEEP, AMBER, AMBER_WASH, INK, HAIRLINE,
        );
    }
    v.window_fill = bg;
    v.panel_fill = panel;
    v.faint_bg_color = wash;
    v.extreme_bg_color = surface;
    v.code_bg_color = surface;
    v.hyperlink_color = terra;
    v.selection.bg_fill = wash;
    v.selection.stroke = Stroke::new(1.0, ink);
    v.override_text_color = Some(ink);
    v.warn_fg_color = amber;
    v.error_fg_color = DANGER;
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow::NONE;
    v.window_corner_radius = CornerRadius::same(FRAME_ROUNDING);
    v.menu_corner_radius = CornerRadius::same(BUTTON_ROUNDING);

    v.widgets.noninteractive.bg_fill = surface;
    v.widgets.noninteractive.weak_bg_fill = surface;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(BUTTON_ROUNDING);

    v.widgets.inactive.bg_fill = chip;
    v.widgets.inactive.weak_bg_fill = chip;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, line);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.inactive.corner_radius = CornerRadius::same(BUTTON_ROUNDING);

    v.widgets.hovered.bg_fill = wash;
    v.widgets.hovered.weak_bg_fill = wash;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, amber);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, ink);
    v.widgets.hovered.corner_radius = CornerRadius::same(BUTTON_ROUNDING);

    v.widgets.active.bg_fill = terra;
    v.widgets.active.weak_bg_fill = deep;
    v.widgets.active.bg_stroke = Stroke::NONE;
    v.widgets.active.fg_stroke = Stroke::new(1.0, bg);
    v.widgets.active.corner_radius = CornerRadius::same(BUTTON_ROUNDING);

    v.widgets.open.bg_fill = deep;
    v.widgets.open.weak_bg_fill = deep;
    v.widgets.open.bg_stroke = Stroke::NONE;
    v.widgets.open.fg_stroke = Stroke::new(1.0, bg);
    v.widgets.open.corner_radius = CornerRadius::same(BUTTON_ROUNDING);
    v
}

/// Apply the warm theme to this frame: full visuals plus the type scale
/// (body 15, button 14, small 12.5, heading 20) and the (8, 6) spacing.
pub fn apply(ctx: &egui::Context, dark: bool) {
    ctx.set_visuals(visuals(dark));
    ctx.all_styles_mut(|s| {
        s.text_styles
            .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        s.text_styles.insert(
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        );
        s.text_styles.insert(
            TextStyle::Small,
            FontId::new(12.5, FontFamily::Proportional),
        );
        s.text_styles.insert(
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        );
        s.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(13.5, FontFamily::Monospace),
        );
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
    });
}

/// Sidebar panel frame: tinted fill so the thread column reads as one card.
#[must_use]
pub fn sidebar_frame(dark: bool) -> egui::Frame {
    egui::Frame {
        fill: if dark { EMBER_PANEL } else { SURFACE },
        inner_margin: Margin::same(8),
        ..Default::default()
    }
}

/// Accent fill + on-accent text for Send / active controls.
#[must_use]
pub fn accent(dark: bool) -> (Color32, Color32) {
    if dark {
        (EMBER_TERRA, EMBER_BG)
    } else {
        (TERRACOTTA, PAPER)
    }
}

/// Bubble fill + body text: user right/terracotta, assistant left/surface.
#[must_use]
pub fn bubble(dark: bool, user: bool) -> (Color32, Color32) {
    if user {
        accent(dark)
    } else if dark {
        (EMBER_SURFACE, EMBER_INK)
    } else {
        (SURFACE, INK)
    }
}

/// Muted footnote/label text for the given surface (`user` = on terracotta).
#[must_use]
pub fn soft_text(dark: bool, user: bool) -> Color32 {
    if user {
        return if dark { EMBER_BG } else { PAPER };
    }
    if dark {
        EMBER_SOFT
    } else {
        INK_SOFT
    }
}

/// Hairline borders, separators, chip strokes.
#[must_use]
pub fn hairline(dark: bool) -> Color32 {
    if dark {
        EMBER_LINE
    } else {
        HAIRLINE
    }
}

/// Idle chip fill.
#[must_use]
pub fn chip_fill(dark: bool) -> Color32 {
    if dark {
        EMBER_CHIP
    } else {
        CHIP
    }
}

/// Banner wash + edge + text.
#[must_use]
pub fn danger(dark: bool) -> (Color32, Color32, Color32) {
    if dark {
        (EMBER_DANGER_WASH, DANGER, EMBER_INK)
    } else {
        (DANGER_WASH, DANGER, INK)
    }
}

/// Toast pill fill + text (dark pill on paper, lit pill on ember).
#[must_use]
pub fn toast(dark: bool) -> (Color32, Color32) {
    if dark {
        (EMBER_CHIP, EMBER_INK)
    } else {
        (INK, PAPER)
    }
}

/// Small bordered action chip (copy, …, dismiss, + new).
pub fn chip(ui: &mut egui::Ui, text: &str, dark: bool) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).small())
            .stroke(Stroke::new(1.0, hairline(dark)))
            .corner_radius(CornerRadius::same(6)),
    )
}
