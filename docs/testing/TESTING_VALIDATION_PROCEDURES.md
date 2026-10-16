# MEVana Testing & Validation Procedures
Last updated: 2025-10-01

**Document Version:** 1.0  
**Created:** September 12, 2025  
**Target:** Production readiness validation  
**Scope:** Comprehensive testing framework for execution integration

---

## 🎯 Testing Strategy Overview

### **Testing Philosophy**
Given that MEVana has sophisticated existing components, our testing focuses on **integration validation** rather than unit testing individual components. The primary risk is in the connections between systems, not the systems themselves.

**Hotpath live-data coverage.** The `crates/hotpath_arb_engine/tests/live_transaction_data_test.rs` suite now treats live Solana + Prometheus checks as opt-in so CI and offline developers stay green. Export `RUN_HOTPATH_LIVE_TESTS=1` (any of `1/true/on/yes`, case-insensitive) before running the suite to exercise:
- `test_live_transaction_data_flow_integration`
- `test_geyser_endpoint_connectivity`
- `test_metrics_endpoint_availability`

When the flag is set, ensure the local hotpath binary is running with metrics on `http://localhost:8088/metrics`, Prometheus reachable at `http://localhost:9090`, and the property store toggles flipped to `edge_detection_enabled=true`, `heuristic_detection_enabled=false` so edge-driven opportunities flow.

### **Risk-Based Testing Priorities**
1. **CRITICAL**: Execution integration (HotPath → Executor Service)
2. **HIGH**: AI strategy integration and learning feedback loops
3. **MEDIUM**: Performance under load and memory stability
4. **LOW**: Individual component functionality (already proven)

---

## 🧪 Phase 1: Unit & Integration Testing (Week 1)

### **Test 1.1: Execution Integration Tests**
**Priority**: CRITICAL  
**Location**: `crates/hotpath_arb_engine/tests/execution_integration_test.rs`

```rust
#[cfg(test)]
mod execution_integration_tests {
    use super::*;
    use executor_service::execution_orchestrator::{ExecutionMode, ExecutionOrchestrator};
    use tokio_test;

    #[tokio::test]
    async fn test_opportunity_conversion() {
        // Test: HotPath opportunity → Executor opportunity conversion
        let hotpath_opportunity = create_mock_hotpath_opportunity();
        let executor_opportunity = convert_to_executor_opportunity(&hotpath_opportunity);
        
        assert_eq!(executor_opportunity.opportunity_id, hotpath_opportunity.id);
        assert_eq!(executor_opportunity.profit_lamports, expected_profit);
        assert!(executor_opportunity.expires_at > chrono::Utc::now());
    }

    #[tokio::test] 
    async fn test_execution_mode_selection() {
        // Test: Configuration → ExecutionMode mapping
        let config = HotPathConfig {
            use_flash_loan: true,
            max_slippage_bps: 50,
            paper_trading: false,
        };
        
        let execution_mode = determine_execution_mode(&config, &opportunity);
        
        match execution_mode {
            ExecutionMode::Live { max_slippage_bps, .. } => {
                assert_eq!(max_slippage_bps, 50);
            }
            _ => panic!("Expected Live mode for production config"),
        }
    }

    #[tokio::test]
    async fn test_error_handling_execution_failure() {
        // Test: Proper error handling when execution fails
        let mock_orchestrator = create_failing_orchestrator();
        let engine = create_test_engine(mock_orchestrator);
        
        let result = engine.execute_real_trade(mock_opportunity).await;
        
        assert!(result.is_ok()); // Should not panic
        assert_eq!(result.unwrap(), false); // Should return false on failure
        assert_eq!(engine.failed_trades.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_success_metrics_update() {
        // Test: Success metrics are properly updated
        let mock_orchestrator = create_successful_orchestrator();
        let engine = create_test_engine(mock_orchestrator);
        
        let result = engine.execute_real_trade(mock_opportunity).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert_eq!(engine.successful_trades.load(Ordering::Relaxed), 1);
        assert!(engine.total_profit_lamports.load(Ordering::Relaxed) > 0);
    }
}
```

### **Test 1.2: AI Integration Tests**
**Priority**: HIGH  
**Location**: `crates/hotpath_arb_engine/tests/ai_integration_test.rs`

