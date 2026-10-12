//! Turn-event fan-out to frontends.
//!
//! The engine publishes every runner [`TurnEvent`]; each frontend holds a
//! `tokio::mpsc` receiver from [`StreamHub::subscribe`]. Slow receivers drop
//! (via `try_send`), never stall the turn.

/// Runner event re-exported so frontends subscribe to one type.
pub use computer_agent_agent::runner::TurnEvent;

use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

/// Fan-out hub: `publish` clones one event per live subscriber.
#[derive(Debug, Clone, Default)]
pub struct StreamHub {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<TurnEvent>>>>,
}

impl StreamHub {
    /// Register a frontend; `capacity` bounds its pending events.
    pub fn subscribe(&self, capacity: usize) -> mpsc::Receiver<TurnEvent> {
        let (tx, rx) = mpsc::channel(capacity);
        self.subscribers.lock().expect("stream hub lock").push(tx);
        rx
    }

    /// Deliver `event` to every live subscriber (best effort).
    pub fn publish(&self, event: TurnEvent) {
        let subs = self.subscribers.lock().expect("stream hub lock");
        Self::publish_to(subs, &event);
    }

    /// Live subscriber count (closed receivers are pruned first).
    pub fn subscriber_count(&self) -> usize {
        let mut subs = self.subscribers.lock().expect("stream hub lock");
        subs.retain(|tx| !tx.is_closed());
        subs.len()
    }

    fn publish_to(
        mut subs: std::sync::MutexGuard<'_, Vec<mpsc::Sender<TurnEvent>>>,
        event: &TurnEvent,
    ) {
        subs.retain(|tx| !tx.is_closed());
        for tx in subs.iter() {
            let _ = tx.try_send(event.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fan_out_reaches_every_subscriber() {
        let hub = StreamHub::default();
        let mut a = hub.subscribe(8);
        let mut b = hub.subscribe(8);
        assert_eq!(hub.subscriber_count(), 2);
        hub.publish(TurnEvent::Text("hi".to_string()));
        hub.publish(TurnEvent::Done {
            response: "hi".to_string(),
        });
        for rx in [&mut a, &mut b] {
            assert!(matches!(rx.recv().await, Some(TurnEvent::Text(_))));
            assert!(matches!(rx.recv().await, Some(TurnEvent::Done { .. })));
        }
    }

    #[tokio::test]
    async fn closed_receivers_are_pruned() {
        let hub = StreamHub::default();
        let rx = hub.subscribe(8);
        assert_eq!(hub.subscriber_count(), 1);
        drop(rx);
        assert_eq!(hub.subscriber_count(), 0);
        hub.publish(TurnEvent::Text("lost".to_string()));
        assert_eq!(hub.subscriber_count(), 0);
    }
}
