use reqwest::blocking::Client;
use solana_sdk::signature::{write_keypair_file, Keypair};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

// Basic smoke: start service (dry-run, disable external RPC), check health + metrics endpoints.
#[test]
fn smoke_connect() {
    let tmp = tempfile::tempdir().unwrap();
    let wallet_path = tmp.path().join("smoke-wallet.json");
    write_keypair_file(&Keypair::new(), &wallet_path).unwrap();
    let metrics_port = unused_port();
    let registry_port = unused_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_executor-service"))
        .current_dir(tmp.path())
        .env_clear()
        .env("EXECUTION_DRY_RUN", "1")
        .env("EXECUTION_ALLOW_LIVE", "0")
        .env("EXECUTION_LIVE_FLAG_INITIAL", "0")
        .env("ENABLE_LIVE_EDGE_STREAM", "0")
        .env("DISABLE_DB", "1")
        .env("ALLOW_EMPTY_SHYFT_API", "1")
        .env(
            "DATABASE_URL",
            "postgresql://unused:unused@127.0.0.1:1/unused",
        )
        .env("KAFKA_BROKER", "127.0.0.1:1")
        .env("SHYFT_RPC_URL", "http://127.0.0.1:1")
        .env("SHYFT_JITO_URL", "http://127.0.0.1:1")
        .env("WALLET_KEYPAIR_PATH", &wallet_path)
        .env("METRICS_PORT", metrics_port.to_string())
        .env("TOKEN_REGISTRY_PORT", registry_port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start executor-service");

    let start = Instant::now();
    let health_url = format!("http://127.0.0.1:{metrics_port}/health");
    let metrics_url = format!("http://127.0.0.1:{metrics_port}/metrics");
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let mut health_ok = false;
    let mut metrics_ok = false;
    while start.elapsed() < Duration::from_secs(30) {
        if !health_ok {
            if let Ok(r) = client.get(&health_url).send() {
                if r.status().is_success() {
                    health_ok = true;
                }
            }
        }
        if !metrics_ok {
            if let Ok(r) = client.get(&metrics_url).send() {
                if r.status().is_success() {
                    metrics_ok = true;
                }
            }
        }
        if health_ok && metrics_ok {
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    let _ = child.kill();
    let _ = child.wait();
    assert!(health_ok, "health endpoint not reachable in time");
    assert!(metrics_ok, "metrics endpoint not reachable in time");
}
