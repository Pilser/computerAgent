//! Context-window usage + spend rollup for the active thread (egui-free).

/// Token/context snapshot driving the usage bar and cost label.
#[derive(Debug, Clone, Copy, Default)]
pub struct UsageState {
    pub used: u64,
    pub window: u64,
    pub cost_cents: i32,
}

impl UsageState {
    /// Build a snapshot from raw counters.
    pub fn new(used: u64, window: u64, cost_cents: i32) -> Self {
        Self {
            used,
            window,
            cost_cents,
        }
    }

    /// `used / window` clamped to `0..=1` (zero window reads as empty).
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.window == 0 {
            return 0.0;
        }
        (self.used as f32 / self.window as f32).clamp(0.0, 1.0)
    }

    /// `"<used> / <window> tok"` for the context label.
    #[must_use]
    pub fn ctx_label(&self) -> String {
        format!("{} / {} tok", self.used, self.window)
    }

    /// `"$d.cc"` cost label (mirrors the engine thread formatting).
    #[must_use]
    pub fn cost_label(&self) -> String {
        let dollars = f64::from(self.cost_cents) / 100.0;
        if dollars < 1.0 {
            format!("${dollars:.4}")
        } else {
            format!("${dollars:.2}")
        }
    }
}
