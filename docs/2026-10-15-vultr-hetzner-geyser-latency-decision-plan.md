# MEVana: Vultr vs Hetzner latency decision

**Date:** 2026-10-15

**Status:** Ready for implementation
**Objective:** Choose the better server for MEVana's Solana Geyser ingestion and transaction-submission hot path, without building a benchmarking platform.

## Decision in one sentence
Run the same small, read-only benchmark on the existing Vultr and Hetzner servers, collect repeatable results, choose the faster *reliable* path for production, and stop.

## Existing assets
- `scripts/testing/latency_test.sh`: original 2025 network test. Keep as historical reference; **do not use its letter grade** to select a server. It has obsolete endpoints, fragile parsing and ICMP-dependent assumptions.
- `docs/testing/LATENCY_TESTING_GUIDE.md`: historical rationale and targets, not authoritative 2026 benchmarks.
- `crates/executor-service/src/bin/geyser_probe.rs`: authenticated Yellowstone gRPC subscription probe, useful as a starting point. It currently checks connectivity and logs transactions but **does not measure authoritative event-delivery latency**.
- Primary data-provider configuration historically references Shyft's NY Geyser endpoint. Verify the actual paid endpoint and credential before testing.

## Scope and time box
**Target:** roughly 60–90 minutes of hands-on setup/testing once SSH and the Geyser subscription are ready. Avoid investigations and upgrades that make this a multi-day exercise.

**In scope**
1. Identify both servers' geographic region, CPU/OS, hostnames (use anonymized labels in reports), and the actual Geyser endpoint and transaction submission target used by MEVana.
2. Use one lightweight `scripts/testing/compare_host_latency.sh` script (Python 3 standard library and/or system curl) for DNS/TCP/TLS connection and HTTPS request timing. Measure p50/p95/p99, failures and outliers. Use the same targets, options and sample size on both hosts.
3. Extend or wrap `geyser_probe.rs` minimally to record connection time, time to first update, updates/sec, disconnects/reconnects, and stream stalls for identical subscriptions. Run with read-only permissions and **no trading or wallet operations**.
4. Optional *only if supported by trustworthy timestamp semantics*: compare event-delivery lag to a common source clock or correlate the **same transaction signatures** observed on both hosts. Host clocks must be synchronized; do not claim one-way latency from slot number or a local receive timestamp alone.
5. Run short comparable tests on both hosts: a warm-up, then **3 × 2-minute samples** using the same commitment (`processed`), account/program filters and endpoint. Prefer alternating test order or simultaneous observation of the same signatures; record UTC start times.
6. Produce a single Markdown comparison `docs/testing/VULTR_HETZNER_RESULTS.md` with raw result file references (secrets removed), assumptions and a deployment recommendation.

**Out of scope:** trading simulations, deploying the full MEVana stack, refactoring hot path, tuning kernel/network stack, 24-hour load tests, buying new servers, dashboards or Prometheus.

## Exact execution checklist
- [ ] **0. Preconditions.** SSH into both machines. Record region and server type. Confirm UTC/NTP clock sync. Confirm reachability and valid authentication for the *same* Geyser provider endpoint. Keep secrets only in local environment files, never commands committed to git or logs.
- [ ] **1. Minimal implementation.** Reuse existing scripts/probe rather than rearchitecting. Output identical CSV/JSON fields with `server_label,utc_time,endpoint,trial,metric,value,unit,status`; redact IPs/tokens before commit.
- [ ] **2. Network measurements.** Collect at least 30 iterations/host for DNS, TCP connect, TLS handshake and a representative authenticated HTTP RPC request (where available). A 401/403 or obsolete URL is a test failure, **not a speed measurement**. ICMP ping is optional diagnostic data, not a deciding factor.
- [ ] **3. Geyser live stream.** Same gRPC endpoint, credentials/scopes, filters, commitment and runtime on both machines. Record connect + first-event time, update rate and errors. Avoid subscribing twice if the provider limits concurrent streams; if so, run paired sequential trials and mark environmental variability.
- [ ] **4. Execution path.** If a real Jito or RPC submission endpoint is configured, measure safe connection/request latency **without sending a transaction or bundle**. Do not confuse TLS connectivity with transaction landing speed.
- [ ] **5. Repeat and compare.** Three two-minute stream runs per host; short network tests at more than one time if practical. Compare median/p95/p99, tail spikes, errors and cost. Results must label HTTP timing vs connection timing vs real event lag.
- [ ] **6. Decide and stop.** Pick production host if one has stable stream delivery, no material failure difference and a repeatable latency advantage; otherwise prefer the cheaper/operationally simpler host and designate the other as backup. If results are inconclusive, state that explicitly and run just one extra paired trial, not an open-ended campaign.

## Decision rubric
1. **Hard gate:** working authenticated live Geyser stream with acceptable stability; failure, throttling or major stalls disqualifies a host pending a quick retest.
2. **Primary:** earliest arrival of matched signatures, *if* measured with sound clock synchronization; otherwise use observable connection stability and tail response latency as **proxies**, not proof of Geyser delivery speed.
3. **Secondary:** p95/p99 RPC/Jito-path latency, jitter, error rate and variation across samples.
4. **Tie-breakers:** ongoing cost, maintainability, network path and backup/failover simplicity.

**Important:** Solana public RPC ping is not equivalent to Yellowstone delivery latency. A provider's geographically distributed hostname may resolve differently from each host. Save resolved addresses and the actual target region. One-way delivery lag needs a shared trustworthy timestamp/reference, not a locally stamped arrival alone.

## Deliverables and acceptance criteria
- `scripts/testing/compare_host_latency.sh` (or similarly small equivalent), minimal documented updates to `geyser_probe.rs` *only as needed*.
- Repeatable one-command invocation documented for Vultr and Hetzner; no special deployment infrastructure.
- Result artifact with p50/p95/p99, sample counts, failures, methods, endpoint region, server locations, runtime conditions, and provider monthly cost.
- Clear **Vultr / Hetzner / inconclusive** recommendation and a backup choice.
- No secrets, wallet transactions, trades, or live application modifications.

## Handoff for Codex
Implement the smallest safe benchmark satisfying this document in a dedicated branch/worktree. Before edits, inspect the two existing probes and relevant Cargo configuration. Do not modify production services or secrets. Prefer native tooling and deterministic output over new dependencies. Validate shell syntax, Rust compilation for any changed binary, a no-secret dry-run and explain any inability to run a real Geyser stream without credentials. Commit only scripts/docs to the branch and report the exact run commands and changed files for review. **Do not deploy or change server configuration.**
