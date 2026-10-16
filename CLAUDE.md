# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Commands

### Build & Development
```bash
# Build entire workspace
cargo build --workspace

# Build specific high-performance targets
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --release -p hotpath_arb_engine

# Run tests
cargo test --workspace

# Code quality checks (run before committing)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
./scripts/ci/repo_hygiene.sh
```

### System Operations
```bash
# Start complete system (Docker infrastructure + services)
./scripts/lifecycle/start_system.sh

# Start infrastructure only
cd infrastructure && docker-compose up -d

# Run main HotPath arbitrage engine
PAPER_TRADING=true DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" X_TOKEN="<SHYFT_X_TOKEN>" \
METRICS_PORT=8088 RUST_LOG=info cargo run -p hotpath_arb_engine --bin hotpath_arb_engine

# Graceful system shutdown
./scripts/lifecycle/shutdown_system.sh
```

### Health Monitoring
```bash
# Check service health
curl -s localhost:8088/performance  # HotPath metrics
curl -s localhost:8085/metrics      # Arbitrage engine
curl -s localhost:8091/health       # Monitoring service

# Check running processes
ps aux | grep -E "(hotpath|executor|normalizer|discovery)"
docker ps
```

## Architecture Overview

MEVana is a Solana MEV platform with three distinct architectures:

### 1. HotPath Engine (Primary - Zero Latency)
- **Location**: `crates/hotpath_arb_engine/`
- **Purpose**: Direct Geyser streaming for ultra-low latency MEV detection
- **Key Features**: No Kafka dependency, direct transaction processing, sub-100ms latency
- **Run**: Use environment variables above with `hotpath_arb_engine` binary

### 2. Legacy Kafka Pipeline
- **Components**: normalizer-service → Kafka → edge-assembler → arb-engine
- **Status**: Being phased out in favor of HotPath
- **Use Case**: Historical data analysis and batch processing

### 3. Supporting Services
- **executor-service**: Trade execution with Jito bundle support
- **monitoring-service**: Metrics aggregation and API endpoints
- **wallet-vault**: Secure wallet management with Clerk auth
- **api-service**: REST API for frontend integration

## Critical Configuration

### Database
- PostgreSQL required: `postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb`
- Migrations auto-applied on service startup
- Schema: `docs/database/complete_schema.sql`

### Environment Variables (HotPath)
```bash
PAPER_TRADING=true                    # Enable paper trading mode
DATABASE_URL=...                      # PostgreSQL connection
GEYSER_ENDPOINT="https://grpc.ny.shyft.to"  # Shyft RPC endpoint
X_TOKEN="<SHYFT_X_TOKEN>"  # Auth token
METRICS_PORT=8088                     # Prometheus metrics port
MULTI_ENDPOINT_MODE=true              # Enable multiple Geyser endpoints
```

### Ports (Production)
- 8088: HotPath metrics
- 8085: Arbitrage WebSocket
- 8091: Monitoring API
- 5433: PostgreSQL
- 29092: Kafka
- 6380: Redis
- 3000: Grafana

## Development Guidelines

### File Organization
- **Dashboards**: `infrastructure/grafana/dashboards/`
- **Scripts**: `scripts/{deployment,lifecycle,monitoring,maintenance,testing}/`
- **Documentation**: `docs/{operations,deployment,architecture,strategy,reference}/`
- **Service Code**: `crates/{service-name}/`

### Code Style Requirements
- Run `cargo fmt` before commits
- Fix all `cargo clippy` warnings
- Follow patterns in `docs/development/RUST_STYLE.md`
- Use existing error handling patterns (anyhow for apps, custom errors for libs)

### Testing Requirements
- Write tests for new functionality
- Run `cargo test --workspace` before pushing
- Integration tests in `tests/` directories
- Use `PAPER_TRADING=true` for safe testing

### Git Workflow
```bash
# Check current status
git status

# Run pre-commit checks
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/ci/repo_hygiene.sh

# Create commits with descriptive messages
git add -A
git commit -m "feat: Add adaptive position sizing to HotPath engine"
```

## Important Context

### Active Development Areas
1. **HotPath Optimization**: Primary focus on reducing latency
2. **Adaptive Intelligence**: ML-based position sizing and slippage prediction
3. **Multi-Region Support**: Expanding to global Geyser endpoints
4. **Jupiter Integration**: Adding Jupiter v6 aggregator support

### Known Issues
- Kafka consumer lag during high throughput (legacy pipeline)
- PostgreSQL connection pool exhaustion under load
- Geyser reconnection needs improvement for network interruptions

### Performance Targets
- End-to-end latency: <100ms
- Transaction throughput: 10,000+ TPS
- Profit margin: 5-25 bps minimum
- System uptime: 99.9%

### Security Considerations
- Never commit `.env` files with real credentials
- Use `PAPER_TRADING=true` for all development
- Wallet keys stored in `wallet-vault` service only
- All production configs in `config/environments/`

## Quick Debugging

### Service Won't Start
```bash
# Check ports
lsof -i :8088 -i :8085 -i :5433

# Check Docker services
docker ps
docker logs postgres

# Check database connection
PGPASSWORD=<POSTGRES_PASSWORD> psql -h localhost -p 5433 -U mev -d mevdb -c "SELECT 1"
```

### No Data Flow
```bash
# Check Geyser connection
curl -s localhost:8088/performance | jq .

# Check Kafka topics (legacy)
docker exec -it kafka kafka-topics.sh --list --bootstrap-server localhost:29092

# View logs
RUST_LOG=debug cargo run -p hotpath_arb_engine
```

### Build Failures
```bash
# Clean and rebuild
cargo clean
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --workspace

# Check for missing dependencies
cargo tree -p hotpath_arb_engine
```

## References

- **Master Documentation**: `docs/DOCUMENTATION_INDEX.md`
- **Self-Hosted Deploy**: `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md`
- **Architecture**: `docs/HOT_PATH_DESIGN.md`
- **Contributing**: `CONTRIBUTING.md` and `AGENTS.md`