use std::collections::BTreeMap;

/// Build the cache key for a request.
///
/// The cache key decides correctness before it decides performance. Too
/// coarse and you serve one user's response to another. Too fine and the hit
/// ratio collapses because no two requests ever share a key.
pub fn cache_key(path: &str, query: &str, accept_encoding: Option<&str>) -> String {
    // Sort query parameters. `?a=1&b=2` and `?b=2&a=1` are the same resource,
    // and treating them as different keys halves the hit ratio for no reason.
    let mut params: BTreeMap<&str, &str> = BTreeMap::new();
    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        // Drop tracking parameters: they change per visitor and never change
        // the response body, so including them guarantees a miss every time.
        if k.starts_with("utm_") || k == "fbclid" || k == "gclid" {
            continue;
        }
        params.insert(k, v);
    }

    let normalized_query: Vec<String> =
        params.iter().map(|(k, v)| format!("{k}={v}")).collect();

    // Encoding belongs in the key: a gzip body served to a client that did
    // not ask for gzip is a broken response, not a slow one.
    let enc = match accept_encoding {
        Some(a) if a.contains("br") => "br",
        Some(a) if a.contains("gzip") => "gzip",
        _ => "identity",
    };

    format!("{path}?{}|enc={enc}", normalized_query.join("&"))
}

/// Decide whether a request may be served from cache at all.
pub fn is_cacheable_request(method: &str, headers: &BTreeMap<String, String>) -> bool {
    if method != "GET" && method != "HEAD" {
        return false;
    }
    // An authenticated request may return user-specific content. Caching it
    // risks serving one user's data to another, which is the one failure a
    // cache must never have.
    if headers.contains_key("authorization") || headers.contains_key("cookie") {
        return false;
    }
    true
}

/// A bounded concurrency gate.
///
/// Without a limit, an overloaded node accepts every connection and slows
/// down for everyone. With one, it stays fast for the requests it accepts and
/// signals the steering layer to send work elsewhere.
pub struct ConcurrencyGate {
    limit: u64,
    in_flight: std::sync::atomic::AtomicU64,
}

impl ConcurrencyGate {
    pub fn new(limit: u64) -> Self {
        Self { limit, in_flight: std::sync::atomic::AtomicU64::new(0) }
    }

    pub fn try_enter(&self) -> bool {
        use std::sync::atomic::Ordering;
        let current = self.in_flight.load(Ordering::Relaxed);
        if current >= self.limit {
            return false;
        }
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn leave(&self) {
        use std::sync::atomic::Ordering;
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn load(&self) -> f64 {
        use std::sync::atomic::Ordering;
        self.in_flight.load(Ordering::Relaxed) as f64 / self.limit.max(1) as f64
    }
}
