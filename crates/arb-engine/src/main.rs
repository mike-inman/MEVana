use arb_engine::circuit::circuit_on_message;
use arb_engine::edge_integration::decode_and_validate_edge;
use arb_engine::engine_fast::FastEngine;
use arb_engine::hotpath::SignalQueue;
use arb_engine::jupiter_arbitrage::{JupiterArbitrageScanner, JupiterArbConfig};
use arb_engine::metrics_http::{register_edge_detector, start_http_control};
use arb_engine::stats::{
    record_ingest_latency_ns, EDGE_DECODE_ERRORS, EDGE_PROCESS_ERRORS, EDGE_TOTAL,
};
use edge_assembler::edge::{edge_delta, ClmmState, ClobState, CpmmState, DlmmState, EdgeDelta};
use metrics::counter;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use rand::Rng;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

// WebSocket + metrics server logic moved into metrics_http.rs helper

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("arb_engine=info".parse()?))
        .init();

    // Metrics recorder
    let handle: PrometheusHandle = PrometheusBuilder::new().install_recorder()?;

    // Ring buffer & engine
    let q = Arc::new(SignalQueue::new(1024));
    let tx = q.sender();
    let engine = Arc::new(FastEngine::new(tx.clone()));
    // Register underlying edge detector for depth queries
    if let Some(det) = engine._edge_detector_ref() {
        register_edge_detector(det.clone());
    }

    // Jupiter arbitrage scanner configuration
    let enable_jupiter = std::env::var("ENABLE_JUPITER")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // Enable by default

    let paper_trading = std::env::var("JUPITER_PAPER_TRADING")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // Start in paper trading mode by default

    let jupiter_config = JupiterArbConfig {
        enable_paper_trading: paper_trading,
        min_profit_bps: std::env::var("JUPITER_MIN_PROFIT_BPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50), // Higher threshold for multi-hop
        max_price_impact: std::env::var("JUPITER_MAX_PRICE_IMPACT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3.0),
        scan_interval_ms: std::env::var("JUPITER_SCAN_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000), // Scan every 3 seconds
        ..Default::default()
    };

    tracing::info!(
        "🪐 Jupiter integration: enabled={}, paper_trading={}, min_profit={}bps",
        enable_jupiter,
        paper_trading,
        jupiter_config.min_profit_bps
    );

    // Initialize Jupiter scanner
    let jupiter_scanner = if enable_jupiter {
        let scanner = Arc::new(JupiterArbitrageScanner::new(tx.clone(), Some(jupiter_config)));
        
        // Start Jupiter scanner in background
        let scanner_clone = Arc::clone(&scanner);
        tokio::spawn(async move {
            if let Err(e) = scanner_clone.start().await {
                tracing::error!("Failed to start Jupiter scanner: {}", e);
            }
        });
        
        Some(scanner)
    } else {
        tracing::info!("🪐 Jupiter integration disabled");
        None
    };

    // Edge consumer (Kafka prost edge deltas)
    let broker = std::env::var("KAFKA_BROKER").unwrap_or("localhost:29092".into());
    let edge_topic = std::env::var("EDGE_DELTA_TOPIC").unwrap_or("edge.delta.v1".into());
    let kafka_debug = std::env::var("EDGE_KAFKA_DEBUG")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // Build Kafka consumer manually here to decode and feed engine
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "arb-engine.edge")
        .set("bootstrap.servers", &broker)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    consumer.subscribe(&[&edge_topic])?;

    tracing::info!(broker=%broker, topic=%edge_topic, "kafka_consumer_started");

    // Diagnostics: track message counts & last-seen timestamp so we can report idleness
    static TOTAL_MSG: AtomicU64 = AtomicU64::new(0);
    static LAST_MSG_NS: AtomicU64 = AtomicU64::new(0);

    // Spawn task to read edge deltas
    let engine_clone = engine.clone();
    // Simple ingestion counter incremented per accepted edge (rate derivable externally).
    tokio::spawn(async move {
        use std::time::Instant;
        let mut idle_report_interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = idle_report_interval.tick() => {
                    let total = TOTAL_MSG.load(Ordering::Relaxed);
                    let last_ns = LAST_MSG_NS.load(Ordering::Relaxed);
                    if total == 0 {
                        tracing::warn!("kafka_idle_no_messages_yet interval_secs=5");
                    } else {
                        let now_ns = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
                        let idle_ms = if last_ns > 0 && now_ns > last_ns { (now_ns - last_ns)/1_000_000 } else { 0 };
                        if idle_ms > 10_000 { // >10s since last message
                            tracing::warn!(total_messages=total, idle_ms, "kafka_idle_since_last_message");
                        } else if kafka_debug {
                            tracing::info!(total_messages=total, idle_ms, "kafka_activity_summary");
                        }
                    }
                }
                msg_res = consumer.recv() => {
                    match msg_res {
                        Ok(msg) => {
                    let ingest_start = Instant::now();
                    if let Some(payload) = msg.payload() {
                        counter!("edge_messages_total");
                        TOTAL_MSG.fetch_add(1, Ordering::Relaxed);
                        LAST_MSG_NS.store(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64, Ordering::Relaxed);
                        if kafka_debug {
                            tracing::info!(len=payload.len(), partition=msg.partition(), offset=msg.offset(), "kafka_msg_recv");
                        }
                        let mut msg_error = false;
                        match decode_and_validate_edge(payload) {
                            Ok(delta) => {
                                if let Err(e) = engine_clone.process_edge_update(delta) {
                                    tracing::error!(error=?e, "edge_process_failed");
                                    counter!("edge_rejected_total", "reason" => "process_error");
                                    EDGE_PROCESS_ERRORS
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    msg_error = true;
                                }
                                EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            Err(reason) => {
                                counter!("edge_decode_errors_total", "reason" => reason);
                                EDGE_DECODE_ERRORS
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                msg_error = true;
                            }
                        }
                        // record ingest latency (receive -> processed or rejected)
                        let ns = ingest_start.elapsed().as_nanos() as u64;
                        record_ingest_latency_ns(ns);
                        let now_ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as u64;
                        circuit_on_message(msg_error, now_ns);
                    }
                    let _ = consumer.commit_message(&msg, rdkafka::consumer::CommitMode::Async);
                        }
                        Err(e) => {
                            tracing::warn!(error=?e, "kafka_recv_error");
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                    }
                }
            } // select
        }
    });

    // Start HTTP control plane (/metrics + /ws/arbs)
    let addr: SocketAddr = std::env::var("WS_BIND")
        .unwrap_or("0.0.0.0:8080".into())
        .parse()?;
    let http_q = if std::env::var("DISABLE_WS").ok().as_deref() == Some("1") {
        None
    } else {
        Some(q.clone())
    };
    tokio::spawn(async move {
        if let Err(e) = start_http_control(addr, handle.clone(), http_q).await {
            tracing::error!(error=?e, "http_control_failed");
        }
    });
    tracing::info!(%addr, "http_control_started");

    // Synthetic edge generator (FORBIDDEN in normal runs). Requires EDGE_SYNTH=1 AND ALLOW_SYNTH=1.
    let synth_requested = std::env::var("EDGE_SYNTH")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let synth_allowed = std::env::var("ALLOW_SYNTH").ok().as_deref() == Some("1");
    if synth_requested && !synth_allowed {
        tracing::warn!(
            "synthetic_mode_blocked set ALLOW_SYNTH=1 to override (NOT recommended in prod)"
        );
    }
    if synth_requested && synth_allowed {
        let engine_synth = engine.clone();
        tokio::spawn(async move {
            let mut id: u64 = 1;
            // Config knobs for synthetic data shaping
            let spread = std::env::var("EDGE_SYNTH_SPREAD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| *v > 0.0 && *v < 0.05)
                .unwrap_or(0.0005);
            let cycles_mode = std::env::var("EDGE_SYNTH_CYCLES")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let fake_signals = std::env::var("EDGE_SYNTH_FAKE_SIGNALS")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            // Dynamic venue list reused by both modes
            let venue_ids: Vec<u32> = std::env::var("EDGE_VENUES")
                .ok()
                .map(|s| {
                    s.split([',', ' '])
                        .filter_map(|p| {
                            let t = p.trim();
                            if t.is_empty() {
                                None
                            } else {
                                t.parse::<u32>().ok()
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|v: &Vec<u32>| !v.is_empty())
                .unwrap_or_else(|| {
                    if cycles_mode {
                        vec![1, 2, 3]
                    } else {
                        vec![1, 2, 3, 4]
                    }
                });
            // Optional explicit per-venue biases (comma list matching venue_ids order) relative to spread units
            let venue_biases: Vec<f64> = std::env::var("EDGE_SYNTH_VENUE_BIASES")
                .ok()
                .map(|s| {
                    s.split([',', ' '])
                        .filter_map(|p| {
                            let t = p.trim();
                            if t.is_empty() {
                                None
                            } else {
                                t.parse::<f64>().ok()
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            // Predefine a small token universe for cycle formation when cycles_mode enabled
            let token_bytes: Vec<Vec<u8>> = (0u8..3u8).map(|t| vec![t; 32]).collect(); // T0,T1,T2
            use arb_engine::sizing::record_signal_profit_bps;
            use arb_engine::stats::{incr_signals_emitted, record_profit};
            use rand::Rng;
            loop {
                if cycles_mode {
                    // Deterministic 3-token cycle edges across venues; venue-specific bias for price differentiation
                    // We create directed edges T[i] -> T[(i+1)%3]
                    for (idx, venue) in venue_ids.iter().enumerate() {
                        for i in 0..3usize {
                            let mut ed = EdgeDelta::default();
                            ed.edge_id = id;
                            id += 1;
                            ed.venue_id = *venue;
                            ed.direction = 0;
                            ed.slot = id / 10;
                            ed.updated_at_ns = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos() as u64;
                            ed.fee_bps_total = 10; // 0.10%
                            ed.decimals_in = 6;
                            ed.decimals_out = 6;
                            ed.cu_estimate = 0;
                            let tin = &token_bytes[i];
                            let tout = &token_bytes[(i + 1) % 3];
                            ed.token_in = tin.clone();
                            ed.token_out = tout.clone();
                            // Pool key derived deterministically
                            let mut pk = [*venue as u8; 32];
                            pk[0] = tin[0];
                            pk[1] = tout[0];
                            ed.pool_pubkey = pk.to_vec();
                            // Venue bias ensures cycle product > 1 slightly (customizable)
                            let venue_bias = if !venue_biases.is_empty() && idx < venue_biases.len()
                            {
                                spread * venue_biases[idx]
                            } else {
                                // fallback default pattern (roughly net positive)
                                match idx {
                                    0 => spread * 0.8,
                                    1 => -spread * 0.4,
                                    2 => spread * 0.5,
                                    _ => 0.0,
                                }
                            };
                            let jitter = rand::thread_rng().gen_range(-spread * 0.1..spread * 0.1);
                            let price = 1.0 + venue_bias + jitter;
                            let raw = (price * 18446744073709551616.0) as u128;
                            ed.marginal_price_q64 = raw.to_le_bytes().to_vec();
                            let size: u128 = 5_000_000u128;
                            ed.size_to_next_boundary_in = size.to_le_bytes().to_vec();
                            // Simple CLMM state for venue 1, CPMM for 2, DLMM for 3
                            ed.state = Some(match *venue {
                                1 => edge_delta::State::Clmm(ClmmState {
                                    sqrt_price_x64: raw.to_le_bytes().to_vec(),
                                    liquidity: 1_000_000u128.to_le_bytes().to_vec(),
                                    tick_current: 0,
                                    tick_spacing: 64,
                                    size_to_next_in: size.to_le_bytes().to_vec(),
                                }),
                                2 => edge_delta::State::Cpmm(CpmmState {
                                    reserve_in: 10_000_000u128.to_le_bytes().to_vec(),
                                    reserve_out: 9_900_000u128.to_le_bytes().to_vec(),
                                }),
                                3 => edge_delta::State::Dlmm(DlmmState {
                                    active_bin: 0,
                                    bin_price_q64: raw.to_le_bytes().to_vec(),
                                    depth_hint_in: size.to_le_bytes().to_vec(),
                                }),
                                _ => unreachable!(),
                            });
                            let _ = engine_synth.process_edge_update(ed);
                            // Generate reverse direction edge to enable full cycle reconstruction unless disabled.
                            let bidir = std::env::var("EDGE_SYNTH_BIDIR")
                                .ok()
                                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                                .unwrap_or(true);
                            if bidir {
                                let mut rev = EdgeDelta::default();
                                rev.edge_id = id;
                                id += 1;
                                rev.venue_id = *venue;
                                rev.direction = 1; // opposite direction marker
                                rev.slot = id / 10;
                                rev.updated_at_ns = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos()
                                    as u64;
                                rev.fee_bps_total = 10;
                                rev.decimals_in = 6;
                                rev.decimals_out = 6;
                                rev.cu_estimate = 0;
                                rev.token_in = token_bytes[(i + 1) % 3].clone();
                                rev.token_out = token_bytes[i].clone();
                                // Invert price (avoid division by zero)
                                let price_f = price; // original forward price
                                let inv_price = if price_f > 0.0 { 1.0 / price_f } else { 1.0 };
                                let inv_raw = (inv_price * 18446744073709551616.0) as u128;
                                rev.marginal_price_q64 = inv_raw.to_le_bytes().to_vec();
                                let size: u128 = 5_000_000u128;
                                rev.size_to_next_boundary_in = size.to_le_bytes().to_vec();
                                rev.state = Some(match *venue {
                                    1 => edge_delta::State::Clmm(ClmmState {
                                        sqrt_price_x64: inv_raw.to_le_bytes().to_vec(),
                                        liquidity: 1_000_000u128.to_le_bytes().to_vec(),
                                        tick_current: 0,
                                        tick_spacing: 64,
                                        size_to_next_in: size.to_le_bytes().to_vec(),
                                    }),
                                    2 => edge_delta::State::Cpmm(CpmmState {
                                        reserve_in: 10_000_000u128.to_le_bytes().to_vec(),
                                        reserve_out: 9_900_000u128.to_le_bytes().to_vec(),
                                    }),
                                    3 => edge_delta::State::Dlmm(DlmmState {
                                        active_bin: 0,
                                        bin_price_q64: inv_raw.to_le_bytes().to_vec(),
                                        depth_hint_in: size.to_le_bytes().to_vec(),
                                    }),
                                    _ => unreachable!(),
                                });
                                let mut rpk = [*venue as u8; 32];
                                rpk[0] = rev.token_in[0];
                                rpk[1] = rev.token_out[0];
                                rev.pool_pubkey = rpk.to_vec();
                                let _ = engine_synth.process_edge_update(rev);
                            }
                        }
                    }
                } else {
                    // Original random mode (slight modifications already captured above)
                    for venue in venue_ids.iter() {
                        let mut ed = EdgeDelta::default();
                        ed.edge_id = id;
                        id += 1;
                        ed.venue_id = *venue;
                        ed.direction = (id % 2) as u32;
                        ed.slot = id / 10;
                        ed.write_version = 0;
                        ed.updated_at_ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as u64;
                        ed.fee_bps_total = 20;
                        ed.decimals_in = 6;
                        ed.decimals_out = 6;
                        ed.cu_estimate = 0;
                        let mut buf = [0u8; 32];
                        rand::thread_rng().fill(&mut buf);
                        ed.token_in = buf.to_vec();
                        rand::thread_rng().fill(&mut buf);
                        ed.token_out = buf.to_vec();
                        rand::thread_rng().fill(&mut buf);
                        ed.pool_pubkey = buf.to_vec();
                        let price = 1.0 + rand::thread_rng().gen_range(-spread..spread);
                        let raw = (price * 18446744073709551616.0) as u128;
                        ed.marginal_price_q64 = raw.to_le_bytes().to_vec();
                        let size: u128 = 5_000_000u128;
                        ed.size_to_next_boundary_in = size.to_le_bytes().to_vec();
                        ed.state = Some(match *venue {
                            1 => edge_delta::State::Clmm(ClmmState {
                                sqrt_price_x64: raw.to_le_bytes().to_vec(),
                                liquidity: 1_000_000u128.to_le_bytes().to_vec(),
                                tick_current: 0,
                                tick_spacing: 64,
                                size_to_next_in: size.to_le_bytes().to_vec(),
                            }),
                            2 => edge_delta::State::Cpmm(CpmmState {
                                reserve_in: 10_000_000u128.to_le_bytes().to_vec(),
                                reserve_out: 9_900_000u128.to_le_bytes().to_vec(),
                            }),
                            3 => edge_delta::State::Dlmm(DlmmState {
                                active_bin: 0,
                                bin_price_q64: raw.to_le_bytes().to_vec(),
                                depth_hint_in: size.to_le_bytes().to_vec(),
                            }),
                            4 => edge_delta::State::Clob(ClobState {
                                best_bid_px_q64: raw.to_le_bytes().to_vec(),
                                best_bid_size: (1000u64).to_le_bytes().to_vec(),
                                best_ask_px_q64: raw.to_le_bytes().to_vec(),
                                best_ask_size: (1200u64).to_le_bytes().to_vec(),
                            }),
                            _ => edge_delta::State::Clmm(ClmmState {
                                sqrt_price_x64: raw.to_le_bytes().to_vec(),
                                liquidity: 500_000u128.to_le_bytes().to_vec(),
                                tick_current: 0,
                                tick_spacing: 64,
                                size_to_next_in: size.to_le_bytes().to_vec(),
                            }),
                        });
                        let _ = engine_synth.process_edge_update(ed);
                    }
                }
                if fake_signals {
                    // Emit fabricated signal metrics to drive dashboards (profit_bps sampled around 80-200 bps)
                    let profit_bps = rand::thread_rng().gen_range(80..200) as i32;
                    record_profit(profit_bps);
                    record_signal_profit_bps(profit_bps);
                    incr_signals_emitted();
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                // pacing
            }
        });
        tracing::info!("synthetic_edge_generator_started");
    }

    #[cfg(feature = "diagnostics")]
    {
        use std::time::Duration;
        let engine_metrics = engine.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                engine_metrics.refresh_structural_gauges();
            }
        });
    }
    // Keep process alive until Ctrl+C
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { tracing::info!("shutdown_ctrl_c"); },
            _ = sigint.recv() => { tracing::info!("shutdown_sigint"); },
            _ = sigterm.recv() => { tracing::info!("shutdown_sigterm"); },
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        tracing::info!("shutdown_ctrl_c");
    }
    Ok(())
}
