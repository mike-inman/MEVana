use dashmap::DashMap;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::mpsc,
    time::{Duration, Instant},
};
use tracing::info;

use crate::models::{ArbSignal, DexEvent, Pair, Quote};
use crate::plugin::DexPlugin;

pub enum StreamEvent {
    /// Upstream (your Kafka/redis/geyser layer) pushes already-normalized JSONs here.
    RawUpdate(serde_json::Value),
}

pub struct Engine {
    plugins: Vec<Arc<dyn DexPlugin>>,
    // The last quotes we saw per (pair, dex, side_ab, amount_in)
    last_quotes: DashMap<(Pair, String, bool, u128), Quote>,
}

impl Engine {
    pub fn new(plugins: Vec<Arc<dyn DexPlugin>>) -> Self {
        Self {
            plugins,
            last_quotes: DashMap::new(),
        }
    }

    /// Feed raw geyser-derived JSON updates (accounts or tx). The plugins will emit DexEvents.
    pub async fn handle_update(&self, raw: serde_json::Value) -> anyhow::Result<Vec<DexEvent>> {
        // naive: broadcast to all plugins; you can route by program id if you add it to the JSON
        let mut out = Vec::new();
        for p in &self.plugins {
            match p.handle_galpha(&raw).await {
                Ok(mut v) if !v.is_empty() => out.append(&mut v),
                _ => {}
            }
        }
        Ok(out)
    }

    /// Ask every plugin that supports the pair for quick quotes on multiple size buckets.
    pub async fn refresh_quotes_for_pair(
        &self,
        pair: &Pair,
        amount_buckets: &[u128],
    ) -> anyhow::Result<Vec<Quote>> {
        let mut quotes = Vec::new();
        for p in &self.plugins {
            let mut v = p
                .quote_exact_in(pair, amount_buckets)
                .await
                .unwrap_or_default();
            if !v.is_empty() {
                for q in &v {
                    self.last_quotes.insert(
                        (pair.clone(), q.dex.clone(), q.side_ab, q.amount_in),
                        q.clone(),
                    );
                }
                quotes.append(&mut v);
            }
        }
        Ok(quotes)
    }

    /// Very fast 2-leg cycle check across DEXs on the same pair using latest quotes.
    pub fn scan_simple_cycle(
        &self,
        pair: &Pair,
        amount_in: u128,
        now_slot: u64,
    ) -> Option<ArbSignal> {
        // Collect A->B from each dex, then B->A from each dex, and compare across dexes
        let mut a_to_b: HashMap<String, Quote> = HashMap::new();
        let mut b_to_a: HashMap<String, Quote> = HashMap::new();

        for r in self.last_quotes.iter() {
            let (kpair, kdex, side_ab, ain) = r.key();
            if *kpair != *pair || *ain != amount_in {
                continue;
            }
            if *side_ab {
                a_to_b.insert(kdex.clone(), (*r).clone());
            } else {
                b_to_a.insert(kdex.clone(), (*r).clone());
            }
        }

        let mut best: Option<ArbSignal> = None;
        for (sell_dex, q1) in &a_to_b {
            for (buy_dex, q2) in &b_to_a {
                if sell_dex == buy_dex {
                    continue;
                }
                // A -> B on sell_dex, then B -> A on buy_dex
                let back = q2.amount_out as i128; // assumes same amount_in; in a full version use q2 with amount_in = q1.amount_out
                let profit = back - (q1.amount_in as i128);

                if profit > 0 {
                    let sig = ArbSignal {
                        pair: pair.clone(),
                        dex_sell: (*sell_dex).clone(),
                        dex_buy: (*buy_dex).clone(),
                        amount_in,
                        expected_profit_out: profit,
                        slot: now_slot,
                    };
                    if best
                        .as_ref()
                        .map(|b| b.expected_profit_out)
                        .unwrap_or_default()
                        < profit
                    {
                        best = Some(sig);
                    }
                }
            }
        }
        best
    }
}

/// Example runner that binds the pieces together.
/// In your real system, replace the channel with your Kafka consumer → `StreamEvent::RawUpdate`.
pub async fn run_engine(
    mut rx: mpsc::Receiver<StreamEvent>,
    engine: Arc<Engine>,
) -> anyhow::Result<()> {
    let amount_buckets = [1_000u128, 10_000u128, 100_000u128]; // tune per token/decimals
    let mut last_refresh = Instant::now();
    loop {
        tokio::select! {
            maybe_msg = rx.recv() => {
                if let Some(StreamEvent::RawUpdate(v)) = maybe_msg {
                    let events = engine.handle_update(v).await?;
                    // On pool updates, refresh quotes for that pair quickly
                    for e in events {
                        if let DexEvent::PoolUpdate(snap) = e {
                            let pair = Pair::canonical(&snap.token_a_mint, &snap.token_b_mint);
                            engine.refresh_quotes_for_pair(&pair, &amount_buckets).await?;
                            // quick pass arb check
                            if let Some(sig) = engine.scan_simple_cycle(&pair, amount_buckets[0], snap.slot) {
                                // TODO: emit to Kafka "arb.signal", or push to your executor
                                info!(?sig, "arb-signal");
                            }
                        }
                    }
                } else {
                    // channel closed → end gracefully
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                // periodic maintenance: optional global rescans, stats, etc.
                if last_refresh.elapsed() > Duration::from_secs(5) {
                    last_refresh = Instant::now();
                }
            }
        }
    }
    Ok(())
}
