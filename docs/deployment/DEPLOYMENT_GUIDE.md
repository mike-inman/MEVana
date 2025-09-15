# MEVana Deployment Guide
Last updated: 2025-09-15

## Overview
This guide covers deployment of the MEVana MEV trading bot with enterprise-grade trade execution optimization.

## Components Ready for Deployment

### ✅ Core Trading Components
- **Arbitrage Engine**: Multi-venue opportunity detection with adaptive sizing
- **Executor Service**: Enterprise-grade execution with Jito bundling and flash loan optimization
- **Edge Assembler**: Real-time transaction processing and venue handling
- **Discovery Service**: Wide-open DEX discovery across all Solana venues

### ✅ Trade Execution Features
- **JitoOptimizer**: Advanced bundle optimization with tip escalation and retry logic
- **FlashLoanOptimizer**: Multi-provider flash loan aggregation with reliability scoring
- **ExecutionOrchestrator**: Unified execution coordinator with comprehensive risk management
- **Test Mode**: Full simulation capability for pre-live validation

### ✅ Infrastructure
- Kafka message broker for real-time communication
- Redis for caching and state management
- PostgreSQL for persistence
- Prometheus + Grafana for monitoring

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     MEVana Production Stack                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│  │  Geyser Stream  │ -> │ Edge Assembler  │ -> │ Arb Engine   │ │
│  │   (Shyft.to)    │    │  (4 venues)     │    │  (Adaptive)  │ │
│  └─────────────────┘    └─────────────────┘    └──────────────┘ │
│           │                       │                      │      │
│           v                       v                      v      │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│  │     Kafka       │    │     Redis       │    │ ExecutorSvc  │ │
│  │   (Messages)    │    │    (Cache)      │    │ (Jito+Flash) │ │
│  └─────────────────┘    └─────────────────┘    └──────────────┘ │
│           │                       │                      │      │
│           └───────────────────────┼──────────────────────┘      │
│                                   │                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│  │   PostgreSQL    │    │   Prometheus    │    │   Grafana    │ │
│  │ (Persistence)   │    │   (Metrics)     │    │  (Dashboards)│ │
│  └─────────────────┘    └─────────────────┘    └──────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Pre-Deployment Checklist

### Infrastructure Requirements
- [ ] Ubuntu 22.04 or similar Linux server
- [ ] Docker and Docker Compose installed
- [ ] Minimum 16GB RAM, 8 CPU cores
- [ ] SSD storage with 500GB+ available
- [ ] Stable internet connection (low latency to Solana mainnet)

### Security Requirements
- [ ] Firewall configured (only necessary ports open)
- [ ] SSH key-based authentication
- [ ] SSL/TLS certificates for external endpoints
- [ ] Wallet security (hardware wallet or secure key storage)

### Environment Configuration
```bash
# Core environment variables
export KAFKA_BROKER=localhost:29092
export REDIS_URL=redis://localhost:6379
export POSTGRES_URL=postgresql://user:pass@localhost:5432/mevana

# Shyft.to configuration
export ENDPOINT=https://grpc.ny.shyft.to
export X_TOKEN=your_shyft_api_token

# Trading configuration
export ARB_MIN_PROFIT_BPS=25
export ARB_SIZING_MODE=adaptive
export HOTPATH_LOG=true

# Execution configuration
export EXECUTOR_TEST_MODE=true  # Start in test mode!
export ARB_HTTP_PORT=8081
```

## Deployment Steps

### 1. Infrastructure Setup
```bash
# Clone repository
git clone <repository-url>
cd MEVana

# Start infrastructure services
cd infrastructure
docker-compose up -d

# Verify services are running
docker-compose ps
```

### 2. Build Services
```bash
# Build all services
cargo build --release

# Verify builds successful
ls target/release/
```

