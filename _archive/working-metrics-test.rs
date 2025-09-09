// Simple test to prove metrics can work
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() {
    // Start a simple HTTP server on 8087
    let app = axum::Router::new()
        .route("/metrics", axum::routing::get(metrics_handler));
    
    // Start counter incrementing
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            COUNTER.fetch_add(1, Ordering::Relaxed);
            println!("Counter: {}", COUNTER.load(Ordering::Relaxed));
        }
    });
    
    println!("Starting metrics server on :8087");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8087").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn metrics_handler() -> String {
    let count = COUNTER.load(Ordering::Relaxed);
    format!(
        "# TYPE test_counter counter\ntest_counter {}\n# TYPE messages_per_second gauge\nmessages_per_second {}\n",
        count, count * 2
    )
}