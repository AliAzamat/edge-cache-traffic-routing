use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::cache::Entry;

/// Ensures only one origin fetch happens per key at a time.
///
/// The failure this prevents: a popular object expires, and every concurrent
/// request for it misses simultaneously. Without coalescing, a thousand
/// requests become a thousand origin fetches in the same instant. The origin
/// is sized for your MISS rate, not your request rate, so it falls over.
pub struct SingleFlight {
    in_flight: Mutex<HashMap<String, broadcast::Sender<Arc<Entry>>>>,
}

impl SingleFlight {
    pub fn new() -> Self {
        Self { in_flight: Mutex::new(HashMap::new()) }
    }

    /// Run `fetch` for `key`, or wait for the fetch already running.
    pub async fn fetch<F, Fut>(&self, key: String, fetch: F) -> Result<Arc<Entry>, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Entry, String>>,
    {
        // Subscribe under the lock, so a fetch cannot complete between our
        // check and our subscription. That gap is the classic coalescing bug:
        // the waiter subscribes just after the result was broadcast and then
        // waits forever.
        let existing = {
            let map = self.in_flight.lock().await;
            map.get(&key).map(|tx| tx.subscribe())
        };

        if let Some(mut rx) = existing {
            return rx.recv().await.map_err(|_| "leader failed".to_string());
        }

        let (tx, _) = broadcast::channel(1);
        {
            let mut map = self.in_flight.lock().await;
            // Re-check: another task may have become leader while we waited
            // for the lock.
            if let Some(tx) = map.get(&key) {
                let mut rx = tx.subscribe();
                drop(map);
                return rx.recv().await.map_err(|_| "leader failed".to_string());
            }
            map.insert(key.clone(), tx.clone());
        }

        let result = fetch().await;

        // Remove BEFORE broadcasting, so the next request after this one
        // starts a fresh fetch instead of joining a finished flight.
        self.in_flight.lock().await.remove(&key);

        match result {
            Ok(entry) => {
                let shared = Arc::new(entry);
                let _ = tx.send(shared.clone()); // no receivers is fine
                Ok(shared)
            }
            Err(e) => Err(e),
        }
    }
}

/// Serve the stale copy while refreshing in the background.
///
/// Coalescing makes one request pay the origin latency. This makes zero pay
/// it: everyone gets the slightly-stale object instantly, and the refresh
/// happens out of band.
pub fn should_serve_stale_while_revalidating(
    entry: &Entry,
    now: u64,
    stale_tolerance_secs: u64,
) -> bool {
    now > entry.expires_at && now - entry.expires_at <= stale_tolerance_secs
}
