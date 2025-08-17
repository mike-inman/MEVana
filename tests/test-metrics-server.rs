// Test Metrics Server
// This server exposes Prometheus metrics for testing our observability pipeline
// Created: 2024-08-17 19:15 CDT

use actix_web::{web, App, HttpServer, Responder};
use prometheus::{Registry, Counter, Histogram, Gauge, Encoder, TextEncoder};
use tokio::time::{Duration, Instant};

lazy_static::lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    // Test execution metrics
    static ref TEST_EXECUTION_COUNTER: Counter = Counter::new(
        "test_execution_total",
        "Total number of test executions"
    ).unwrap();
    
    static ref TEST_SUCCESS_COUNTER: Counter = Counter::new(
        "test_success_total",
        "Total number of successful tests"
    ).unwrap();
    
    static ref TEST_FAILURE_COUNTER: Counter = Counter::new(
        "test_failure_total",
        "Total number of failed tests"
    ).unwrap();
    
    static ref TEST_DURATION: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new("test_duration_seconds", "Test execution duration in seconds")
    ).unwrap();
    
    // Service performance metrics
    static ref SERVICE_LATENCY: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new("service_latency_seconds", "Service response latency in seconds")
    ).unwrap();
    
    static ref MEMORY_USAGE: Gauge = Gauge::new(
        "memory_usage_bytes",
        "Memory usage in bytes"
    ).unwrap();
    
    static ref CPU_USAGE: Gauge = Gauge::new(
        "cpu_usage_percent",
        "CPU usage percentage"
    ).unwrap();
}

fn register_metrics() {
    REGISTRY.register(Box::new(TEST_EXECUTION_COUNTER.clone())).unwrap();
    REGISTRY.register(Box::new(TEST_SUCCESS_COUNTER.clone())).unwrap();
    REGISTRY.register(Box::new(TEST_FAILURE_COUNTER.clone())).unwrap();
    REGISTRY.register(Box::new(TEST_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(SERVICE_LATENCY.clone())).unwrap();
    REGISTRY.register(Box::new(MEMORY_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(CPU_USAGE.clone())).unwrap();
}

async fn metrics() -> impl Responder {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

async fn health() -> impl Responder {
    "OK"
}

async fn test_endpoint() -> impl Responder {
    let start = Instant::now();
    
    // Simulate some work
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let duration = start.elapsed();
    
    // Record metrics
    TEST_EXECUTION_COUNTER.inc();
    TEST_SUCCESS_COUNTER.inc();
    TEST_DURATION.observe(duration.as_secs_f64());
    SERVICE_LATENCY.observe(duration.as_secs_f64());
    
    // Simulate memory and CPU usage
    MEMORY_USAGE.set(1024.0 * 1024.0); // 1MB
    CPU_USAGE.set(25.0); // 25%
    
    format!("Test completed in {:?}", duration)
}

async fn failure_endpoint() -> impl Responder {
    TEST_EXECUTION_COUNTER.inc();
    TEST_FAILURE_COUNTER.inc();
    
    // Simulate error
    actix_web::HttpResponse::InternalServerError()
        .body("Simulated test failure")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting Test Metrics Server");
    println!("📊 Metrics endpoint: http://localhost:9091/metrics");
    println!("🏥 Health endpoint: http://localhost:9091/health");
    println!("🧪 Test endpoint: http://localhost:9091/test");
    println!("❌ Failure endpoint: http://localhost:9091/failure");
    
    // Register metrics
    register_metrics();
    
    // Start server
    HttpServer::new(|| {
        App::new()
            .route("/metrics", web::get().to(metrics))
            .route("/health", web::get().to(health))
            .route("/test", web::get().to(test_endpoint))
            .route("/failure", web::get().to(failure_endpoint))
    })
    .bind("0.0.0.0:9091")?
    .run()
    .await
}
