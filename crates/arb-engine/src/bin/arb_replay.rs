//! Replay harness for arb-engine.
//!
//! Current capabilities:
//! 1. Offline determinism: feed previously captured EdgeDelta prost frames embedded in JSONL.
//! 2. Adjustable pacing: realtime (original nanos), accelerated (divide delays by factor), or burst (no waits).
//! 3. Metrics: reuses ingest latency + edge counters; final summary of emitted signals & aggregate profit.
//! 4. Filtering: by venue id.
//!
//! Supported input (today): JSONL where each line contains either:
//!   {"edge_b64": "<base64 prost EdgeDelta bytes>"}
//! Future extensions (placeholders in comments): length‑delimited protobuf stream, fully decoded JSON EdgeDelta.
//!
//! Example:
//!   cargo run -q -p arb-engine --bin arb_replay -- --input sample.jsonl --realtime
//!   cargo run -q -p arb-engine --bin arb_replay -- --input sample.jsonl --accelerate 5
//!   cargo run -q -p arb-engine --bin arb_replay -- --input sample.jsonl --venue 3 --max 100000

use arb_engine::edge_integration::{decode_and_validate_edge, EdgeArbitrageDetector};
use arb_engine::hotpath::SignalQueue;
use arb_engine::models::ArbSignal;
use arb_engine::stats::{record_ingest_latency_ns, EDGE_PROCESS_ERRORS, EDGE_TOTAL};
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing_subscriber::EnvFilter;

type LineChannel = (Sender<(u64, String)>, Receiver<(u64, String)>);
type EdgeChannel = (
    Sender<(u64, Option<edge_assembler::edge::EdgeDelta>)>,
    Receiver<(u64, Option<edge_assembler::edge::EdgeDelta>)>,
);

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ReplayLine {
    // { "edge_b64": "..." }
    B64 { edge_b64: String },
    // Future: a fully decoded EdgeDelta JSON object. We only try to locate an embedded base64 field currently.
    Raw { edge: serde_json::Value },
}

#[derive(Debug, Clone)]
struct ReplayConfig {
    input: String,
    pace: PaceMode,
    max: Option<usize>,
    filter_venue: Option<u32>,
    start_ns: Option<u64>,
    end_ns: Option<u64>,
    binary: bool, // length-delimited protobuf frames
    signals_out: Option<String>,
    metrics_out: Option<String>,
    workers: usize,
}

#[derive(Debug, Clone)]
enum PaceMode {
    Realtime,         // respect original inter-arrival deltas (requires updated_at_ns ordering)
    Accelerated(f64), // scale down delays by factor
    Burst,            // no sleeps
}

impl PaceMode {
    fn sleep_for(&self, prev_ns: u64, next_ns: u64) {
        if next_ns <= prev_ns {
            return;
        }
        match self {
            PaceMode::Burst => {}
            PaceMode::Realtime => {
                let delta_ns = next_ns - prev_ns;
                std::thread::sleep(Duration::from_nanos(delta_ns));
            }
            PaceMode::Accelerated(f) => {
                if *f <= 0.0 {
                    return;
                }
                let delta_ns = next_ns - prev_ns;
                let scaled = (delta_ns as f64 / *f) as u64;
                if scaled > 0 {
                    std::thread::sleep(Duration::from_nanos(scaled));
                }
            }
        }
    }
}

fn print_help_and_exit() -> ! {
    eprintln!("Usage: arb_replay --input <file> [--realtime | --accelerate <factor> | --burst] [--venue <id>] [--max <n>] [--binary] [--start-ns <n>] [--end-ns <n>] [--signals-out <file>] [--metrics-out <file>] [--workers <n>]\n\n\
Flags:\n  --input <file>        Input file (JSONL default OR length-delimited protobuf if --binary)\n  --binary              Treat input as length-delimited protobuf EdgeDelta frames (varint len + bytes)\n  --realtime            Sleep exact inter-arrival deltas (default pacing is burst)\n  --accelerate <f>      Like realtime but divide delays by <f> (e.g. 5 = 5x faster)\n  --burst               No sleeping (default)\n  --venue <id>          Only process edges matching venue id\n  --max <n>             Stop after processing n edges (after filtering)\n  --start-ns <n>        Only process edges with updated_at_ns >= n\n  --end-ns <n>          Only process edges with updated_at_ns <= n\n  --signals-out <file>  Write emitted ArbSignal JSONL to file after run\n  --metrics-out <file>  Write final Prometheus metrics snapshot\n  --workers <n>         Parallel decode workers (JSONL mode; default cores/2)\n  --help                Show this help\n");
    std::process::exit(1);
}

