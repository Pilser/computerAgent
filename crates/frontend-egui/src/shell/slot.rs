//! Slot ids, sizing policies, and pre-allocated rects.
//!
//! Policies are computed ONCE per frame, BEFORE any component draws, so a
//! component physically cannot move its siblings.

/// Every base region the shell partitions the window into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotId {
    /// Model picker + thread controls (top strip).
    Topbar,
    /// Thread list + search (left strip).
    Sidebar,
    /// Message bubbles (all leftover space).
    Feed,
    /// Draft input + send (bottom strip).
    Composer,
    /// Error/status notice (below the topbar, capped height).
    Banner,
}

/// How the shell sizes one slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlotPolicy {
    /// Exact pixels (sidebar collapses to zero under 900px windows).
    Fixed(f32),
    /// Share of leftover space after every `Fixed` slot is placed.
    Flex(f32),
    /// Natural height, capped at the given pixels.
    ContentCapped(f32),
}

/// Sizing policy per slot (see `ARCHITECTURE.md` §2 for the numbers).
#[must_use]
pub const fn policy(slot: SlotId) -> SlotPolicy {
    match slot {
        SlotId::Topbar => SlotPolicy::Fixed(48.0),
        SlotId::Sidebar => SlotPolicy::Fixed(300.0),
        SlotId::Feed => SlotPolicy::Flex(1.0),
        SlotId::Composer => SlotPolicy::Fixed(132.0),
        SlotId::Banner => SlotPolicy::ContentCapped(96.0),
    }
}

/// Child `Ui` scope handed to one component: its slot plus the rect the
/// shell pre-allocated for it.
#[derive(Debug, Clone, Copy)]
pub struct Allocated {
    /// Which shell region this rect belongs to.
    pub slot: SlotId,
    /// Pre-computed rect; components draw inside it only.
    pub rect: egui::Rect,
}

impl Allocated {
    /// Wrap a live rect for a slot.
    #[must_use]
    pub const fn new(slot: SlotId, rect: egui::Rect) -> Self {
        Self { slot, rect }
    }
}
