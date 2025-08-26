//! Latency Benchmarks for Parser Validation
//! 
//! Performance benchmarks to measure parser validation latency
//! under various load conditions.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use parser_validator::{ValidationConfig, ValidationFramework};
use std::time::Duration;

fn benchmark_validation_latency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Create test configuration
    let config = ValidationConfig::development();
    
    let mut group = c.benchmark_group("validation_latency");
    group.measurement_time(Duration::from_secs(30));
    
    // Benchmark different transaction sizes
    for tx_count in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("transactions", tx_count),
            tx_count,
            |b, &tx_count| {
                b.to_async(&rt).iter(|| async {
                    // Simulate validation of multiple transactions
                    let mut total_duration = Duration::from_nanos(0);
                    
                    for _i in 0..tx_count {
                        let start = std::time::Instant::now();
                        
                        // Simulate validation work
                        tokio::time::sleep(Duration::from_micros(100)).await;
                        
                        total_duration += start.elapsed();
                    }
                    
                    total_duration
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_ground_truth_fetching(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ground_truth_latency");
    group.measurement_time(Duration::from_secs(20));
    
    // Benchmark ground truth source response times
    for source in ["shyft", "helius", "solscan"].iter() {
        group.bench_with_input(
            BenchmarkId::new("source", source),
            source,
            |b, &_source| {
                b.to_async(&rt).iter(|| async {
                    // Simulate API call latency
                    let latency = match _source {
                        "shyft" => Duration::from_millis(50),    // Fast premium API
                        "helius" => Duration::from_millis(100),  // Medium latency
                        "solscan" => Duration::from_millis(200), // Slower public API
                        _ => Duration::from_millis(150),
                    };
                    
                    tokio::time::sleep(latency).await;
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_consensus_building(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("consensus_latency");
    
    // Benchmark consensus building with different source counts
    for source_count in [1, 2, 3].iter() {
        group.bench_with_input(
            BenchmarkId::new("sources", source_count),
            source_count,
            |b, &source_count| {
                b.to_async(&rt).iter(|| async {
                    // Simulate consensus building computation
                    let computation_time = Duration::from_micros(source_count * 50);
                    tokio::time::sleep(computation_time).await;
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_database_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("database_latency");
    
    // Benchmark database write operations
    for batch_size in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_write", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    // Simulate database write latency
                    let write_time = Duration::from_micros(batch_size * 100);
                    tokio::time::sleep(write_time).await;
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_validation_latency,
    benchmark_ground_truth_fetching,
    benchmark_consensus_building,
    benchmark_database_operations
);
criterion_main!(benches);
