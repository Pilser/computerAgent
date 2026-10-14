//! Overlay stack: modality, queuing, and input routing live HERE,
//! never in components. Overlays never resize base slots.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How an overlay treats the layers below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// Dims below, consumes all input (approval dialog, palette).
    Modal,
    /// Timed, pass-through input ("copied", send errors).
    Toast,
    /// Anchored to cursor/widget, dismiss on click-elsewhere.
    Menu,
}

/// Where the shell positions an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Window center.
    Center,
    /// Last pointer position.
    Cursor,
    /// Bottom-right corner (toasts).
    DockBottomRight,
}

/// One transient notice with its expiry.
#[derive(Debug, Clone)]
struct Toast {
    text: String,
    expires: Instant,
}

/// Overlay ownership: at most ONE modal at a time (a second queues);
/// toasts expire on their own; menus are fire-and-forget markers.
#[derive(Debug, Default)]
pub struct OverlayStack {
    modal: Option<String>,
    queued: VecDeque<String>,
    toasts: Vec<Toast>,
    menu: Option<String>,
}

impl OverlayStack {
    /// Empty stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request a modal overlay. `true` = shown now; `false` = queued
    /// behind the live one.
    pub fn show_modal(&mut self, id: &str) -> bool {
        if self.modal.is_none() {
            self.modal = Some(id.to_string());
            true
        } else {
            self.queued.push_back(id.to_string());
            false
        }
    }

    /// Live modal id, if any.
    #[must_use]
    pub fn modal(&self) -> Option<&str> {
        self.modal.as_deref()
    }

    /// Dismiss the live modal; promotes the next queued one.
    pub fn dismiss_modal(&mut self) {
        self.modal = self.queued.pop_front();
    }

    /// Queue a toast for `ttl`.
    pub fn push_toast(&mut self, text: &str, ttl: Duration) {
        self.toasts.push(Toast {
            text: text.to_string(),
            expires: Instant::now() + ttl,
        });
    }

    /// Live toast texts, oldest first (expired ones pruned first).
    pub fn toasts(&mut self) -> Vec<String> {
        self.prune_toasts();
        self.toasts.iter().map(|t| t.text.clone()).collect()
    }

    /// Mark an anchored menu live (replaces any previous one).
    pub fn show_menu(&mut self, id: &str) {
        self.menu = Some(id.to_string());
    }

    /// Live menu id, if any.
    #[must_use]
    pub fn menu(&self) -> Option<&str> {
        self.menu.as_deref()
    }

    /// Dismiss the live menu (click-elsewhere path).
    pub fn dismiss_menu(&mut self) {
        self.menu = None;
    }

    fn prune_toasts(&mut self) {
        let now = Instant::now();
        self.toasts.retain(|t| t.expires > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_modal_queues_until_dismiss() {
        let mut stack = OverlayStack::new();
        assert!(stack.show_modal("approval"));
        assert!(!stack.show_modal("palette"));
        assert_eq!(stack.modal(), Some("approval"));
        stack.dismiss_modal();
        assert_eq!(stack.modal(), Some("palette"));
        stack.dismiss_modal();
        assert_eq!(stack.modal(), None);
    }

    #[test]
    fn expired_toasts_drop_out() {
        let mut stack = OverlayStack::new();
        stack.push_toast("old", Duration::from_nanos(0));
        std::thread::sleep(Duration::from_millis(10));
        stack.push_toast("fresh", Duration::from_secs(60));
        assert_eq!(stack.toasts(), vec!["fresh".to_string()]);
    }

    #[test]
    fn menu_replaces_and_dismisses() {
        let mut stack = OverlayStack::new();
        stack.show_menu("msg-1");
        stack.show_menu("msg-2");
        assert_eq!(stack.menu(), Some("msg-2"));
        stack.dismiss_menu();
        assert_eq!(stack.menu(), None);
    }
}
