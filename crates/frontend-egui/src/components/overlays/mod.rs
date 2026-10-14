//! Overlay plug-ins: approval dialog, toasts, palette, message menu.
//!
//! Re-export only; one folder per overlay, one `register()` line each.

pub mod approval;
pub mod menu;
pub mod palette;
pub mod toast;
