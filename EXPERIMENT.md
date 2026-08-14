# The experiment that proves the design

The point of this project is one demonstration: change a node's price and
watch traffic move, with the latency cost of that move measured.

## Setup

Run three nodes with different characteristics:

| node | region | latency | cost/GB | cache size |
|------|--------|--------:|--------:|-----------:|
| a | us-east | 20ms | 8c | 256MB |
| b | us-east | 25ms | 3c | 256MB |
| c | eu-west | 90ms | 2c | 256MB |

Latency budget: 60ms. Node c is cheapest and fails the budget, so it should
receive no traffic at the start.

## Run 1 — establish the baseline

Drive 2,000 rps of zipf-skewed traffic for five minutes. Record per node:
hit ratio, p95 latency, origin bytes, share of requests.

Expected: node b takes most traffic. It meets the 60ms budget and costs less
than a. Node c is excluded by the budget despite being cheapest.

## Run 2 — raise a price

Raise node b's cost to 12c/GB mid-run. Do not restart anything.

Expected: traffic shifts to a within one health-report interval. Node c stays
excluded because the budget did not change. Watch `node_cost_per_gb_cents`
and the request share move together.

## Run 3 — relax the budget

Set the latency budget to 120ms and rerun.

Expected: node c becomes eligible and, being cheapest, takes the majority.
Measured p95 rises substantially. This is the trade stated numerically: how
much latency did you buy for how much saving?

## What to write down

For each run: the request share per node, p95, total origin bytes, and
estimated cost per million requests. The deliverable is a short paragraph
answering: at what latency budget does the cheapest node become the right
choice, and what did it cost the user?

## Failure modes to watch for

- Traffic oscillating between two nodes: EWMA alpha is too high.
- Hit ratio far below expectations: keys are too uniform (raise the skew) or
  the cache is too small for the working set.
- Origin bytes spiking at every TTL boundary: coalescing is not engaging.
