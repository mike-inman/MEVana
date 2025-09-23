use anyhow::Result;
use futures_util::StreamExt;
use log::{error, info, warn};
use tokio::time::{sleep, Duration, Instant};
use tokio_tungstenite::connect_async;
use url::Url;
use uuid::Uuid;

use crate::execution_engine::{ArbitrageOpportunity, ExecutionEngine};
use crate::metrics::MetricsServer;
use shared_protocol::{ArbSignal, ArbSignalV1};
use std::sync::Arc;

/// Map hotpath ArbSignal into internal ArbitrageOpportunity (minimal fields for now)
fn map_signal(sig: ArbSignalV1) -> ArbitrageOpportunity {
    ArbitrageOpportunity {
        opportunity_id: Uuid::new_v4(),
        token_in: bs58::decode(&sig.pair.a).into_vec().unwrap_or_default(),
        token_out: bs58::decode(&sig.pair.b).into_vec().unwrap_or_default(),
        edges: vec![],
        venues: vec![],
        profit_bps: sig.profit_bps.max(0) as u64,
        max_size_in: sig.amount_in,
        total_cu_estimate: 0,
        total_fees_bps: 0,
        detected_at_ns: sig.emitted_at_ns,
        max_edge_age_ms: 0,
    }
}

/// Spawn a task that consumes ArbSignals over WS and injects into execution path with reconnect + metrics.
pub async fn start_hotpath_ws(
    engine: Arc<ExecutionEngine>,
    metrics: Arc<MetricsServer>,
    ws_url: &str,
) -> Result<()> {
    let base_url = Url::parse(ws_url)?;
    let mut attempt: u32 = 0;
    let max_backoff = Duration::from_secs(30);
    let mut last_connect = Instant::now();
    tokio::spawn(async move {
        loop {
            attempt += 1;
            match connect_async(base_url.clone()).await {
                Ok((ws, _resp)) => {
                    info!("hotpath websocket connected (attempt {})", attempt);
                    metrics.hotpath_ws_reconnections.inc();
                    let (_write, mut read) = ws.split();
                    attempt = 0; // reset after success
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(m) => {
                                if m.is_text() {
                                    metrics.hotpath_ws_messages.inc();
                                    if let Ok(ArbSignal::V1(v1)) =
                                        serde_json::from_str::<ArbSignal>(m.to_text().unwrap_or(""))
                                    {
                                        let opp = map_signal(v1);
                                        let start = Instant::now();
                                        if let Err(e) =
                                            engine.process_hotpath_opportunity(opp).await
                                        {
                                            error!("hotpath_process_error: {}", e);
                                            metrics.hotpath_ws_errors.inc();
                                        }
                                        metrics.hotpath_opportunities_processed.inc();
                                        metrics
                                            .hotpath_processing_time_seconds
                                            .observe(start.elapsed().as_secs_f64());
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("hotpath websocket stream error: {}", e);
                                metrics.hotpath_ws_errors.inc();
                                break;
                            }
                        }
                    }
                    warn!("hotpath websocket disconnected");
                }
                Err(e) => {
                    warn!(
                        "hotpath websocket connect error: {} (attempt {})",
                        e, attempt
                    );
                    metrics.hotpath_ws_errors.inc();
                }
            }
            // backoff
            let since = last_connect.elapsed();
            if since < Duration::from_secs(2) {
                // avoid spin if immediate failures
                let sleep_dur = (Duration::from_millis(500) * attempt).min(max_backoff);
                sleep(sleep_dur).await;
            }
            last_connect = Instant::now();
        }
    });
    Ok(())
}