```rust
#[tokio::test]
async fn test_ai_strategy_selection() {
    // Test: AI engine selects appropriate strategy for market conditions
    let ai_engine = create_test_ai_engine();
    let market_conditions = MarketConditions {
        volatility: VolatilityLevel::High,
        liquidity: LiquidityLevel::Medium,
        competition: CompetitionLevel::Low,
    };
    
    let strategy = ai_engine.get_optimal_strategy(&market_conditions).await?;
    
    assert_eq!(strategy.strategy_type, StrategyType::BlitzkriegArb);
    assert!(strategy.confidence_score > 0.7);
}

#[tokio::test]
async fn test_learning_feedback_loop() {
    // Test: AI learns from execution results
    let ai_engine = create_test_ai_engine();
    let execution_result = ExecutionResult {
        success: true,
        profit_lamports: 10000,
        execution_time_ms: 450,
        strategy_used: StrategyType::PrecisionSniper,
    };
    
    let initial_q_value = ai_engine.get_q_value(&state_action_pair);
    ai_engine.update_from_execution_result(&execution_result).await?;
    let updated_q_value = ai_engine.get_q_value(&state_action_pair);
    
    assert!(updated_q_value > initial_q_value); // Should improve with success
}
```

### **Test 1.3: Configuration Integration Tests**
**Priority**: HIGH  
**Location**: `crates/hotpath_arb_engine/tests/config_integration_test.rs`

```rust
#[tokio::test]
async fn test_real_time_config_propagation() {
    // Test: Configuration changes propagate to execution decisions
    let engine = create_test_engine();
    
    // Initial configuration
    update_smart_filter_config("min_profit_bps", 5).await?;
    tokio::time::sleep(Duration::from_millis(100)).await; // Allow propagation
    
    let low_profit_opportunity = create_opportunity_with_profit_bps(3);
    let result = engine.should_execute_opportunity(&low_profit_opportunity).await;
    assert_eq!(result, false); // Below threshold
    
    // Update configuration
    update_smart_filter_config("min_profit_bps", 2).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let result = engine.should_execute_opportunity(&low_profit_opportunity).await;
    assert_eq!(result, true); // Now above threshold
}
```

---

## 🔄 Phase 2: End-to-End Integration Testing (Week 1-2)

### **Test 2.1: Complete Pipeline Test**
**Priority**: CRITICAL  
**Environment**: Docker testnet with mock data

```bash
#!/bin/bash
# File: tests/integration/full_pipeline_test.sh

echo "🔄 Starting full pipeline integration test..."

# 1. Start infrastructure
docker-compose -f tests/docker-compose.test.yml up -d
sleep 30

# 2. Start services with test configuration
export DATABASE_URL="postgresql://test:test@localhost:5434/testdb"
export PAPER_TRADING=true
export RUST_LOG=debug

# Start monitoring service
cargo run -p monitoring-service &
MONITORING_PID=$!

# Start hotpath engine with test config
cargo run -p hotpath_arb_engine --bin hotpath_arb_engine &
HOTPATH_PID=$!

sleep 15

# 3. Inject test opportunities
echo "📊 Injecting test opportunities..."
curl -X POST http://localhost:8088/test/inject-opportunity \
  -H "Content-Type: application/json" \
  -d '{
    "token_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "profit_bps": 15,
    "max_size_in": 100000000,
    "venue_a": "Raydium-CLMM",
    "venue_b": "Orca-Whirlpool"
  }'

# 4. Wait and verify execution
sleep 5
EXECUTIONS=$(curl -s http://localhost:8088/performance | jq '.successful_trades')

if [ "$EXECUTIONS" -gt "0" ]; then
    echo "✅ Pipeline test PASSED: $EXECUTIONS executions completed"
    exit 0
else
    echo "❌ Pipeline test FAILED: No executions detected"
    exit 1
fi

# Cleanup
kill $MONITORING_PID $HOTPATH_PID
docker-compose -f tests/docker-compose.test.yml down
```

### **Test 2.2: Database Integration Test**
**Priority**: HIGH  
**Purpose**: Verify complete data flow through AsyncHistoryBuffer

```sql
-- File: tests/integration/database_integration_test.sql

-- Test: Verify opportunity detection writes to database
INSERT INTO test_opportunities (
    token_mint, profit_bps, max_size_in, detected_at, venue_a, venue_b
) VALUES (
    'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
    12, 50000000, NOW(), 'Raydium-CLMM', 'Orca-Whirlpool'
);

-- Test: Verify execution results are recorded
SELECT 
    COUNT(*) as total_opportunities,
    COUNT(execution_result) as executed_opportunities,
    AVG(profit_bps) as avg_profit_bps,
    AVG(execution_time_ms) as avg_execution_time
FROM arbitrage_opportunities 
WHERE timestamp > NOW() - INTERVAL '1 hour';

-- Expected results validation
-- total_opportunities > 0
-- executed_opportunities > 0  
-- avg_profit_bps > min_profit_threshold
-- avg_execution_time < 1000ms
```

