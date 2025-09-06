use lazy_static::lazy_static;
use prometheus::{HistogramVec, IntCounterVec, IntGauge};

lazy_static! {
    pub static ref TOKEN_ENRICH_REQUESTS: IntCounterVec = prometheus::register_int_counter_vec!(
        "token_enrich_requests_total",
        "Token enrichment requests",
        &["source"]
    )
    .unwrap();
    pub static ref TOKEN_ENRICH_FAILURES: IntCounterVec = prometheus::register_int_counter_vec!(
        "token_enrich_failures_total",
        "Token enrichment failures",
        &["source", "reason"]
    )
    .unwrap();
    pub static ref TOKEN_CONFLICTS: IntCounterVec = prometheus::register_int_counter_vec!(
        "token_conflicts_total",
        "Merge conflicts per field",
        &["field"]
    )
    .unwrap();
    pub static ref TOKEN_TOTAL: IntGauge =
        prometheus::register_int_gauge!("token_total", "Total tokens in hot store").unwrap();
    pub static ref TOKEN_VERSIONS: IntCounterVec = prometheus::register_int_counter_vec!(
        "token_versions_total",
        "Token version rows appended",
        &["mint"]
    )
    .unwrap();
    pub static ref TOKEN_ENRICH_LATENCY: HistogramVec = prometheus::register_histogram_vec!(
        "token_enrich_latency_seconds",
        "Latency per source",
        &["source"],
        vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0]
    )
    .unwrap();
}
