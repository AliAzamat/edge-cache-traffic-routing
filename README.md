# Edge Caching & Traffic Routing — Decide Where the Byte Comes From

An intermediate systems project about the decision at the heart of content delivery: which node should serve this request. You build a cache node with a real eviction policy and honest hit-ratio accounting, add HTTP conditional revalidation so freshness does not mean refetching, write a steering layer that picks a node from live latency and load, attach a cost model so the cheapest node is not always the fastest one, defend the origin against a cache stampede, and finish with a load test where you raise one node's price and watch traffic move. Every decision is judged by numbers the node exports rather than by intuition.

Built step-by-step with [KhwajaLabs Build](https://khwajalabs.com).

## Stack
- Rust
- Tokio
- HTTP
- LRU
- Prometheus
