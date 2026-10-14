//! Region map + density scale: the ONLY resize logic.
//!
//! Pure geometry on `f32` sizes (plus one `egui::Rect` splitter); the shell
//! calls [`regions`] once per frame before any component draws.

use super::slot::SlotId;

/// Window width below which the sidebar collapses to zero.
pub const NARROW_BREAKPOINT: f32 = 900.0;

/// Sidebar width for a window of `width` (0 when narrow/collapsed).
#[must_use]
pub fn sidebar_width(width: f32) -> f32 {
    if width < NARROW_BREAKPOINT {
        0.0
    } else {
        300.0
    }
}

/// Density scale for type/spacing (compact under the breakpoint).
#[must_use]
pub fn density_scale(width: f32) -> f32 {
    if width < NARROW_BREAKPOINT {
        0.9
    } else {
        1.0
    }
}

/// Pre-allocated rect per slot for a window rect. Banner height is the
/// measured content capped at 96px (0 = no banner, feed keeps the space).
/// The caller (panel-based shell) uses widths/heights; this map documents
/// the contract and is unit-tested.
#[derive(Debug, Clone)]
pub struct RegionMap {
    /// Slot order with exact pixel sizes along the main axis.
    pub columns: Vec<(SlotId, f32)>,
    /// Banner height actually reserved (0 when no banner is live).
    pub banner_height: f32,
}

/// Column widths + banner height for `width` (banner content capped at 96).
#[must_use]
pub fn regions(width: f32, banner_content: f32, has_banner: bool) -> RegionMap {
    let sidebar = sidebar_width(width);
    let banner_height = if has_banner {
        banner_content.min(96.0)
    } else {
        0.0
    };
    RegionMap {
        columns: vec![
            (SlotId::Sidebar, sidebar),
            (SlotId::Feed, (width - sidebar).max(0.0)),
        ],
        banner_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_collapses_when_narrow() {
        assert_eq!(sidebar_width(899.0), 0.0);
        assert_eq!(sidebar_width(900.0), 300.0);
        let map = regions(800.0, 40.0, true);
        assert_eq!(map.columns[0], (SlotId::Sidebar, 0.0));
        assert_eq!(map.columns[1], (SlotId::Feed, 800.0));
    }

    #[test]
    fn banner_height_caps_at_96() {
        assert_eq!(regions(1200.0, 200.0, true).banner_height, 96.0);
        assert_eq!(regions(1200.0, 40.0, true).banner_height, 40.0);
        assert_eq!(regions(1200.0, 40.0, false).banner_height, 0.0);
    }

    #[test]
    fn wide_window_keeps_sidebar() {
        let map = regions(1400.0, 0.0, false);
        assert_eq!(map.columns[0], (SlotId::Sidebar, 300.0));
        assert_eq!(map.columns[1], (SlotId::Feed, 1100.0));
        assert_eq!(density_scale(1400.0), 1.0);
        assert_eq!(density_scale(800.0), 0.9);
    }
}