---

## 🏋️ Phase 3: Load & Performance Testing (Week 2)

### **Test 3.1: High-Frequency Processing Test**
**Priority**: HIGH  
**Purpose**: Verify system handles production-level opportunity volume

```rust
// File: tests/performance/high_frequency_test.rs

#[tokio::test]
async fn test_1000_opportunities_per_minute() {
    let engine = create_test_engine().await;
    let start_time = Instant::now();
    
    // Simulate 1000 opportunities over 1 minute
    for i in 0..1000 {
        let opportunity = create_varied_opportunity(i);
        
        // Non-blocking processing
        tokio::spawn(async move {
            engine.process_opportunity(opportunity).await
        });
        
        // 60ms intervals = 1000/minute
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    
    // Wait for all processing to complete
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    let total_time = start_time.elapsed();
    let successful_trades = engine.successful_trades.load(Ordering::Relaxed);
    let failed_trades = engine.failed_trades.load(Ordering::Relaxed);
    
    // Validate performance targets
    assert!(total_time.as_secs() < 90); // Should complete within 1.5 minutes
    assert!(successful_trades > 100); // At least 10% success rate
    assert!(failed_trades + successful_trades >= 900); // Most processed
    
    println!("📊 Performance Results:");
    println!("  Total time: {:?}", total_time);
    println!("  Successful: {}", successful_trades);
    println!("  Failed: {}", failed_trades);
    println!("  Success rate: {:.2}%", successful_trades as f64 / 1000.0 * 100.0);
}
```

### **Test 3.2: Memory Stability Test**
**Priority**: HIGH  
**Purpose**: Verify no memory leaks during extended operation

```rust
// File: tests/performance/memory_stability_test.rs

#[tokio::test]
async fn test_24_hour_memory_stability() {
    let engine = create_test_engine().await;
    let initial_memory = get_process_memory_usage();
    let start_time = Instant::now();
    
    // Simulate 24 hours of operation (compressed to 10 minutes for testing)
    let test_duration = Duration::from_secs(600); // 10 minutes
    let opportunity_interval = Duration::from_millis(100); // 10/second
    
    while start_time.elapsed() < test_duration {
        let opportunity = create_random_opportunity();
        engine.process_opportunity(opportunity).await;
        
        tokio::time::sleep(opportunity_interval).await;
        
        // Check memory every minute
        if start_time.elapsed().as_secs() % 60 == 0 {
            let current_memory = get_process_memory_usage();
            let memory_growth = current_memory - initial_memory;
            
            // Memory should not grow continuously
            assert!(memory_growth < initial_memory * 0.5); // Max 50% growth
            println!("Memory usage: {} MB (growth: {} MB)", 
                    current_memory / 1_000_000, memory_growth / 1_000_000);
        }
    }
    
    let final_memory = get_process_memory_usage();
    let total_growth = final_memory - initial_memory;
    
    // Final validation
    assert!(total_growth < initial_memory * 0.3); // Max 30% total growth
    println!("✅ Memory stability test PASSED");
    println!("  Initial: {} MB", initial_memory / 1_000_000);
    println!("  Final: {} MB", final_memory / 1_000_000);
    println!("  Growth: {} MB ({:.2}%)", 
            total_growth / 1_000_000,
            total_growth as f64 / initial_memory as f64 * 100.0);
}
```

### **Test 3.3: Database Connection Pool Stress Test**
**Priority**: MEDIUM  
**Purpose**: Verify database connections handle high load

```rust
// File: tests/performance/database_stress_test.rs

#[tokio::test]
async fn test_concurrent_database_operations() {
    let pool = create_test_db_pool().await;
    let concurrent_operations = 100;
    let operations_per_worker = 1000;
    
    let start_time = Instant::now();
    
    // Spawn concurrent workers
    let mut handles = Vec::new();
    for worker_id in 0..concurrent_operations {
        let pool_clone = pool.clone();
        
        let handle = tokio::spawn(async move {
            for op_id in 0..operations_per_worker {
                // Mix of read and write operations
                if op_id % 2 == 0 {
                    // Write operation
                    insert_test_opportunity(&pool_clone, worker_id, op_id).await?;
                } else {
                    // Read operation
                    query_recent_opportunities(&pool_clone).await?;
                }
            }
            Ok::<(), anyhow::Error>(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all workers to complete
    for handle in handles {
        handle.await??;
    }
    
    let total_time = start_time.elapsed();
    let total_operations = concurrent_operations * operations_per_worker;
    let ops_per_second = total_operations as f64 / total_time.as_secs_f64();
    
    // Validate performance
    assert!(ops_per_second > 1000.0); // At least 1000 ops/second
    assert!(total_time.as_secs() < 30); // Complete within 30 seconds
    
    println!("📊 Database Stress Test Results:");
    println!("  Total operations: {}", total_operations);
    println!("  Total time: {:?}", total_time);
    println!("  Ops/second: {:.2}", ops_per_second);
}
```

