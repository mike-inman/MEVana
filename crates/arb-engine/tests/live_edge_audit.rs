//! Live edge audit test.
//! Run manually with: LIVE_KAFKA=1 KAFKA_BROKER=host:port EDGE_DELTA_TOPIC=edge.delta.v1 \
//!    cargo test -p arb-engine --test live_edge_audit -- --nocapture --ignored
//! If LIVE_KAFKA != 1 this test is skipped (prints SKIP message and passes).
//! It consumes up to MAX_MESSAGES (default 200) or TIMEOUT_SECS (default 10) seconds worth
//! of EdgeDelta messages and produces a JSON summary per venue of what fields are present.
//! It does NOT currently fail on missing advanced depth fields; it reports them so we can
//! track implementation progress. It only fails if decoding catastrophically fails for all messages.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use arb_engine::edge_integration::decode_and_validate_edge;
use edge_assembler::edge::edge_delta;
use rdkafka::consumer::Consumer; // bring trait into scope for subscribe/commit
use rdkafka::Message; // payload()
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore] // opt-in only
async fn live_edge_audit() {
    if std::env::var("LIVE_KAFKA").ok().as_deref() != Some("1") {
        eprintln!("SKIP_LIVE_TEST set LIVE_KAFKA=1 to run");
        return;
    }

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let topic = std::env::var("EDGE_DELTA_TOPIC").unwrap_or_else(|_| "edge.delta.v1".into());
    let max_messages: usize = std::env::var("MAX_MESSAGES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let timeout_secs: u64 = std::env::var("TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    let consumer: rdkafka::consumer::StreamConsumer = match rdkafka::config::ClientConfig::new()
        .set("group.id", "arb-engine.live-audit")
        .set("bootstrap.servers", &broker)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()
    {
        Ok(c) => c,
        Err(e) => {
            panic!("Unable to create consumer: {e}");
        }
    };
    if let Err(e) = consumer.subscribe(&[&topic]) {
        panic!("Subscribe failed: {e}");
    }

    #[derive(Default)]
    struct VenueReport {
        total: u64,
        decode_fail: u64,
        ok_price_len: u64,
        ok_size_len: u64,
        ok_tokens_len: u64,
        state_counts: HashMap<&'static str, u64>,
        latency_ns_sum: u128,
        latency_ns_max: u64,
        first_ts_ns: Option<u64>,
        missing_fields: Vec<String>,
    }

    let mut venues: HashMap<u32, VenueReport> = HashMap::new();
    let start = Instant::now();
    let deadline = start + Duration::from_secs(timeout_secs);

    while start.elapsed() < Duration::from_secs(timeout_secs)
        && venues.values().map(|v| v.total).sum::<u64>() < max_messages as u64
    {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(50)) => {},
            msg = consumer.recv() => {
                match msg {
                    Ok(m) => {
                        if let Some(payload) = m.payload() {
                            let ed_res = decode_and_validate_edge(payload);
                            let rep = venues.entry(ed_res.as_ref().ok().map(|e| e.venue_id).unwrap_or(0)).or_default();
                            rep.total += 1;
                            match ed_res {
                                Ok(ed) => {
                                    // field checks
                                    if ed.marginal_price_q64.len()==16 { rep.ok_price_len += 1; } else { rep.missing_fields.push(format!("price_len={} venue_edge_id={}", ed.marginal_price_q64.len(), ed.edge_id)); }
                                    if ed.size_to_next_boundary_in.len()==16 { rep.ok_size_len += 1; } else { rep.missing_fields.push(format!("size_len={} venue_edge_id={}", ed.size_to_next_boundary_in.len(), ed.edge_id)); }
                                    if ed.token_in.len()==32 && ed.token_out.len()==32 { rep.ok_tokens_len += 1; } else { rep.missing_fields.push(format!("token_lens={}:{} id={}", ed.token_in.len(), ed.token_out.len(), ed.edge_id)); }
                                    let state_label = match &ed.state { Some(edge_delta::State::Clmm(_)) => "clmm", Some(edge_delta::State::Cpmm(_)) => "cpmm", Some(edge_delta::State::Dlmm(_)) => "dlmm", Some(edge_delta::State::Clob(_)) => "clob", None => "missing" };
                                    *rep.state_counts.entry(state_label).or_insert(0) += 1;
                                    if rep.first_ts_ns.is_none() { rep.first_ts_ns = Some(ed.updated_at_ns); }
                                    // arrival latency (approx real-time vs embedded updated_at)
                                    let now_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
                                    if now_ns > ed.updated_at_ns { let lat_ns = now_ns - ed.updated_at_ns; rep.latency_ns_sum += lat_ns as u128; if lat_ns > rep.latency_ns_max { rep.latency_ns_max = lat_ns; } }
                                }
                                Err(_e) => { rep.decode_fail += 1; }
                            }
                        }
                        // commit async but ignore error
                        let _ = consumer.commit_message(&m, rdkafka::consumer::CommitMode::Async);
                    }
                    Err(_e) => { /* ignore transient */ }
                }
            }
        }
        if Instant::now() >= deadline {
            break;
        }
    }

    // If no data consumed across venues, mark inconclusive (pass but report)
    if venues.is_empty() || venues.values().all(|r| r.total == 0) {
        println!("LIVE_EDGE_AUDIT_RESULT {{\"status\":\"no_messages\",\"broker\":\"{}\",\"topic\":\"{}\"}}", broker, topic);
        return; // pass
    }

    let mut venue_summaries = Vec::new();
    for (vid, rep) in venues.iter() {
        let avg_latency_ms = if rep.total > 0 {
            (rep.latency_ns_sum as f64 / rep.total as f64) / 1_000_000.0
        } else {
            0.0
        };
        // Identify high-level missing categories
        let mut missing_highlevel = Vec::new();
        if rep.ok_price_len < rep.total {
            missing_highlevel.push("price_len");
        }
        if rep.ok_size_len < rep.total {
            missing_highlevel.push("size_len");
        }
        if rep.ok_tokens_len < rep.total {
            missing_highlevel.push("token_len");
        }
        if !rep.state_counts.contains_key("clmm") && *vid == 1 {
            missing_highlevel.push("clmm_state");
        }
        if !rep.state_counts.contains_key("cpmm") && *vid == 2 {
            missing_highlevel.push("cpmm_state");
        }
        if !rep.state_counts.contains_key("dlmm") && *vid == 3 {
            missing_highlevel.push("dlmm_state");
        }
        if !rep.state_counts.contains_key("clob") && *vid == 4 {
            missing_highlevel.push("clob_state");
        }
        venue_summaries.push(json!({
            "venue_id": vid,
            "messages": rep.total,
            "decode_fail": rep.decode_fail,
            "ok_price_len": rep.ok_price_len,
            "ok_size_len": rep.ok_size_len,
            "ok_tokens_len": rep.ok_tokens_len,
            "state_counts": rep.state_counts,
            "avg_latency_ms": avg_latency_ms,
            "max_latency_ms": (rep.latency_ns_max as f64)/1_000_000.0,
            "missing_highlevel": missing_highlevel,
            "examples_missing": rep.missing_fields.iter().take(5).collect::<Vec<_>>()
        }));
    }
    let summary = json!({
        "broker": broker,
        "topic": topic,
        "elapsed_secs": start.elapsed().as_secs_f64(),
        "venues": venue_summaries,
    });
    println!("LIVE_EDGE_AUDIT_RESULT {}", summary);

    // Minimal assert: at least one successfully decoded message overall
    let total_ok: u64 = venues.values().map(|r| r.total - r.decode_fail).sum();
    assert!(total_ok > 0, "No successfully decoded messages");
}
