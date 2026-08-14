# Edge Caching & Traffic Routing — Where the Byte Comes From

A project about the decision at the heart of content delivery: which node should serve this request. Built a cache node with a real eviction policy and honest hit-ratio accounting, add HTTP conditional revalidation so freshness does not mean refetching, wrote a steering layer that picks a node from live latency and load, attached a cost model so the cheapest node is not always the fastest one, defended the origin against a cache stampede, and finished with a load test where one node's price is raised and traffic's move is shown. Every decision is judged by numbers the node exports rather than by intuition.

## Stack
- Rust
- Tokio
- HTTP
- LRU
- Prometheus
