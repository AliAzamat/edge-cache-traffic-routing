use crate::cache::Stats;
use std::fmt::Write;

/// Prometheus text exposition for one node.
///
/// These metrics have two audiences. A human reads them during an incident,
/// and the steering layer reads them to make routing decisions. The second
/// audience is why hit ratio is exported rather than kept internal.
pub struct NodeMetrics {
    pub node_id: String,
    pub region: String,
    pub stats: Stats,
    pub ewma_latency_ms: f64,
    pub in_flight: u64,
    pub concurrency_limit: u64,
    pub origin_bytes_total: u64,
    pub cost_per_gb_cents: f64,
}

impl NodeMetrics {
    pub fn render(&self) -> String {
        let mut out = String::new();
        // Labels are node id and region only. Never the cache KEY: an
        // unbounded label set creates one time series per URL and takes down
        // the metrics backend faster than any traffic spike.
        let l = format!(r#"node="{}",region="{}""#, self.node_id, self.region);

        let _ = writeln!(out, "# TYPE cache_hits_total counter");
        let _ = writeln!(out, "cache_hits_total{{{l}}} {}", self.stats.hits);

        let _ = writeln!(out, "# TYPE cache_misses_total counter");
        let _ = writeln!(out, "cache_misses_total{{{l}}} {}", self.stats.misses);

        let _ = writeln!(out, "# TYPE cache_revalidations_total counter");
        let _ = writeln!(out, "cache_revalidations_total{{{l}}} {}", self.stats.revalidations);

        let _ = writeln!(out, "# TYPE cache_evictions_total counter");
        let _ = writeln!(out, "cache_evictions_total{{{l}}} {}", self.stats.evictions);

        // Origin bytes is the number that becomes the bandwidth bill.
        let _ = writeln!(out, "# TYPE origin_bytes_total counter");
        let _ = writeln!(out, "origin_bytes_total{{{l}}} {}", self.origin_bytes_total);

        // Gauges: a point-in-time reading, not an accumulation.
        let _ = writeln!(out, "# TYPE node_latency_ewma_ms gauge");
        let _ = writeln!(out, "node_latency_ewma_ms{{{l}}} {:.2}", self.ewma_latency_ms);

        let load = self.in_flight as f64 / self.concurrency_limit.max(1) as f64;
        let _ = writeln!(out, "# TYPE node_load_ratio gauge");
        let _ = writeln!(out, "node_load_ratio{{{l}}} {:.4}", load);

        let _ = writeln!(out, "# TYPE node_cost_per_gb_cents gauge");
        let _ = writeln!(out, "node_cost_per_gb_cents{{{l}}} {:.4}", self.cost_per_gb_cents);

        out
    }
}
