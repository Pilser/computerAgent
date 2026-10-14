//! Warm theme: paper background, terracotta accents, amber selection.
//!
//! The ONLY styling entry point: components use ambient visuals, never
//! hard-coded colors.

/// Paper wash (window background).
pub const PAPER: egui::Color32 = egui::Color32::from_rgb(0xFA, 0xF6, 0xEF);
/// Panel fill, a shade deeper than paper.
pub const PAPER_DEEP: egui::Color32 = egui::Color32::from_rgb(0xF1, 0xE8, 0xD8);
/// Terracotta accent (links, active controls).
pub const TERRACOTTA: egui::Color32 = egui::Color32::from_rgb(0xC0, 0x5C, 0x33);
/// Amber selection wash.
pub const AMBER: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xA8, 0x3D);
/// Ink text on paper.
pub const INK: egui::Color32 = egui::Color32::from_rgb(0x2B, 0x26, 0x22);

/// Warm [`egui::Visuals`] over the light preset.
#[must_use]
pub fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.window_fill = PAPER;
    v.panel_fill = PAPER_DEEP;
    v.hyperlink_color = TERRACOTTA;
    v.selection.bg_fill = AMBER;
    v.override_text_color = Some(INK);
    v
}

/// Apply the warm theme to this frame's context.
pub fn apply(ctx: &egui::Context) {
    ctx.set_visuals(visuals());
}
