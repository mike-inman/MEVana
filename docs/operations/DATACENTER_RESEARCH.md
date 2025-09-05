# Datacenter / Hosting Research Plan

Objective: Minimize end-to-end arbitrage cycle latency (signal -> bundle landed) while maintaining reliability and cost efficiency.

## Latency Components
1. Hotpath signal ingress (WS) -> local processing
2. RPC calls (account fetch, simulation if remote) round-trip
3. Bundle submission (Jito / block engine) network RTT
4. Slot leader schedule variance / block propagation
5. Oracle update latency (price freshness)

## Candidate Regions / Providers
- Equinix Metal (bare metal) in NY/NJ (proximity to major Solana validators)
- AWS us-east-1 / us-east-2 (commodity, higher jitter)
- GCP us-east1 (evaluate network paths)
- OVH / Vultr / Hetzner (cost trade-offs, measure actual RTT first)
- Specialized Solana infra providers offering colocated RPC endpoints

## Measurement Plan
Automate a probe suite (Rust binary or script):
- Ping & TCP connect latency to: selected high-quality public RPCs, private RPC endpoint(s), Jito relays, key validator IPs (where public)
- Simulated RPC load test: getRecentBlockhash, getAccountInfo (batch) -> measure p50/p95
- Jito bundle submission dry-run (no-op) response latency
- WebSocket subscription latency: subscribe to slot notifications & timestamp callback

Collect for 24h:
- Packet loss
- Jitter (stddev) vs median
- Time-of-day patterns

## Selection Criteria
- Median relay RTT < 20ms
- p95 RPC response < 120ms for critical methods
- Stable jitter (< 5ms stddev) for WS streams
- No > 0.5% packet loss sustained

## Architecture Options
1. Single primary region + CDN/edge for telemetry only
2. Active/Passive pair (primary low-latency region + warm standby) with heartbeat & state snapshot replication (token registry, recent edges)
3. Active/Active (complex): both ingest signals, only one elected to submit bundles (leader election via Redis or etcd lock)

## Recommended Initial Approach
- Start with primary region (low-latency NY/NJ bare metal) + passive cloud fallback (AWS us-east-1)
- Implement health probes: circuit breaker state, recent successful bundle count, WS lag
- On primary failure (N minutes no landed bundles + circuit open), fail over by flipping feature flag or DNS for submission service

## Tooling
- Add `latency-probe` crate/binary
- Store results in time-series (Prometheus + remote write) for comparative dashboard
- Automate weekly summary: region deltas, anomalies

## Future Enhancements
- Dynamic route selection: choose best RPC per slot based on rolling latency
- Multi-relay parallel bundle submission with quorum tracking
- Predictive scheduling (prefetch blockhash near leader slots)

## Action Items
- [ ] Enumerate candidate RPC + relay endpoints
- [ ] Implement probe binary
- [ ] Provision short-term trial instances (small shape) in 3 providers
- [ ] Run 48h measurement + collect metrics
- [ ] Produce comparison dashboard & decide primary