fn parse_args() -> ReplayConfig {
    let mut input = None;
    let mut pace = PaceMode::Burst;
    let mut max = None;
    let mut filter_venue = None;
    let mut start_ns = None;
    let mut end_ns = None;
    let mut binary = false;
    let mut signals_out = None;
    let mut metrics_out = None;
    let mut workers: Option<usize> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--input" => {
                input = args.next();
            }
            "--realtime" => {
                pace = PaceMode::Realtime;
            }
            "--accelerate" => {
                if let Some(f) = args.next().and_then(|v| v.parse().ok()) {
                    pace = PaceMode::Accelerated(f);
                }
            }
            "--burst" => {
                pace = PaceMode::Burst;
            }
            "--max" => {
                max = args.next().and_then(|v| v.parse().ok());
            }
            "--venue" => {
                filter_venue = args.next().and_then(|v| v.parse().ok());
            }
            "--start-ns" => {
                start_ns = args.next().and_then(|v| v.parse().ok());
            }
            "--end-ns" => {
                end_ns = args.next().and_then(|v| v.parse().ok());
            }
            "--binary" => {
                binary = true;
            }
            "--signals-out" => {
                signals_out = args.next();
            }
            "--metrics-out" => {
                metrics_out = args.next();
            }
            "--workers" => {
                workers = args.next().and_then(|v| v.parse().ok());
            }
            "--help" | "-h" => print_help_and_exit(),
            _ => eprintln!("Unknown arg: {a}"),
        }
    }
    let input = input.unwrap_or_else(|| {
        print_help_and_exit();
    });
    let workers_final = workers.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| (n.get() / 2).max(1))
            .unwrap_or(1)
    });
    ReplayConfig {
        input,
        pace,
        max,
        filter_venue,
        start_ns,
        end_ns,
        binary,
        signals_out,
        metrics_out,
        workers: workers_final,
    }
}