---

## 🌐 Phase 4: Testnet Validation (Week 2)

### **Test 4.1: Testnet Environment Setup**
```bash
#!/bin/bash
# File: scripts/setup_testnet_validation.sh

echo "🌐 Setting up testnet validation environment..."

# 1. Configure for testnet
export SOLANA_RPC_URL="https://api.testnet.solana.com"
export GEYSER_ENDPOINT="wss://api.testnet.solana.com"
export DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb_testnet"

# 2. Create testnet database
createdb -h localhost -p 5433 -U mev mevdb_testnet
psql -h localhost -p 5433 -U mev -d mevdb_testnet -f database/schema.sql

# 3. Fund test wallets with testnet SOL
solana config set --url testnet
solana airdrop 10 # Request testnet SOL

# 4. Deploy test token if needed
# [Token deployment commands]

echo "✅ Testnet environment ready"
```

### **Test 4.2: Small Amount Live Trading Test**
**Priority**: CRITICAL  
**Purpose**: Validate real execution with minimal risk

```rust
// File: tests/testnet/small_amount_test.rs

#[tokio::test]
async fn test_minimal_live_trading() {
    // Configure for minimal amounts
    let config = TestnetConfig {
        max_trade_size_sol: 0.01, // 1 cent USD equivalent
        max_total_exposure_sol: 0.1,
        paper_trading: false, // ACTUAL EXECUTION
        require_profit_confirmation: true,
    };
    
    let engine = create_testnet_engine(config).await;
    let initial_balance = get_wallet_balance().await?;
    
    // Run for 1 hour
    let test_duration = Duration::from_secs(3600);
    let start_time = Instant::now();
    
    while start_time.elapsed() < test_duration {
        // Process real opportunities with minimal amounts
        if let Some(opportunity) = engine.get_next_opportunity().await {
            if opportunity.profit_bps > 20 { // Only high-confidence trades
                let result = engine.execute_real_trade(opportunity).await?;
                
                if result {
                    println!("✅ Small trade executed successfully");
                }
            }
        }
        
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
    
    let final_balance = get_wallet_balance().await?;
    let net_result = final_balance - initial_balance;
    
    // Validate results
    assert!(net_result > -0.05); // Max loss 5 cents
    println!("📊 Small Amount Test Results:");
    println!("  Initial balance: {:.6} SOL", initial_balance);
    println!("  Final balance: {:.6} SOL", final_balance);
    println!("  Net result: {:.6} SOL", net_result);
}
```

### **Test 4.3: Flash Loan Integration Validation**
**Priority**: HIGH  
**Purpose**: Verify flash loan functionality in real environment

```rust
// File: tests/testnet/flash_loan_test.rs

#[tokio::test]
async fn test_flash_loan_marginfi_integration() {
    let flash_loan_manager = create_testnet_flash_loan_manager().await;
    
    // Test parameters
    let loan_amount = 1_000_000; // 0.001 SOL
    let token_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    
    // Step 1: Request flash loan quote
    let quote = flash_loan_manager
        .get_flash_loan_quote(token_mint, loan_amount)
        .await?;
    
    assert!(quote.fee_lamports > 0);
    assert!(quote.fee_lamports < loan_amount / 100); // Less than 1% fee
    
    // Step 2: Execute flash loan transaction
    let transaction_builder = flash_loan_manager
        .build_flash_loan_transaction(&quote, |borrowed_funds| async {
            // Simulate arbitrage with borrowed funds
            // In real test, this would be actual DEX swaps
            simulate_profitable_arbitrage(borrowed_funds).await
        })
        .await?;
    
    // Step 3: Submit and confirm transaction
    let signature = submit_transaction(transaction_builder).await?;
    wait_for_confirmation(&signature).await?;
    
    println!("✅ Flash loan test completed successfully: {}", signature);
}
```

---

