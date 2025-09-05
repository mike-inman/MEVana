use reqwest::blocking::get;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// Basic smoke: start service (dry-run, disable external RPC), check health + metrics endpoints.
#[test]
fn smoke_connect() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "executor-service", "--bin", "executor-service"])
        .env("EXECUTION_DRY_RUN", "1")
        .env("DISABLE_DB", "1") // skip DB for offline connectivity
        .env("METRICS_PORT", "9393")
        .env("TOKEN_REGISTRY_PORT", "9998")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start executor-service");

    let start = Instant::now();
    let health_url = "http://127.0.0.1:9393/health"; // metrics server health
    let metrics_url = "http://127.0.0.1:9393/metrics";
    let mut health_ok = false;
    let mut metrics_ok = false;
    while start.elapsed() < Duration::from_secs(30) {
        if !health_ok {
            if let Ok(r) = get(health_url) {
                if r.status().is_success() {
                    health_ok = true;
                }
            }
        }
        if !metrics_ok {
            if let Ok(r) = get(metrics_url) {
                if r.status().is_success() {
                    metrics_ok = true;
                }
            }
        }
        if health_ok && metrics_ok {
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    child.kill().ok();
    assert!(health_ok, "health endpoint not reachable in time");
    assert!(metrics_ok, "metrics endpoint not reachable in time");
}