// Read next varint (LEB128) length; return None on EOF.
fn read_varint_len<R: Read>(r: &mut R) -> anyhow::Result<Option<usize>> {
    let mut result: usize = 0;
    let mut shift = 0;
    loop {
        let mut byte = [0u8; 1];
        let bytes_read = r.read(&mut byte)?;
        if bytes_read == 0 {
            if shift == 0 {
                return Ok(None);
            } else {
                return Err(anyhow::anyhow!("truncated_varint"));
            }
        }
        let b = byte[0];
        result |= ((b & 0x7F) as usize) << shift;
        if (b & 0x80) == 0 {
            return Ok(Some(result));
        }
        shift += 7;
        if shift > 63 {
            return Err(anyhow::anyhow!("varint_too_long"));
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("arb_engine=info".parse()?))
        .init();

    #[cfg(feature = "profiling")]
    let guard = {
        use pprof::ProfilerGuardBuilder;
        let freq = std::env::var("PPROF_FREQ")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(99);
        match ProfilerGuardBuilder::default().frequency(freq).build() {
            Ok(g) => {
                tracing::info!(freq, "pprof_started");
                Some(g)
            }
            Err(e) => {
                tracing::warn!(error=?e, "pprof_start_failed");
                None
            }
        }
    };

    let cfg = parse_args();
    tracing::info!(?cfg, "replay_start");

    let q = Arc::new(SignalQueue::new(1024));
    let tx = q.sender();
    let rx = q.receiver();
    let engine = Arc::new(EdgeArbitrageDetector::new(tx));

    let mut processed = 0usize;
    let mut prev_ts: Option<u64> = None;
    let start_wall = Instant::now();

    if cfg.binary {
        let mut file = File::open(&cfg.input)?;
        loop {
            if let Some(limit) = cfg.max {
                if processed >= limit {
                    break;
                }
            }
            let len_opt = read_varint_len(&mut file)?;
            let Some(len) = len_opt else { break }; // EOF
            let mut buf = vec![0u8; len];
            file.read_exact(&mut buf)?;
            let edge = match decode_and_validate_edge(&buf) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if let Some(v) = cfg.filter_venue {
                if edge.venue_id != v {
                    continue;
                }
            }
            if let Some(s) = cfg.start_ns {
                if edge.updated_at_ns < s {
                    continue;
                }
            }
            if let Some(e_ns) = cfg.end_ns {
                if edge.updated_at_ns > e_ns {
                    continue;
                }
            }
            let this_ts = edge.updated_at_ns;
            if let Some(prev) = prev_ts {
                cfg.pace.sleep_for(prev, this_ts);
            }
            let ingest_start = Instant::now();
            match engine.process_edge_update(edge) {
                Ok(_) => {
                    EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Err(_) => {
                    EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    EDGE_PROCESS_ERRORS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            record_ingest_latency_ns(ingest_start.elapsed().as_nanos() as u64);
            processed += 1;
            prev_ts = Some(this_ts);
            if processed.is_multiple_of(10_000) {
                tracing::info!(processed, "replay_progress");
            }
        }
    } else {
        // Parallel JSONL decoding preserving order
        use std::collections::BTreeMap;
        let file = File::open(&cfg.input)?;
        let reader = BufReader::new(file);
        let (line_tx, line_rx): LineChannel = unbounded();
        let (edge_tx, edge_rx): EdgeChannel = unbounded();
        for _ in 0..cfg.workers {
            let rx = line_rx.clone();
            let txe = edge_tx.clone();
            thread::spawn(move || {
                while let Ok((id, line)) = rx.recv() {
                    if line.trim().is_empty() {
                        let _ = txe.send((id, None));
                        continue;
                    }
                    let parsed: Result<ReplayLine, _> = serde_json::from_str(&line);
                    if parsed.is_err() {
                        let _ = txe.send((id, None));
                        continue;
                    }
                    let parsed = parsed.unwrap();
                    let maybe_edge_bytes = match parsed {
                        ReplayLine::B64 { edge_b64 } => {
                            use base64::engine::general_purpose::STANDARD as B64;
                            use base64::Engine;
                            B64.decode(edge_b64).ok()
                        }
                        ReplayLine::Raw { edge } => {
                            edge.get("edge_b64").and_then(|v| v.as_str()).and_then(|s| {
                                use base64::engine::general_purpose::STANDARD as B64;
                                use base64::Engine;
                                B64.decode(s).ok()
                            })
                        }
                    };
                    if let Some(bytes) = maybe_edge_bytes {
                        let edge = match decode_and_validate_edge(&bytes) {
                            Ok(e) => Some(e),
                            Err(_) => {
                                if bytes.len() > 4 {
                                    decode_and_validate_edge(&bytes[4..]).ok()
                                } else {
                                    None
                                }
                            }
                        };
                        let _ = txe.send((id, edge));
                    } else {
                        let _ = txe.send((id, None));
                    }
                }
            });
        }
        drop(edge_tx.clone()); // keep original for drop semantics
        thread::spawn(move || {
            for (i, line) in reader.lines().enumerate() {
                if let Ok(l) = line {
                    if line_tx.send((i as u64, l)).is_err() {
                        break;
                    }
                }
            }
        });
        let mut buffer: BTreeMap<u64, Option<edge_assembler::edge::EdgeDelta>> = BTreeMap::new();
        let mut next_id: u64 = 0;
        'outer: loop {
            if let Some(limit) = cfg.max {
                if processed >= limit {
                    break;
                }
            }
            match edge_rx.recv() {
                Ok((id, opt)) => {
                    buffer.insert(id, opt);
                    while let Some(opt_edge) = buffer.remove(&next_id) {
                        next_id += 1;
                        if let Some(edge) = opt_edge {
                            if let Some(v) = cfg.filter_venue {
                                if edge.venue_id != v {
                                    continue;
                                }
                            }
                            if let Some(s) = cfg.start_ns {
                                if edge.updated_at_ns < s {
                                    continue;
                                }
                            }
                            if let Some(e_ns) = cfg.end_ns {
                                if edge.updated_at_ns > e_ns {
                                    continue;
                                }
                            }
                            let this_ts = edge.updated_at_ns;
                            if let Some(prev) = prev_ts {
                                cfg.pace.sleep_for(prev, this_ts);
                            }
                            let ingest_start = Instant::now();
                            if engine.process_edge_update(edge).is_err() {
                                EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                EDGE_PROCESS_ERRORS
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            } else {
                                EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            record_ingest_latency_ns(ingest_start.elapsed().as_nanos() as u64);
                            processed += 1;
                            prev_ts = Some(this_ts);
                            if processed.is_multiple_of(10_000) {
                                tracing::info!(processed, "replay_progress");
                            }
                            if let Some(limit) = cfg.max {
                                if processed >= limit {
                                    break 'outer;
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }

    let elapsed = start_wall.elapsed();
    tracing::info!(processed, secs = elapsed.as_secs_f64(), "replay_complete");

    // Drain any signals produced (optional summary)
    let mut drained: Vec<ArbSignal> = Vec::new();
    while let Some(sig) = rx.try_recv() {
        drained.push(sig);
    }
    drained.sort_by_key(|s| s.emitted_at_ns);
    let total_profit_bps: i64 = drained.iter().map(|s| s.profit_bps as i64).sum();
    tracing::info!(signals = drained.len(), total_profit_bps, "replay_summary");

    if let Some(path) = cfg.signals_out {
        if let Ok(file) = File::create(&path) {
            let mut w = BufWriter::new(file);
            for sig in &drained {
                if let Ok(line) = serde_json::to_string(sig) {
                    let _ = writeln!(w, "{}", line);
                }
            }
            let _ = w.flush();
            tracing::info!(file=%path, count=drained.len(), "signals_written");
        } else {
            tracing::warn!(file=%path, "signals_write_failed");
        }
    }

    if let Some(path) = cfg.metrics_out {
        if let Err(e) = std::fs::write(&path, arb_engine::stats::render_all()) {
            tracing::warn!(file=%path, error=?e, "metrics_write_failed");
        } else {
            tracing::info!(file=%path, "metrics_written");
        }
    }

    #[cfg(feature = "profiling")]
    if let Some(g) = guard {
        if let Ok(report) = g.report().build() {
            if let Ok(out) = std::env::var("PPROF_FLAME") {
                if let Ok(f) = std::fs::File::create(&out) {
                    let _ = report.flamegraph(f);
                    tracing::info!(file=%out, "pprof_flamegraph_written");
                }
            }
        }
    }

    Ok(())
}
