use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct NodeHealth {
    pub id: String,
    /// Exponentially weighted moving average of recent p50 latency, ms.
    pub ewma_latency_ms: f64,
    /// In-flight requests divided by configured concurrency limit.
    pub load: f64,
    pub hit_ratio: f64,
    /// What this node's egress costs, in cents per gigabyte.
    pub cost_per_gb_cents: f64,
    pub healthy: bool,
}

impl NodeHealth {
    /// Blend a new latency sample into the average.
    ///
    /// EWMA rather than a raw last-sample: one slow request should nudge the
    /// estimate, not redirect all traffic away from a healthy node.
    pub fn observe_latency(&mut self, sample_ms: f64, alpha: f64) {
        self.ewma_latency_ms = alpha * sample_ms + (1.0 - alpha) * self.ewma_latency_ms;
    }
}

pub struct Registry {
    nodes: HashMap<String, NodeHealth>,
}

impl Registry {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    pub fn upsert(&mut self, node: NodeHealth) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Score a node. Lower is better.
    ///
    /// Latency is the base. Load is a multiplier rather than an addend,
    /// because a node at 95% capacity is not "a bit slower" — it is about to
    /// queue, and queueing is where tail latency comes from.
    fn score(&self, node: &NodeHealth) -> f64 {
        let load_penalty = 1.0 / (1.0 - node.load.min(0.95));
        node.ewma_latency_ms * load_penalty
    }

    pub fn choose(&self) -> Option<&NodeHealth> {
        self.nodes
            .values()
            .filter(|n| n.healthy)
            // Shed nodes that are effectively saturated. Sending more work to
            // a node at 90% only converts its capacity problem into everyone's
            // latency problem.
            .filter(|n| n.load < 0.90)
            .min_by(|a, b| {
                self.score(a)
                    .partial_cmp(&self.score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Last resort when every node is shed: pick the least-bad one rather than
    /// serving an error. A slow response beats a 503.
    pub fn choose_degraded(&self) -> Option<&NodeHealth> {
        self.nodes
            .values()
            .filter(|n| n.healthy)
            .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
    }
}
