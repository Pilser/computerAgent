//! Status line + error banner queue (egui-free).

use std::collections::VecDeque;

/// Status text/ok flag plus a FIFO of banner notices.
#[derive(Debug, Default)]
pub struct StatusState {
    pub ok: bool,
    pub text: String,
    banners: VecDeque<String>,
}

impl StatusState {
    /// Idle state: ok, empty text, no banners.
    pub fn new() -> Self {
        Self {
            ok: true,
            text: String::new(),
            banners: VecDeque::new(),
        }
    }

    /// Healthy status line (does not touch the banner queue).
    pub fn set_ok(&mut self, text: &str) {
        self.ok = true;
        self.text = text.to_string();
    }

    /// Error status line; also surfaces as a dismissible banner.
    pub fn set_err(&mut self, text: &str) {
        self.ok = false;
        self.text = text.to_string();
        self.banners.push_back(text.to_string());
    }

    /// Queue a banner without changing the status line.
    pub fn push_banner(&mut self, text: &str) {
        self.banners.push_back(text.to_string());
    }

    /// Oldest banner (the one the banner slot shows).
    #[must_use]
    pub fn banner_current(&self) -> Option<&str> {
        self.banners.front().map(String::as_str)
    }

    /// Dismiss the oldest banner; `false` when the queue was empty.
    pub fn dismiss_banner(&mut self) -> bool {
        self.banners.pop_front().is_some()
    }
}
