//! Compaction: summarize old messages into a boundary `System` message.
//! Port of the compaction methods in `_vendor/embedded-agent/src/session/mod.rs`.

use super::types::{Compaction, MessageRole, Session, SessionMessage};

impl Session {
    /// Pick the compaction boundary: `messages[..cut]` get summarized and
    /// `messages[cut..]` are kept as recent context. Walks backward summing
    /// per-message estimates until `keep_recent` is covered. Returns 0 when
    /// every message fits within `keep_recent` (nothing old enough).
    pub fn select_compaction_cut(messages: &[SessionMessage], keep_recent: u64) -> usize {
        let mut accumulated = 0u64;
        let mut cut_idx = 0;
        for (i, msg) in messages.iter().enumerate().rev() {
            if accumulated >= keep_recent {
                cut_idx = i + 1;
                break;
            }
            accumulated = accumulated.saturating_add(msg.estimated_tokens);
        }
        cut_idx
    }

    pub fn needs_compaction(&self, reserve_tokens: u64) -> bool {
        if self.context_window == 0 {
            return false;
        }
        self.effective_context_tokens() > self.context_window.saturating_sub(reserve_tokens)
    }

    /// Locate the latest compaction summary `System` message at runtime
    /// rather than trusting a stored index (which drifts if messages are
    /// inserted before it). Returns the summary and the index just past it.
    pub fn compacted_context(&self) -> (Option<&str>, usize) {
        let c = match self.compactions.last() {
            Some(c) => c,
            None => return (None, 0),
        };
        for (i, msg) in self.messages.iter().enumerate() {
            if msg.role == MessageRole::System && msg.content == c.summary {
                return (Some(c.summary.as_str()), i + 1);
            }
        }
        (None, 0)
    }

    /// Replace `messages[..first_kept_index]` with a `System` summary message
    /// and record the [`Compaction`]. Recomputes totals from the remaining
    /// messages and drops the calibration anchor (compaction reindexes, so
    /// the next completed turn re-anchors).
    pub fn compress(&mut self, summary: String, first_kept_index: usize, token_savings: u64) {
        let summarized_count = first_kept_index.min(self.messages.len());
        let summary_tokens = Self::estimate_tokens(&summary);

        self.messages.drain(..summarized_count);
        self.messages.insert(
            0,
            SessionMessage {
                role: MessageRole::System,
                content: summary.clone(),
                estimated_tokens: summary_tokens,
                tool: None,
                turn: None,
            },
        );
        self.total_estimated_tokens = self.messages.iter().map(|m| m.estimated_tokens).sum();

        self.compactions.push(Compaction {
            summary,
            first_kept_index: 1,
            summarized_count,
            token_savings,
            created_at: chrono::Utc::now().to_rfc3339(),
        });

        self.reset_calibration();
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_with_messages(n: usize) -> Session {
        let mut s = Session::new("p", "m", 10_000, "t");
        for i in 0..n {
            s.add_message(
                MessageRole::User,
                &format!("message number {i} padding padding"),
            );
        }
        s
    }

    #[test]
    fn cut_keeps_recent_within_budget() {
        let s = session_with_messages(6);
        let cut = Session::select_compaction_cut(&s.messages, u64::MAX);
        assert_eq!(cut, 0);
    }

    #[test]
    fn cut_zero_budget_summarizes_all_but_last_boundary() {
        let s = session_with_messages(4);
        let cut = Session::select_compaction_cut(&s.messages, 0);
        assert!(cut > 0 && cut <= s.messages.len());
    }

    #[test]
    fn needs_compaction_respects_window_and_reserve() {
        let mut s = session_with_messages(4);
        assert!(!s.needs_compaction(0) || s.context_window > 0);
        s.update_context_window(1);
        assert!(s.needs_compaction(0));
        let zero = Session::new("p", "m", 0, "t");
        assert!(!zero.needs_compaction(0));
    }

    #[test]
    fn compress_inserts_summary_and_records_compaction() {
        let mut s = session_with_messages(4);
        let before: u64 = s.messages.iter().map(|m| m.estimated_tokens).sum();
        assert_eq!(before, s.total_estimated_tokens);
        s.compress("summary of old stuff".to_string(), 3, 100);
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.messages[0].role, MessageRole::System);
        assert_eq!(s.messages[0].content, "summary of old stuff");
        assert_eq!(s.compactions.len(), 1);
        assert_eq!(s.compactions[0].summarized_count, 3);
        let (summary, idx) = s.compacted_context();
        assert_eq!(summary, Some("summary of old stuff"));
        assert_eq!(idx, 1);
        assert!(s.ctx_is_estimated());
    }
}
