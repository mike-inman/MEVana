# Vultr and Hetzner Geyser latency benchmark runbook

**Date:** 2026-10-15
**Purpose:** Run the same read-only measurements on each existing server and collect a host decision.

## Preconditions

Use the same paid `GEYSER_ENDPOINT`, `GEYSER_X_TOKEN`, `GEYSER_PROGRAM_IDS`, and authenticated `SHYFT_RPC_URL` on both hosts. Set them in each host's existing private environment, without printing or copying credentials into reports. Confirm the endpoint is the active paid endpoint; the old NY hostname in repository examples is not proof. Confirm `timedatectl` reports synchronized clocks, and record each host's region, instance type, OS, CPU, monthly cost and UTC test time. Do not include hostnames or IPs in committed results.

The RPC URL must authorize a `getHealth` request. If no authenticated RPC target is available, omit the RPC variable and mark RPC comparison unavailable. For Jito, the network checker can measure TLS to an HTTPS endpoint by assigning it to `GEYSER_ENDPOINT` for a separate run, but this says nothing about transaction landing time. Do not issue submission calls.

## One host at a time

On each host, use a checkout of the benchmark branch. Build the probe once with `cargo build --release --locked --manifest-path scripts/testing/geyser-probe/Cargo.toml`. The small standalone manifest points at the same read-only probe source and avoids unrelated executor-service dependencies. Use the same source revision and build mode on both hosts. Do not alter the running MEVana stack.

**2026-10-16 deployment note:** The probe was instead built once locally for Linux x86_64 in a Debian Bookworm container from commit `543c3559`, then copied with the Python checker to `/opt/mevana-latency-benchmark-20261016/` on the SSH-accessible Vultr and Hetzner machines. Both are shared with AI-Influencer-Factory. The remote directory is mode 700 and no credentials, services, or MEVana application files were changed. The `geyser_probe` SHA-256 is `f4f6ca88ee98119aa0dc32e593adda4784b7c48799f98144374abbc225e38b2c` on both machines. Run the deployed `./geyser_probe` from that directory; the source checkout commands below remain the reproducible build path.

Set `BENCH_SERVER_LABEL` to `vultr` or `hetzner`, and make a private output directory (`mkdir -p tmp/latency-benchmark && chmod 700 tmp/latency-benchmark`). The repository ignores `tmp/`; raw files stay local. For each of trials `t1`, `t2`, `t3`:

```sh
export BENCH_TRIAL=t1
export BENCH_OUTPUT="tmp/latency-benchmark/${BENCH_SERVER_LABEL}-stream.csv"
export GEYSER_PROBE_SECS=120
python3 scripts/testing/compare_host_latency.py \
  --server-label "$BENCH_SERVER_LABEL" --trial "$BENCH_TRIAL" \
  --samples 30 --warmup 2 \
  --output "tmp/latency-benchmark/${BENCH_SERVER_LABEL}-network.csv" \
  --addresses-output "tmp/latency-benchmark/${BENCH_SERVER_LABEL}-addresses.csv"
scripts/testing/geyser-probe/target/release/geyser_probe
```

Run `t2` and `t3` by changing `BENCH_TRIAL`. Alternate host order if the Geyser plan allows only one concurrent stream. Record the UTC start time of each trial and whether streams overlap. Both probes use a processed transaction subscription with vote and failed transactions excluded and the same program IDs. The script performs independent DNS, TCP and TLS connections for each sample, then an authenticated HTTP RPC `getHealth` request where configured. It fails on a non-200 status or a JSON-RPC error. A failed trial is kept in the raw CSV and must not be counted as a fast success.

## Interpretation

The Python command prints p50/p95/p99 over successful samples. The stream CSV includes connect time, time to first matching transaction, updates per second, stalls and disconnects. No automatic reconnect is attempted: a disconnect ends the trial and fails the hard gate. A stall means a 10-second interval without a matching transaction; check whether the filter is active enough before treating it as provider instability. Time to first update is affected by transaction frequency and is not event delivery lag. TCP/TLS and HTTP times are path proxies. Neither script claims one-way delivery latency, transaction landing time or earliest arrival of matched signatures.

Copy only scrubbed aggregate statistics and operational conditions into `docs/testing/2026-10-16-vultr-hetzner-latency-results.md`. Keep raw CSV and resolved-address files outside Git. Before committing, check `git status --short` and inspect the result document for credentials, hostnames and IPs. Choose a host only after all three stream trials on both machines are stable and the measured advantage is repeatable; otherwise mark the result inconclusive and use the plan's one extra paired trial rule.
