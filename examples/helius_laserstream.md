

Build this as a service 



Secure RPC endpoints
These URLs are specifically masked and IP rate-limited at 5 TPS, making them safe to use directly from frontend applications without exposing your API credits:
Mainnet: https://abc-456-fast-mainnet.helius-rpc.com
Devnet: https://123-xyz-fast-devnet.helius-rpc.com


Solana Websocket Endpoints
Standard WebSocket endpoints that provide real-time data streaming according to the Solana WebSocket protocol. These endpoints support all standard Solana WebSocket subscription methods including account, program, signature, slot, and log subscriptions:
Mainnet: wss://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY
Devnet: wss://devnet.helius-rpc.com/?api-key=YOUR_API_KEY


Geyser Enhanced Websocket Endpoints
Enhanced WebSockets offer faster response times than traditional RPC WebSockets and provide real-time transaction and account updates directly to your applications:
Mainnet: wss://atlas-mainnet.helius-rpc.com/?api-key=YOUR_API_KEY
Devnet: wss://atlas-devnet.helius-rpc.com/?api-key=YOUR_API_KEY


https://dashboard.helius.dev/laserstream?projectId=8f5ecd30-bd52-405f-a21f-aef5208b44ea
https://dashboard.helius.dev/laserstream?projectId=8f5ecd30-bd52-405f-a21f-aef5208b44ea
https://laserstream-devnet-ewr.helius-rpc.com
ab84506c-783e-460f-af33-418122da0f84

https://dashboard.helius.dev/endpoints?projectId=8f5ecd30-bd52-405f-a21f-aef5208b44ea


1) Add metrics + HTTP exporter to your Rust app
Cargo.toml
[package]
name = "laserstream-metrics"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
futures = "0.3"
helius-laserstream = "0.1"
axum = "0.7"
hyper = { version = "1", features = ["http1", "server"] }
prometheus = "0.13"
once_cell = "1.19"
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

src/main.rs
use axum::{routing::get, Router};
use futures::StreamExt;
use helius_laserstream::{subscribe, CommitmentLevel, LaserstreamConfig, SubscribeRequest, TransactionsFilter, TransactionFilter};
use once_cell::sync::Lazy;
use prometheus::{register_counter_vec, register_histogram_vec, CounterVec, Encoder, HistogramVec, TextEncoder};
use serde_json::Value as Json;
use std::{env, net::SocketAddr, time::Duration};

/// --------- Prometheus metrics (label-low, extend later) ----------
static TX_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "mevana_tx_total",
        "Total Laserstream transactions seen",
        &["program", "status"] // program id we detected, status ok|err
    ).unwrap()
});

static PARSE_ERRORS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "mevana_parse_errors_total",
        "Number of parse errors",
        &["stage"] // stage = "decode" | "extract"
    ).unwrap()
});

