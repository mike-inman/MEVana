use arb_engine::circuit::{circuit_on_message_with_config, CircuitConfig};
use arb_engine::stats::{
    CIRCUIT_HALF_OPEN, CIRCUIT_HALF_OPEN_ERRORS, CIRCUIT_HALF_OPEN_TOTAL, CIRCUIT_LAST_OPEN_NS,
    CIRCUIT_OPEN, EDGE_DECODE_ERRORS, EDGE_PROCESS_ERRORS, EDGE_TOTAL,
};
use std::sync::atomic::Ordering as Ao;
use std::sync::Mutex;

// These tests exercise process-wide circuit state and must not reset it concurrently.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn reset_state() {
    EDGE_TOTAL.store(0, Ao::Relaxed);
    EDGE_DECODE_ERRORS.store(0, Ao::Relaxed);
    EDGE_PROCESS_ERRORS.store(0, Ao::Relaxed);
    CIRCUIT_OPEN.store(false, Ao::Relaxed);
    CIRCUIT_HALF_OPEN.store(false, Ao::Relaxed);
    CIRCUIT_HALF_OPEN_TOTAL.store(0, Ao::Relaxed);
    CIRCUIT_HALF_OPEN_ERRORS.store(0, Ao::Relaxed);
    CIRCUIT_LAST_OPEN_NS.store(0, Ao::Relaxed);
}

#[test]
fn opens_after_threshold() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_state();
    let cfg = CircuitConfig {
        threshold: 0.5,
        min_msgs: 50,
        cooldown_ms: 1,
        half_window: 10,
        eval_mod: 10,
    };
    let mut now = 1_000_000_000u64;
    for i in 1..=100u64 {
        EDGE_TOTAL.store(i, Ao::Relaxed);
        let is_error = i % 5 != 0; // 80% errors
        if is_error {
            EDGE_DECODE_ERRORS.fetch_add(1, Ao::Relaxed);
        }
        circuit_on_message_with_config(is_error, now, &cfg);
        now += 1_000_000;
    }
    assert!(CIRCUIT_OPEN.load(Ao::Relaxed), "circuit should be open");
}

#[test]
fn half_open_and_close_success() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_state();
    let cfg = CircuitConfig {
        threshold: 0.25,
        min_msgs: 0,
        cooldown_ms: 0,
        half_window: 8,
        eval_mod: 1,
    };
    // Directly seed half-open state
    CIRCUIT_OPEN.store(true, Ao::Relaxed);
    CIRCUIT_HALF_OPEN.store(true, Ao::Relaxed);
    CIRCUIT_HALF_OPEN_TOTAL.store(cfg.half_window - 1, Ao::Relaxed); // one short of completion
    CIRCUIT_HALF_OPEN_ERRORS.store(0, Ao::Relaxed);
    let now = 10_000_000_000u64; // arbitrary
                                 // Final clean message completes window => should close
    circuit_on_message_with_config(false, now, &cfg);
    assert!(
        !CIRCUIT_OPEN.load(Ao::Relaxed),
        "circuit should close after clean window"
    );
    assert!(!CIRCUIT_HALF_OPEN.load(Ao::Relaxed));
}

#[test]
fn half_open_reopens_on_errors() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_state();
    let cfg = CircuitConfig {
        threshold: 0.25,
        min_msgs: 0,
        cooldown_ms: 0,
        half_window: 6,
        eval_mod: 1,
    };
    CIRCUIT_OPEN.store(true, Ao::Relaxed);
    CIRCUIT_HALF_OPEN.store(true, Ao::Relaxed);
    // Set counters so one more error message completes window with high error ratio
    CIRCUIT_HALF_OPEN_TOTAL.store(cfg.half_window - 1, Ao::Relaxed);
    CIRCUIT_HALF_OPEN_ERRORS.store(cfg.half_window - 1, Ao::Relaxed); // all previous were errors
    let now = 42_000_000_000u64;
    circuit_on_message_with_config(true, now, &cfg); // final error drives ratio=1.0 > threshold
    assert!(
        CIRCUIT_OPEN.load(Ao::Relaxed),
        "circuit should remain open (re-opened)"
    );
    assert!(
        !CIRCUIT_HALF_OPEN.load(Ao::Relaxed),
        "half-open window should end"
    );
    assert_eq!(
        CIRCUIT_HALF_OPEN_TOTAL.load(Ao::Relaxed),
        0,
        "window counters reset"
    );
    assert_eq!(
        CIRCUIT_HALF_OPEN_ERRORS.load(Ao::Relaxed),
        0,
        "error counters reset"
    );
}
