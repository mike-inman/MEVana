use arb_engine::metrics_http::build_http_router;
use axum::http::Request;
use metrics_exporter_prometheus::PrometheusBuilder;
use once_cell::sync::OnceCell;
use serde_json::json;
use tower::util::ServiceExt;

static HANDLE: OnceCell<metrics_exporter_prometheus::PrometheusHandle> = OnceCell::new();

#[tokio::test]
#[serial_test::serial]
async fn log_rotation_triggers_on_size() {
    if std::env::var("SKIP_SLOW_TESTS").is_ok() {
        return;
    }
    // Unique temp dir
    let tmp_dir = std::env::temp_dir().join(format!(
        "mevana_log_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    std::env::set_var("AI_LOG_DIR", &tmp_dir);
    std::env::set_var("AI_API_TOKEN", "rotok");
    std::env::set_var("AI_LOG_MAX_BYTES", "400"); // small to force rotation quickly
    std::env::set_var("AI_LOG_MAX_ROTATIONS", "3");

    let handle = HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .unwrap_or_else(|_| {
                    PrometheusBuilder::new()
                        .install_recorder()
                        .expect("prometheus init failed twice")
                })
        })
        .clone();
    let app = build_http_router(handle, None);

    // Write enough advice entries with large payloads to exceed size threshold
    let payload = "a".repeat(300);
    for _ in 0..5 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/ai/advice")
                    .method("POST")
                    .header("X-AI-Token", "rotok")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        json!({"message":payload,"tag":"rot"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }
    // Poll briefly for rotated file instead of fixed sleep to minimize test time.
    let rotated = tmp_dir.join("advice.jsonl.1");
    let mut found = false;
    for _ in 0..20 {
        // up to ~100ms worst-case
        if rotated.exists() {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(found, "expected rotated log file {:?} to exist", rotated);
}
