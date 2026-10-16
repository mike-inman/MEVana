# Vultr and Hetzner latency comparison

**Date:** 2026-10-16
**Purpose:** Record the read-only Geyser and network benchmark decision.
**Status:** Historical-endpoint network preflight complete; authenticated trials pending.

| Condition | Vultr | Hetzner |
| --- | --- | --- |
| Region / instance / OS / CPU | Region and instance type pending; Ubuntu 24.04.5, 8 vCPU, AMD EPYC-Milan | Region and instance type pending; Ubuntu 24.04.4, 8 vCPU, AMD EPYC-Rome |
| Monthly cost | Pending | Pending |
| Clock synchronized | Yes (`timedatectl`) | Yes (`timedatectl`) |
| Paid Geyser endpoint region | Pending | Pending |
| Program filter and processed commitment | Pending | Pending |
| Trial UTC starts and stream overlap | Pending | Pending |
| Geyser connect and first update | Pending | Pending |
| Updates/sec, stalls, disconnects (3 × 120 s) | Pending | Pending |
| Historical `grpc.ny.shyft.to` DNS p50/p95/p99, 30/30 successes | 4.655 / 39.684 / 59.397 ms | 1.021 / 1.534 / 1.655 ms |
| Historical endpoint TCP p50/p95/p99, 30/30 successes | 61.192 / 68.850 / 72.888 ms | 61.015 / 61.694 / 62.539 ms |
| Historical endpoint TLS p50/p95/p99, 30/30 successes | 89.861 / 105.121 / 108.777 ms | 86.524 / 103.606 / 105.173 ms |
| Authenticated RPC p50/p95/p99 and failures | Pending | Pending |
| Local raw CSV references (no secrets or IPs) | `/tmp/mevana-linux-benchmark-20261016/results/vultr-historical-network.csv` | `/tmp/mevana-linux-benchmark-20261016/results/hetzner-historical-network.csv` |

The diagnostic runs began at **08:14:11 UTC** (Vultr) and **08:14:20 UTC** (Hetzner) on 2026-10-16. Each used two warm-up attempts followed by 30 independent DNS/TCP/TLS connections with a 5-second timeout. Both hosts resolved two addresses during the run; the address files remain local and uncommitted. The Vultr machine had a load average near 2.1 during the preflight, while Hetzner was near zero, so runtime conditions were not matched. The paid Geyser endpoint has not been verified and no subscription or authenticated RPC request was made.

**Recommendation:** Inconclusive. The historical endpoint's median TCP connect time was about 61 ms from both hosts; these data cannot select a Geyser production host.
**Backup choice:** Pending.
**Limits:** Connection and HTTP response timings are proxies. Time to first transaction is not delivery lag, and this benchmark does not measure transaction landing.

## 2026-10-16 SHYFT RPC follow-up

The authenticated, read-only `getSlot` benchmark completed with 30/30 successful mainnet and 30/30 successful devnet responses from each host. The local, uncommitted report is `benchmark_results/shyft/comparison.md`. Vultr had lower p50/p95/p99 total response times on both networks, so it is the provisional choice for the SHYFT RPC path. The recommendation above remains inconclusive for **Geyser delivery** until a live Yellowstone stream is measured; RPC timing cannot establish Geyser event-arrival or transaction-landing latency.