static E2E_LATENCY_MS: Lazy<HistogramVec> = Lazy::new(|| {
    // buckets in ms: tune later
    register_histogram_vec!(
        "mevana_e2e_latency_ms",
        "End-to-end latency from on-chain blockTime (ms)",
        &["program"],
        vec![1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0]
    ).unwrap()
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Config from env (so it works in Docker too) ---
    let api_key = env::var("HELIUS_API_KEY").unwrap_or_else(|_| "YOUR_API_KEY".to_string());
    let endpoint = env::var("HELIUS_ENDPOINT")
        .unwrap_or_else(|_| "https://laserstream-mainnet-ewr.helius-rpc.com".to_string());
    let metrics_addr = env::var("METRICS_ADDR").unwrap_or_else(|_| "0.0.0.0:9100".to_string());
    // filter for Jupiter v6 by default (change as you like)
    let program_filter = env::var("PROGRAM_ID")
        .unwrap_or_else(|_| "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string());

    // --- Laserstream config & subscription ---
    let config = LaserstreamConfig {
        api_key,
        endpoint,
    };

    let request = SubscribeRequest {
        transactions: Some(TransactionsFilter {
            client: Some(TransactionFilter {
                account_include: vec![program_filter.clone()],
                vote: false,
                failed: false,
                ..Default::default()
            }),
        }),
        commitment: CommitmentLevel::Confirmed,
        ..Default::default()
    };

    let mut stream = subscribe(config, request).await?;

    // --- Start metrics HTTP server ---
    let app = Router::new().route("/metrics", get(|| async move {
        let mut buf = vec![];
        let encoder = TextEncoder::new();
        let mf = prometheus::gather();
        encoder.encode(&mf, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }));

    let addr: SocketAddr = metrics_addr.parse()?;
    tokio::spawn(async move {
        println!("Serving metrics on http://{}/metrics", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // --- Consume stream, parse a little, export metrics ---
    while let Some(res) = stream.next().await {
        match res {
            Ok(data) => {
                // We don't rely on crate structs; we convert to JSON to be future-proof.
                let json = match serde_json::to_value(&data) {
                    Ok(v) => v,
                    Err(_) => {
                        PARSE_ERRORS.with_label_values(&["decode"]).inc();
                        continue;
                    }
                };

                // 1) detect a "program id" for labeling from inner instr if present
                let program = detect_program_id(&json).unwrap_or_else(|| "unknown".to_string());

                // 2) tx status (ok/err) heuristic
                let status = extract_status(&json).unwrap_or("ok".to_string());

                TX_TOTAL.with_label_values(&[&program, &status]).inc();

                // 3) e2e latency from blockTime if present (seconds -> ms)
                if let Some(block_time_ms) = extract_block_time_ms(&json) {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    if now_ms >= block_time_ms {
                        let delta = (now_ms - block_time_ms) as f64;
                        E2E_LATENCY_MS.with_label_values(&[&program]).observe(delta);
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {e:?}");
                // Optional: backoff to avoid tight loop on errors
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }

    Ok(())
}

/// Try a few common places Laserstream/Helius put program ids.
/// We keep this defensive & JSON-based so it doesn’t break on schema drift.
fn detect_program_id(v: &Json) -> Option<String> {
    // common paths (adjust as you inspect your payload):
    //  - tx.meta.logMessages (search for "Program X invoke")
    //  - tx.transaction.message.instructions[].programId
    //  - simplified fields helius sometimes includes
    if let Some(prog) = v.pointer("/transaction/message/instructions/0/programId").and_then(|x| x.as_str()) {
        return Some(prog.to_string());
    }
    if let Some(arr) = v.pointer("/transaction/message/instructions").and_then(|x| x.as_array()) {
        for ins in arr {
            if let Some(p) = ins.get("programId").and_then(|x| x.as_str()) {
                return Some(p.to_string());
            }
        }
    }
    // fallback: some Helius shapes include `programId` higher up
    if let Some(p) = v.get("programId").and_then(|x| x.as_str()) {
        return Some(p.to_string());
    }
    None
}

fn extract_status(v: &Json) -> Option<String> {
    // Look for explicit error flags; else "ok"
    // Helius tx often has meta.err == null when OK.
    if let Some(err) = v.pointer("/meta/err") {
        if !err.is_null() {
            return Some("err".into());
        }
    }
    if let Some(failed) = v.get("failed").and_then(|x| x.as_bool()) {
        return Some(if failed { "err".into() } else { "ok".into() });
    }
    Some("ok".into())
}

fn extract_block_time_ms(v: &Json) -> Option<i64> {
    // Common: transaction.blockTime (seconds)
    if let Some(bt) = v.pointer("/blockTime").and_then(|x| x.as_i64()) {
        return Some(bt * 1000);
    }
    if let Some(bt) = v.pointer("/transaction/blockTime").and_then(|x| x.as_i64()) {
        return Some(bt * 1000);
    }
    None
}

How this “parser” helps

Program ID extraction → you get per-DEX/per-program rates (e.g., Jupiter, Raydium).

Status → split OK vs error/fail.

E2E latency → from on-chain blockTime to “now”, so you can sanity-check freshness.

You can extend detect_program_id later to parse more deeply (e.g., first non-system instruction, or map program IDs to friendly names).





2) Run it locally & wire Prometheus + Grafana
Start your app
export HELIUS_API_KEY=...            # required
export HELIUS_ENDPOINT=https://laserstream-mainnet-ewr.helius-rpc.com
export PROGRAM_ID=JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4  # example
export METRICS_ADDR=0.0.0.0:9100

cargo run
# or dockerize later — this exposes http://localhost:9100/metrics

Prometheus scrape config (add to your prometheus.yml)
scrape_configs:
  - job_name: "laserstream-metrics"
    static_configs:
      - targets: ["localhost:9100"]


Restart Prometheus and visit http://localhost:9090/targets to confirm the job is “UP”.

3) Quick Grafana queries (paste into panels)

Transactions per second (overall):

rate(mevana_tx_total[1m])


Transactions per second by program:

sum by (program) (rate(mevana_tx_total[1m]))


Error rate (per second):

sum(rate(mevana_tx_total{status="err"}[1m]))


E2E latency p50/p90/p99 (ms):

histogram_quantile(0.50, sum by (le) (rate(mevana_e2e_latency_ms_bucket[2m])))
histogram_quantile(0.90, sum by (le) (rate(mevana_e2e_latency_ms_bucket[2m])))
histogram_quantile(0.99, sum by (le) (rate(mevana_e2e_latency_ms_bucket[2m])))


Latency by program (p90):

histogram_quantile(
  0.90,
  sum by (le, program) (rate(mevana_e2e_latency_ms_bucket[2m]))
)


Create a Grafana Time series panel for each, set unit to “ops” or “ms” as appropriate.

4) Dockerize (optional, simple)

Dockerfile

FROM rust:1.79 as build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM gcr.io/distroless/cc
WORKDIR /app
COPY --from=build /app/target/release/laserstream-metrics /app/laserstream-metrics
EXPOSE 9100
ENTRYPOINT ["/app/laserstream-metrics"]


docker run

docker build -t laserstream-metrics:local .
docker run --rm -p 9100:9100 \
  -e HELIUS_API_KEY=... \
  -e HELIUS_ENDPOINT=https://laserstream-mainnet-ewr.helius-rpc.com \
  -e PROGRAM_ID=JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 \
  laserstream-metrics:local

5) What to improve next (when you’re ready)

Richer parsing: extract mint A/B, amount in/out, or swap program variant. Add controlled labels like dex="jupiter" and pair="USDC/SOL" only for a small allow-list to avoid Prometheus cardinality explosions.

Consumer health gauges: last-message age (ms), stream disconnects, reconnects, backoff counters.

SLOs/alerts: Prometheus recording rules for “<2s p99 latency” or “error rate < 1%”.

If you share a sample Laserstream payload (one or two JSONs), I can tighten detect_program_id and add a tiny “pair detector” (safe, low-cardinality) so your Grafana shows “Jupiter USDC/SOL swaps / s” out of the gate.