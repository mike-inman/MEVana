# Arb Replay Harness
Last updated: 2025-09-15

Offline driver for the arbitrage engine using previously captured `EdgeDelta` prost frames.

## Why
- Deterministic backtests / profiling.
- Load / latency testing (burst or accelerated pacing).
- Strategy validation on historical slices before enabling live trading.

## Input Format (current)
JSON Lines where each line has a base64 field with raw prost bytes:
```
{"edge_b64": "AAAA..."}
{"edge_b64": "AAAA..."}
```

Capture helpers can dump the raw prost bytes and then base64 encode per line.

## Usage
```
cargo run -q -p arb-engine --bin arb_replay -- --input data/edges.jsonl --realtime
cargo run -q -p arb-engine --bin arb_replay -- --input data/edges.jsonl --accelerate 5
cargo run -q -p arb-engine --bin arb_replay -- --input data/edges.jsonl --burst --max 200000
cargo run -q -p arb-engine --bin arb_replay -- --input data/edges.jsonl --venue 3
```

Flags:
- `--input <file>`: required path to JSONL.
- `--realtime`: sleep exact inter-arrival deltas (`updated_at_ns`).
- `--accelerate <factor>`: faster than realtime (divide delays by factor).
- `--burst`: no sleeping (default).
- `--venue <id>`: only process that venue.
- `--max <n>`: stop after n edges.
- `--help`: display help.

## Output
Logs progress every 10k edges and prints a completion + summary line with:
- total processed edges
- wall clock elapsed seconds
- number of emitted signals drained
- aggregate profit (bps)

Metrics are recorded through the existing metrics pipeline (ingest latency, totals, errors); run alongside your Prometheus exporter to observe distributions.

## Roadmap
- Length-delimited protobuf input (zero JSON overhead).
- Optional write of emitted signals to JSON/CSV.
- Time window filtering (start/end ns).
- Token / pool filtering.

## Gotchas
- Ensure lines are ordered by `updated_at_ns` for correct pacing in realtime/accelerated modes.
- Burst mode can generate very high CPU usage; throttle via `--accelerate` if you need approximate pacing without full wall-clock delay.
