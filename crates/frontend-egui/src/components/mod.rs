//! Registered plug-ins: base slots first, overlays last.
//!
//! One `register()` line per component; adding a plug-in touches nothing
//! existing. Phase 1 bodies are stubs (empty rects + id label); real
//! rendering lands in phases 2–4.

pub mod banner;
pub mod composer;
pub mod feed;
pub mod overlays;
pub mod sidebar;
pub mod topbar;

use crate::shell::Registry;

/// Register every component, base slots first, overlays last.
pub fn register_all(registry: &mut Registry) {
    registry.register(topbar::Topbar::new());
    registry.register(sidebar::Sidebar::new());
    registry.register(feed::Feed::new());
    registry.register(composer::Composer::new());
    registry.register(banner::Banner::new());
    registry.register(overlays::approval::Approval::new());
    registry.register(overlays::toast::Toast::new());
    registry.register(overlays::palette::Palette::new());
    registry.register(overlays::menu::Menu::new());
}
