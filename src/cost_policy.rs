use crate::steering::NodeHealth;

/// A routing policy with an explicit latency budget.
///
/// Pure latency minimization always picks the fastest node and ignores the
/// bill. Pure cost minimization picks the cheapest and ignores the user. The
/// useful policy is: satisfy the latency budget, then minimize cost among the
/// nodes that qualify.
pub struct CostPolicy {
    /// Requests should complete within this budget, in milliseconds.
    pub latency_budget_ms: f64,
    /// Average object size, used to convert a per-GB price into a per-request one.
    pub avg_object_bytes: f64,
}

impl CostPolicy {
    /// Expected cost of serving one request from this node, in cents.
    ///
    /// A cache hit costs the node's egress. A miss ALSO pulls from origin, so
    /// a low-hit-ratio node is more expensive per request even at the same
    /// advertised price per gigabyte.
    pub fn expected_cost_cents(&self, node: &NodeHealth, origin_cost_per_gb_cents: f64) -> f64 {
        let gb = self.avg_object_bytes / 1_073_741_824.0;
        let egress = node.cost_per_gb_cents * gb;
        let origin_pull = (1.0 - node.hit_ratio) * origin_cost_per_gb_cents * gb;
        egress + origin_pull
    }

    /// Choose the cheapest node that still meets the latency budget.
    pub fn choose<'a>(
        &self,
        nodes: &'a [NodeHealth],
        origin_cost_per_gb_cents: f64,
    ) -> Option<&'a NodeHealth> {
        let qualifying: Vec<&NodeHealth> = nodes
            .iter()
            .filter(|n| n.healthy && n.load < 0.90)
            .filter(|n| n.ewma_latency_ms <= self.latency_budget_ms)
            .collect();

        if !qualifying.is_empty() {
            return qualifying.into_iter().min_by(|a, b| {
                self.expected_cost_cents(a, origin_cost_per_gb_cents)
                    .partial_cmp(&self.expected_cost_cents(b, origin_cost_per_gb_cents))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Nothing meets the budget. Fall back to fastest: when the user
        // experience is already degraded, cost stops being the priority.
        nodes
            .iter()
            .filter(|n| n.healthy)
            .min_by(|a, b| {
                a.ewma_latency_ms
                    .partial_cmp(&b.ewma_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}
