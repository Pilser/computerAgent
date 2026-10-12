//! Repeated-identical-call detection (port of vendor doom-loop tracking).
//!
//! Threshold matches vendor: 3 consecutive identical `(tool, input)` calls.

/// Tracks the last tool call; reports a loop after `threshold` repeats.
#[derive(Debug, Clone)]
pub struct DoomLoop {
    last: Option<(String, String)>,
    count: usize,
    threshold: usize,
}

impl DoomLoop {
    pub fn new(threshold: usize) -> Self {
        Self {
            last: None,
            count: 0,
            threshold: threshold.max(1),
        }
    }

    /// Record a call. Returns the new consecutive-repeat count.
    pub fn record(&mut self, tool: &str, input: &str) -> usize {
        let current = (tool.to_string(), input.to_string());
        match &self.last {
            Some(prev) if *prev == current => self.count += 1,
            _ => {
                self.last = Some(current);
                self.count = 1;
            }
        }
        self.count
    }

    pub fn is_loop(&self) -> bool {
        self.count >= self.threshold
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn reset(&mut self) {
        self.last = None;
        self.count = 0;
    }
}

impl Default for DoomLoop {
    fn default() -> Self {
        Self::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_after_threshold_repeats() {
        let mut d = DoomLoop::new(3);
        assert_eq!(d.record("bash", "ls"), 1);
        assert!(!d.is_loop());
        assert_eq!(d.record("bash", "ls"), 2);
        assert!(!d.is_loop());
        assert_eq!(d.record("bash", "ls"), 3);
        assert!(d.is_loop());
    }

    #[test]
    fn different_input_resets_streak() {
        let mut d = DoomLoop::default();
        d.record("bash", "ls");
        d.record("bash", "ls");
        d.record("bash", "pwd");
        assert_eq!(d.count(), 1);
        assert!(!d.is_loop());
    }

    #[test]
    fn different_tool_resets_streak() {
        let mut d = DoomLoop::default();
        d.record("bash", "ls");
        d.record("read", "ls");
        assert_eq!(d.count(), 1);
    }
}
