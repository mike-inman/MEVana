use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::main] 
async fn main() {
    println!("Starting simple counter on port 8087");
    
    // Increment counter every second
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let count = COUNTER.fetch_add(1, Ordering::Relaxed);
            println!("Count: {}", count + 1);
        }
    });
    
    // Simple HTTP server
    let app = axum::Router::new()
        .route("/metrics", axum::routing::get(serve_metrics));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8087").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_metrics() -> String {
    let count = COUNTER.load(Ordering::Relaxed);
    format!("simple_counter {}\n", count)
}