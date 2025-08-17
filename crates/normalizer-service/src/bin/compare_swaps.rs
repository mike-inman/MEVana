use anyhow::Result;
use dotenv::dotenv;
use env_logger;
use log::{info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize, Clone)]
struct V1Swap {
    sig: String,
    dex: String,
    token_in: Option<String>,
    token_out: Option<String>,
    amount_in: Option<f64>,
    amount_out: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct V2Swap {
    signature: String,
    dex: String,
    token_in_mint: String,
    token_out_mint: String,
    amount_in: u128,
    amount_out: u128,
    price: f64,
}

#[derive(Default)]
struct State {
    v1_by_sig: HashMap<String, V1Swap>,
    v2_by_sig: HashMap<String, V2Swap>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let topic_v1 = std::env::var("TOPIC_V1").unwrap_or_else(|_| "orca-swaps.parsed".into());
    let topic_v2 = std::env::var("TOPIC_V2").unwrap_or_else(|_| "orca-swaps.parsed.v2".into());

    info!("compare-swaps: broker={broker} v1={topic_v1} v2={topic_v2}");

    let state = Arc::new(Mutex::new(State::default()));

    // V1 consumer
    {
        let broker = broker.clone();
        let topic = topic_v1.clone();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let c: StreamConsumer = ClientConfig::new()
                .set("bootstrap.servers", &broker)
                .set("group.id", "compare-swaps-v1")
                .set("enable.auto.commit", "true")
                .set("auto.offset.reset", "latest")
                .create()
                .expect("consumer v1");
            c.subscribe(&[&topic]).expect("subscribe v1");
            loop {
                match c.recv().await {
                    Ok(m) => {
                        if let Some(p) = m.payload() {
                            if let Ok(v) = serde_json::from_slice::<V1Swap>(p) {
                                let mut s = state.lock().await;
                                s.v1_by_sig.insert(v.sig.clone(), v);
                            }
                        }
                    }
                    Err(e) => warn!("v1 recv error: {e:?}"),
                }
            }
        });
    }

    // V2 consumer
    {
        let broker = broker.clone();
        let topic = topic_v2.clone();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let c: StreamConsumer = ClientConfig::new()
                .set("bootstrap.servers", &broker)
                .set("group.id", "compare-swaps-v2")
                .set("enable.auto.commit", "true")
                .set("auto.offset.reset", "latest")
                .create()
                .expect("consumer v2");
            c.subscribe(&[&topic]).expect("subscribe v2");
            loop {
                match c.recv().await {
                    Ok(m) => {
                        if let Some(p) = m.payload() {
                            if let Ok(v) = serde_json::from_slice::<V2Swap>(p) {
                                let mut s = state.lock().await;
                                s.v2_by_sig.insert(v.signature.clone(), v);
                            }
                        }
                    }
                    Err(e) => warn!("v2 recv error: {e:?}"),
                }
            }
        });
    }

    // Periodically compare overlaps
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        let mut comparisons = 0usize;
        let mut mint_mismatch = 0usize;
        let mut missing_fields = 0usize;

        {
            let s = state.lock().await;
            for (sig, v1) in s.v1_by_sig.iter() {
                if let Some(v2) = s.v2_by_sig.get(sig) {
                    comparisons += 1;

                    let mints_match = v1
                        .token_in
                        .as_ref()
                        .map(|m| m == &v2.token_in_mint)
                        .unwrap_or(false)
                        && v1
                            .token_out
                            .as_ref()
                            .map(|m| m == &v2.token_out_mint)
                            .unwrap_or(false);
                    if !mints_match {
                        mint_mismatch += 1;
                    }

                    let p1 = match (v1.amount_in, v1.amount_out) {
                        (Some(ai), Some(ao)) if ai > 0.0 && ao > 0.0 => Some(ao / ai),
                        _ => None,
                    };
                    if p1.is_none() {
                        missing_fields += 1;
                    }

                    if let Some(p1v) = p1 {
                        let p2v = v2.price;
                        let diff = (p1v - p2v).abs();
                        info!(
                            "cmp sig={} mints={} v1_price={:.6} v2_price={:.6} diff={:.6}",
                            sig, mints_match, p1v, p2v, diff
                        );
                    } else {
                        info!(
                            "cmp sig={} mints={} v1_price=NA v2_price={:.6}",
                            sig, mints_match, v2.price
                        );
                    }
                }
            }
        }

        info!(
            "compare summary: pairs={} mint_mismatch={} v1_missing_fields={}",
            comparisons, mint_mismatch, missing_fields
        );
    }
}