## 📊 Success Criteria & Validation Metrics

### **Phase 1: Integration Testing**
- ✅ All unit tests pass (100% pass rate)
- ✅ Integration tests validate opportunity conversion
- ✅ Error handling tests confirm graceful failure recovery
- ✅ AI integration tests show proper strategy selection
- ✅ Configuration tests confirm real-time updates

### **Phase 2: End-to-End Testing**  
- ✅ Full pipeline test completes successfully
- ✅ Database integration shows proper data flow
- ✅ No "execution queue closed" errors
- ✅ AsyncHistoryBuffer pipeline remains stable
- ✅ All metrics endpoints respond correctly

### **Phase 3: Performance Testing**
- ✅ High-frequency test: >900/1000 opportunities processed
- ✅ Memory stability: <30% growth over extended operation  
- ✅ Database stress: >1000 ops/second sustained
- ✅ Latency targets: <1500μs average processing time
- ✅ Success rate: >15% (baseline established)

### **Phase 4: Testnet Validation**
- ✅ Small amount trading: Net positive or break-even
- ✅ Flash loan integration: Successful loan/repay cycle
- ✅ Real DEX integration: Actual swap execution
- ✅ Profit calculation accuracy: Within 5% of predicted
- ✅ Risk management: No losses exceed configured limits

---

## 🔧 Test Environment Configuration

### **Test Infrastructure**
```yaml
# File: tests/docker-compose.test.yml
version: '3.8'
services:
  postgres-test:
    image: postgres:15
    environment:
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
      POSTGRES_DB: testdb
    ports:
      - "5434:5432"
  
  kafka-test:
    image: confluentinc/cp-kafka:7.4.0
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper-test:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:29093
    ports:
      - "29093:29093"
```

### **Test Data Generation**
```rust
// File: tests/common/test_data.rs

pub fn create_mock_hotpath_opportunity() -> ArbitrageOpportunity {
    ArbitrageOpportunity {
        id: Uuid::new_v4(),
        token_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        profit_bps: 15,
        max_size_in: 50_000_000, // 50 SOL
        route_a: VenueRoute {
            venue: "Raydium-CLMM".to_string(),
            price: 100_000_000,
            liquidity: 1_000_000_000,
        },
        route_b: VenueRoute {
            venue: "Orca-Whirlpool".to_string(), 
            price: 101_500_000,
            liquidity: 800_000_000,
        },
        detected_at: chrono::Utc::now(),
        max_edge_age_ms: 1000,
    }
}

pub fn create_varied_opportunities(count: usize) -> Vec<ArbitrageOpportunity> {
    (0..count)
        .map(|i| {
            let mut opp = create_mock_hotpath_opportunity();
            opp.profit_bps = 5 + (i % 30) as u16; // 5-35 bps range
            opp.max_size_in = 10_000_000 + (i * 1_000_000) as u64; // Varied sizes
            opp
        })
        .collect()
}
```

---

## 🚨 Test Failure Handling

### **Critical Test Failures**
If any CRITICAL priority test fails:
1. **STOP** implementation immediately
2. **Analyze** root cause thoroughly  
3. **Fix** the underlying issue
4. **Re-run** all tests before proceeding

### **High Priority Test Failures**
If HIGH priority tests fail:
1. **Document** the failure and impact assessment
2. **Implement** fix or workaround
3. **Validate** fix with additional testing
4. **Consider** impact on timeline

### **Test Environment Issues**
Common issues and solutions:
- **Database connection failures**: Check port conflicts, restart Docker
- **Kafka connectivity**: Verify network configuration, check logs
- **Memory issues**: Increase Docker memory allocation
- **Timing issues**: Add appropriate delays, check for race conditions

---

## 📈 Testing Metrics Dashboard

### **Real-Time Test Monitoring**
Create dashboard at `http://localhost:3001/test-metrics` showing:
- Test pass/fail rates by category
- Performance metrics during load testing
- Memory usage trends during stability tests
- Database connection pool utilization
- Error rates and types by test phase

### **Test Report Generation**
Automated reports generated after each test phase:
- **Unit Test Report**: Coverage, pass rates, execution times
- **Integration Test Report**: End-to-end validation results  
- **Performance Test Report**: Latency, throughput, resource usage
- **Testnet Report**: Real trading results, profit/loss analysis

---

**This testing framework ensures comprehensive validation of the MEVana integration while minimizing risk through progressive validation from unit tests to small-scale live trading.**

---

*Testing procedures designed for production-critical MEV system validation as of September 12, 2025.*