### 3. Test Mode Deployment (CRITICAL FIRST STEP)
```bash
# ALWAYS start with test mode for validation
export EXECUTOR_TEST_MODE=true

# Start services in test mode
./target/release/whirlpool-live-stream &
./target/release/arb-engine &
./target/release/executor-service &

# Run integration test
cargo run --bin test-execution-orchestrator
```

### 4. Monitor Test Mode Performance
- Watch Grafana dashboards at http://localhost:3000
- Monitor logs for execution accuracy
- Verify test trades are profitable and risk-managed
- Validate all venues are processing correctly

### 5. Live Mode Transition (Only After Successful Testing)
```bash
# Switch to live mode (ONLY after test validation)
export EXECUTOR_TEST_MODE=false

# Restart executor service for live trading
pkill executor-service
./target/release/executor-service &
```

## Configuration Files

### Docker Compose (infrastructure/docker-compose.yml)
Contains all infrastructure services with proper networking and persistence.

### Account Registry (profiles/account_registry.v1.json)
Contains validated account configurations for all trading venues.

### Prometheus Config (infrastructure/prometheus.yml)
Monitoring configuration with MEV-specific metrics.

## Monitoring & Alerting

### Key Metrics to Monitor
- **Execution Success Rate**: Target >95%
- **Profit Realization**: Actual vs estimated profit accuracy
- **Jito Bundle Success**: Bundle landing rate
- **Flash Loan Performance**: Provider reliability and costs
- **Risk Circuit Breakers**: Any triggered limits

### Critical Alerts
- Trading losses exceeding threshold
- System errors or service failures  
- Network connectivity issues
- Account balance warnings

## Risk Management

### Circuit Breakers
- Maximum loss per trade: Configurable
- Daily loss limits: Implemented
- Consecutive failure thresholds: Active
- Market volatility detection: Enabled

### Test Mode Safety
- No real funds at risk
- Full execution simulation
- Performance validation
- Risk parameter testing

## First Live Trades Protocol

### Pre-Live Checklist
- [ ] Test mode running successfully for minimum 4 hours
- [ ] All venues showing consistent activity
- [ ] Profit accuracy within acceptable range (±10%)
- [ ] No system errors or failures
- [ ] Risk parameters validated
- [ ] Circuit breakers tested
- [ ] Monitoring dashboards active
- [ ] Team ready for immediate response

### Initial Live Trading
1. Start with minimal position sizes
2. Monitor first 10 trades closely
3. Validate profit realization
4. Scale up gradually if successful
5. Keep test mode parallel for comparison

### Success Criteria for Full Trading
- First 50 live trades profitable
- Risk management working correctly
- System stability demonstrated
- Monitoring and alerts functional

## Troubleshooting

### Common Issues
- **Kafka Connection Failures**: Check broker configuration
- **Geyser Stream Drops**: Verify Shyft.to API key and network
- **Failed Trades**: Check account balances and permissions
- **Performance Issues**: Monitor system resources

### Log Locations
- Service logs: `journalctl -f -u <service-name>`
- Application logs: Via structured logging to stdout
- Trading logs: Audit trail in PostgreSQL

## Emergency Procedures

### Immediate Stop Trading
```bash
# Emergency stop - kills all trading processes
pkill arb-engine
pkill executor-service
pkill whirlpool-live-stream
```

### Circuit Breaker Override
```bash
# Force enable circuit breaker to stop new trades
curl -X POST http://localhost:8081/emergency/circuit-breaker/enable
```

## Next Steps

1. **Server Provisioning**: Set up production server environment
2. **Security Hardening**: Implement production security measures  
3. **Test Mode Validation**: Run comprehensive test mode execution
4. **Live Trading Transition**: Carefully transition to live trading
5. **Performance Optimization**: Fine-tune based on live performance data

## Contact & Support

For deployment support and trading issues:
- Monitor system logs and Grafana dashboards
- Check circuit breakers and risk metrics
- Validate account balances and permissions
- Review recent configuration changes
