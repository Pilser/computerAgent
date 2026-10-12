//! Cooperative cancellation tokens, one per in-flight run.
//!
//! The engine registers a token when a turn starts and removes it when the
//! turn settles; `AgentCancel` flips the flag via [`CancelMap::cancel`].
//! The v1 runner stub is synchronous so nothing polls the flag yet — the
//! map still owns the lifecycle so the real runner can check it later.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Shared cancel flag for one run (`true` = cancellation requested).
pub type CancelToken = Arc<AtomicBool>;

/// `run_id` -> [`CancelToken`]. Removed on settle or cancel.
#[derive(Debug, Default)]
pub struct CancelMap {
    inner: Mutex<HashMap<String, CancelToken>>,
}

impl CancelMap {
    /// Empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a fresh (unset) token for `run_id`, replacing any stale one.
    pub fn track(&self, run_id: &str) -> CancelToken {
        let token = Arc::new(AtomicBool::new(false));
        self.inner
            .lock()
            .expect("cancel lock")
            .insert(run_id.to_string(), Arc::clone(&token));
        token
    }

    /// Request cancellation: flips the flag and drops the entry.
    /// Returns false when `run_id` was unknown (already settled).
    pub fn cancel(&self, run_id: &str) -> bool {
        match self.inner.lock().expect("cancel lock").remove(run_id) {
            Some(token) => {
                token.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    /// Drop the entry without signalling (normal turn settle).
    pub fn finish(&self, run_id: &str) {
        self.inner.lock().expect("cancel lock").remove(run_id);
    }

    /// Flip every live token and drop all entries (Stop button path).
    /// Returns the number of runs that were in flight.
    pub fn cancel_all(&self) -> usize {
        let mut inner = self.inner.lock().expect("cancel lock");
        let live = inner.len();
        for (_, token) in inner.drain() {
            token.store(true, Ordering::SeqCst);
        }
        live
    }

    /// True while `run_id` has a live (unsettled, uncancelled) entry.
    pub fn contains(&self, run_id: &str) -> bool {
        self.inner.lock().expect("cancel lock").contains_key(run_id)
    }

    /// True when the token's flag was flipped by [`CancelMap::cancel`].
    pub fn is_cancelled(token: &CancelToken) -> bool {
        token.load(Ordering::SeqCst)
    }

    /// Live entry count.
    pub fn len(&self) -> usize {
        self.inner.lock().expect("cancel lock").len()
    }

    /// True when no run is in flight.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_flips_flag_and_removes_token() {
        let map = CancelMap::new();
        let token = map.track("r1");
        assert!(map.contains("r1"));
        assert!(!CancelMap::is_cancelled(&token));
        assert!(map.cancel("r1"));
        assert!(CancelMap::is_cancelled(&token));
        assert!(!map.contains("r1"));
        assert!(map.is_empty());
        // Second cancel: already settled, reports false.
        assert!(!map.cancel("r1"));
    }

    #[test]
    fn finish_removes_without_signalling() {
        let map = CancelMap::new();
        let token = map.track("r2");
        map.finish("r2");
        assert!(!CancelMap::is_cancelled(&token));
        assert!(map.is_empty());
    }

    #[test]
    fn cancel_all_flips_every_live_token() {
        let map = CancelMap::new();
        let a = map.track("a");
        let b = map.track("b");
        assert_eq!(map.cancel_all(), 2);
        assert!(CancelMap::is_cancelled(&a));
        assert!(CancelMap::is_cancelled(&b));
        assert!(map.is_empty());
        assert_eq!(map.cancel_all(), 0);
    }
}
