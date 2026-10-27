//! Typed core events and the bus that carries them to the app (architecture §5).
//!
//! The bus is a `tokio::sync::broadcast` channel, which needs no runtime: core threads publish
//! synchronously, the Tauri side awaits [`Subscription::recv`]. A subscriber that falls more
//! than [`BUS_CAPACITY`] events behind loses the oldest ones. It then receives
//! [`Delivery::Resync`] once and must rebuild its state from a full query instead of trying to
//! patch it from the events it still has.

use serde::Serialize;
use specta::Type;
use tokio::sync::broadcast;

use crate::domain::{AccountId, SessionId, Timestamp};
use crate::indexer::{IndexMilestone, IndexProgress};
use crate::limits::{LimitEvent, PanelSnapshot};
use crate::live_sessions::LiveSession;
use crate::notifier::Notice;

/// Events buffered per subscriber before the oldest are dropped.
pub const BUS_CAPACITY: usize = 256;

/// Something in the core changed. New variants may be added in any release.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CoreEvent {
    /// Initial or safety-net indexing progressed. Incremental passes do not report progress.
    IndexProgress {
        account: AccountId,
        progress: IndexProgress,
    },
    IndexMilestone {
        account: AccountId,
        milestone: IndexMilestone,
    },
    /// New usage events were stored for these sessions, timestamped within `from_ts..=to_ts`.
    UsageAppended {
        account: AccountId,
        sessions: Vec<SessionId>,
        from_ts: Timestamp,
        to_ts: Timestamp,
    },
    /// The complete list of running Claude Code sessions, across all accounts.
    LiveSessionsChanged(Vec<LiveSession>),
    /// The modification time of `.credentials.json` changed. Its content is never read here.
    CredentialsChanged { account: AccountId },
    /// An indexing pass met records it could not use; the provider's format may have changed.
    SchemaDrift {
        source: DriftSource,
        unknown_count: u64,
    },
    /// The panel's view model changed visibly: a rounded percentage, a state, a source or a
    /// freshness level. Also the full state to reload after `Resync`.
    LimitsUpdated(Box<PanelSnapshot>),
    /// A threshold, projection or reset signal for the notifier.
    Limit(LimitEvent),
    /// A notification to show now; mute and quiet hours are already applied.
    Notify(Notice),
}

/// Which parser reported [`CoreEvent::SchemaDrift`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DriftSource {
    ClaudeJsonl,
}

/// What a subscriber receives.
#[derive(Debug, Clone, PartialEq)]
pub enum Delivery {
    Event(CoreEvent),
    /// `missed` events were dropped because this subscriber lagged. Refresh the full state.
    Resync {
        missed: u64,
    },
}

/// Publishing side of the bus. Cheap to clone.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<CoreEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::with_capacity(BUS_CAPACITY)
    }
}

impl EventBus {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tx: broadcast::channel(capacity).0,
        }
    }

    /// Sends to every current subscriber. Having none is not an error.
    pub fn publish(&self, event: CoreEvent) {
        tracing::trace!(?event, "core event");
        let _ = self.tx.send(event);
    }

    /// A subscription that receives every event published from now on.
    pub fn subscribe(&self) -> Subscription {
        Subscription {
            rx: self.tx.subscribe(),
        }
    }
}

/// Receiving side of the bus.
#[derive(Debug)]
pub struct Subscription {
    rx: broadcast::Receiver<CoreEvent>,
}

impl Subscription {
    /// The next delivery, or `None` once the bus is gone (the runtime shut down).
    pub async fn recv(&mut self) -> Option<Delivery> {
        Self::map(self.rx.recv().await)
    }

    /// Blocking [`Subscription::recv`] for plain threads. Panics inside an async context.
    pub fn blocking_recv(&mut self) -> Option<Delivery> {
        Self::map(self.rx.blocking_recv())
    }

    fn map(res: Result<CoreEvent, broadcast::error::RecvError>) -> Option<Delivery> {
        match res {
            Ok(event) => Some(Delivery::Event(event)),
            Err(broadcast::error::RecvError::Lagged(missed)) => Some(Delivery::Resync { missed }),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(account: i64) -> CoreEvent {
        CoreEvent::CredentialsChanged {
            account: AccountId(account),
        }
    }

    #[test]
    fn subscribers_receive_events_published_after_subscribing() {
        let bus = EventBus::default();
        bus.publish(credentials(0)); // nobody listens yet: dropped, not an error
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        bus.publish(credentials(1));
        assert_eq!(a.blocking_recv(), Some(Delivery::Event(credentials(1))));
        assert_eq!(b.blocking_recv(), Some(Delivery::Event(credentials(1))));
        drop(bus);
        assert_eq!(a.blocking_recv(), None);
    }

    /// The lag policy: a consumer that keeps a derived view (here, the last account that
    /// changed) never patches it from a gap. On `Resync` it rebuilds from the source of truth,
    /// then keeps applying events.
    #[test]
    fn a_lagging_subscriber_resyncs_from_a_full_query() {
        struct View {
            last_account: Option<i64>,
            resyncs: u32,
        }
        impl View {
            fn apply(&mut self, delivery: &Delivery, full_state: impl Fn() -> Option<i64>) {
                match delivery {
                    Delivery::Event(CoreEvent::CredentialsChanged { account }) => {
                        self.last_account = Some(account.0);
                    }
                    Delivery::Event(_) => {}
                    Delivery::Resync { .. } => {
                        self.resyncs += 1;
                        self.last_account = full_state();
                    }
                }
            }
        }

        let bus = EventBus::with_capacity(4);
        let mut sub = bus.subscribe();
        for account in 1..=10 {
            bus.publish(credentials(account));
        }
        // Capacity 4: events 1-6 are gone; the subscriber learns how many it missed.
        let first = sub.blocking_recv().expect("open");
        assert_eq!(first, Delivery::Resync { missed: 6 });

        let mut view = View {
            last_account: None,
            resyncs: 0,
        };
        view.apply(&first, || Some(10));
        assert_eq!((view.last_account, view.resyncs), (Some(10), 1));
        // The buffered tail still arrives and is applied on top of the refreshed state.
        for expected in 7..=10 {
            let delivery = sub.blocking_recv().expect("open");
            assert_eq!(delivery, Delivery::Event(credentials(expected)));
            view.apply(&delivery, || unreachable!("no further resync"));
        }
        assert_eq!((view.last_account, view.resyncs), (Some(10), 1));
    }

    #[test]
    fn events_serialize_with_a_type_tag() {
        let json = serde_json::to_value(CoreEvent::UsageAppended {
            account: AccountId(1),
            sessions: vec![SessionId(7)],
            from_ts: Timestamp::from_millis(10),
            to_ts: Timestamp::from_millis(20),
        })
        .expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "type": "usage_appended",
                "data": { "account": 1, "sessions": [7], "from_ts": 10, "to_ts": 20 }
            })
        );
    }
}
