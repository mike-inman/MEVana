use metrics::counter;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("Starting test metrics service on port 8087...");
    
    // Initialize metrics exporter
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 8087))
        .install()
        .expect("Failed to install Prometheus recorder");

    println!("Metrics server started. Check http://localhost:8087/metrics");
    
    // Generate test metrics
    tokio::spawn(async {
        let mut count = 0;
        loop {
            sleep(Duration::from_secs(1)).await;
            count += 1;
            
            counter!("test_messages_total", "stream" => "1").increment(1);
            counter!("test_messages_total", "stream" => "2").increment(2);
            counter!("test_messages_total", "stream" => "3").increment(3);
            
            if count % 10 == 0 {
                println!("Generated {} test metrics", count);
            }
        }
    });
    
    // Keep running
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}