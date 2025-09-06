use crate::stats::{
    CIRCUIT_HALF_OPEN, CIRCUIT_HALF_OPEN_ERRORS, CIRCUIT_HALF_OPEN_TOTAL, CIRCUIT_LAST_OPEN_NS,
    CIRCUIT_OPEN, EDGE_DECODE_ERRORS, EDGE_PROCESS_ERRORS, EDGE_TOTAL,
};
use std::sync::atomic::Ordering as Ao;

/// Evaluate circuit breaker state on each message.
/// msg_error indicates the just-processed message resulted in an error (decode or process).
/// now_ns is current monotonic-ish time (wall-clock ns since epoch acceptable for this logic).
#[derive(Debug, Clone, Copy)]
pub struct CircuitConfig {
    pub threshold: f64,
    pub min_msgs: u64,
    pub cooldown_ms: u64,
    pub half_window: u64,
    pub eval_mod: u64,
}

impl CircuitConfig {
    pub fn from_env() -> Self {
        Self {
            threshold: std::env::var("CIRCUIT_ERROR_RATIO_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.60),
            min_msgs: std::env::var("CIRCUIT_MIN_MESSAGES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1_000),
            cooldown_ms: std::env::var("CIRCUIT_COOLDOWN_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30_000),
            half_window: std::env::var("CIRCUIT_HALF_OPEN_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(200),
            eval_mod: std::env::var("CIRCUIT_EVAL_MOD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
        }
    }
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            threshold: 0.60,
            min_msgs: 1_000,
            cooldown_ms: 30_000,
            half_window: 200,
            eval_mod: 500,
        }
    }
}

pub fn circuit_on_message(msg_error: bool, now_ns: u64) {
    let cfg = CircuitConfig::from_env();
    circuit_on_message_with_config(msg_error, now_ns, &cfg);
}

pub fn circuit_on_message_with_config(msg_error: bool, now_ns: u64, cfg: &CircuitConfig) {
    let total = EDGE_TOTAL.load(Ao::Relaxed);
    let threshold = cfg.threshold;
    let min_msgs = cfg.min_msgs;
    let cooldown_ms = cfg.cooldown_ms;
    let half_window = cfg.half_window;
    let eval_mod = cfg.eval_mod;

    let open = CIRCUIT_OPEN.load(Ao::Relaxed);
    let half = CIRCUIT_HALF_OPEN.load(Ao::Relaxed);
    if open {
        if half {
            CIRCUIT_HALF_OPEN_TOTAL.fetch_add(1, Ao::Relaxed);
            if msg_error {
                CIRCUIT_HALF_OPEN_ERRORS.fetch_add(1, Ao::Relaxed);
            }
            let h_total = CIRCUIT_HALF_OPEN_TOTAL.load(Ao::Relaxed);
            if h_total >= half_window {
                let h_errs = CIRCUIT_HALF_OPEN_ERRORS.load(Ao::Relaxed);
                let ratio = if h_total > 0 {
                    h_errs as f64 / h_total as f64
                } else {
                    0.0
                };
                if ratio <= threshold {
                    // close circuit
                    CIRCUIT_OPEN.store(false, Ao::Relaxed);
                    CIRCUIT_HALF_OPEN.store(false, Ao::Relaxed);
                    tracing::info!(ratio, h_total, "circuit_closed_half_open_success");
                } else {
                    // re-open fully
                    CIRCUIT_HALF_OPEN.store(false, Ao::Relaxed);
                    CIRCUIT_LAST_OPEN_NS.store(now_ns, Ao::Relaxed);
                    CIRCUIT_HALF_OPEN_TOTAL.store(0, Ao::Relaxed);
                    CIRCUIT_HALF_OPEN_ERRORS.store(0, Ao::Relaxed);
                    tracing::warn!(ratio, h_total, "circuit_reopened_after_half_open_failure");
                }
            }
        } else {
            let last_open = CIRCUIT_LAST_OPEN_NS.load(Ao::Relaxed);
            if last_open > 0 {
                let elapsed_ms = if now_ns > last_open {
                    (now_ns - last_open) / 1_000_000
                } else {
                    0
                };
                if elapsed_ms >= cooldown_ms {
                    CIRCUIT_HALF_OPEN.store(true, Ao::Relaxed);
                    CIRCUIT_HALF_OPEN_TOTAL.store(0, Ao::Relaxed);
                    CIRCUIT_HALF_OPEN_ERRORS.store(0, Ao::Relaxed);
                    tracing::info!(elapsed_ms, "circuit_half_open_enter");
                }
            }
        }
    } else if eval_mod > 0 && total % eval_mod == 0 && total >= min_msgs {
        let dec_err = EDGE_DECODE_ERRORS.load(Ao::Relaxed);
        let proc_err = EDGE_PROCESS_ERRORS.load(Ao::Relaxed);
        let err_ratio = (dec_err + proc_err) as f64 / total as f64;
        if err_ratio > threshold {
            CIRCUIT_OPEN.store(true, Ao::Relaxed);
            CIRCUIT_LAST_OPEN_NS.store(now_ns, Ao::Relaxed);
            CIRCUIT_HALF_OPEN.store(false, Ao::Relaxed);
            CIRCUIT_HALF_OPEN_TOTAL.store(0, Ao::Relaxed);
            CIRCUIT_HALF_OPEN_ERRORS.store(0, Ao::Relaxed);
            tracing::warn!(err_ratio, total, "circuit_opened");
        }
    }
}
