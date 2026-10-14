//! Model catalog snapshot + current selection (egui-free).

/// Model ids the UI may offer plus the selected one.
#[derive(Debug, Clone)]
pub struct ModelState {
    pub models: Vec<String>,
    pub current: String,
}

impl ModelState {
    /// Snapshot from the engine catalog; falls back to `current` when empty.
    pub fn new(models: Vec<String>, current: &str) -> Self {
        Self {
            models,
            current: current.to_string(),
        }
    }

    /// Select a catalog model; `false` (no change) on unknown ids.
    pub fn set_current(&mut self, id: &str) -> bool {
        if self.models.iter().any(|m| m == id) {
            self.current = id.to_string();
            true
        } else {
            false
        }
    }
}
