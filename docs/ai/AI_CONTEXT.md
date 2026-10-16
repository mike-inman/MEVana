# MEVana AI Context Guide

> **For AI Assistants**: This document contains everything you need to effectively work on MEVana without requiring lengthy explanations from the user.

**Last Updated**: 2025-01-06
**Status**: Production-ready, actively trading
**Architecture**: HotPath zero-latency MEV detection

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [System Overview](#system-overview)
3. [Architecture](#architecture)
4. [Environment Setup](#environment-setup)
5. [Common Tasks](#common-tasks)
6. [Troubleshooting](#troubleshooting)
7. [Deployment](#deployment)

---

## Quick Start

### Build and Run

```bash
# Build workspace
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --workspace

# Start infrastructure
cd infrastructure && docker-compose up -d

# Run HotPath engine (paper trading)
PAPER_TRADING=true \
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="<SHYFT_X_TOKEN>" \
METRICS_PORT=8088 \
RUST_LOG=info \
cargo run -p hotpath_arb_engine --bin hotpath_arb_engine
```

### Health Checks

```bash
curl localhost:8088/performance  # HotPath metrics
curl localhost:8085/metrics      # Arbitrage engine (legacy)
curl localhost:8091/health       # Monitoring service
```

---

## System Overview

**MEVana** is a Solana MEV (Maximal Extractable Value) arbitrage platform designed for ultra-low latency trade execution.

### Key Goals

- **Speed**: Sub-100ms end-to-end latency from Geyser stream to trade execution
- **Profitability**: Minimum 5-25 bps profit margin per arbitrage
- **Reliability**: 99.9% uptime with automatic failover
- **Security**: Multi-tier wallet management with enterprise-grade protection

### Tech Stack

- **Language**: Rust (for performance)
- **Blockchain**: Solana
- **Data Ingestion**: Yellowstone Geyser (direct RPC streaming)
- **Execution**: Jito bundles for MEV protection
- **Infrastructure**: Docker, PostgreSQL, Redis, Kafka (legacy), Prometheus, Grafana

---

## Architecture

### Three Architectures in MEVana

#### 1. **HotPath Engine** (Primary - Zero Latency)

**Location**: `crates/hotpath_arb_engine/`

**Purpose**: Direct Geyser streaming for ultra-low latency MEV detection

**Data Flow**:
```
Geyser Stream → HotPath → FastEngine → SignalQueue → Executor → Jito
```

**Features**:
- No Kafka dependency
- Direct transaction processing
- Sub-100ms latency
- Adaptive position sizing with AI strategy engine

**Run**:
```bash
PAPER_TRADING=true \
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="<SHYFT_X_TOKEN>" \
METRICS_PORT=8088 \
RUST_LOG=info \
cargo run -p hotpath_arb_engine
```

#### 2. **Legacy Kafka Pipeline** (Being Phased Out)

**Components**:
```
Geyser → dex-stream → Kafka → normalizer → edge-assembler → arb-engine
```

**Status**: Maintained for historical data analysis and batch processing

**Use Case**: Analytics, backtesting, non-critical arbitrage detection

#### 3. **Supporting Services**

- **executor-service**: Trade execution with Jito bundle support
- **monitoring-service**: Metrics aggregation and API endpoints
- **wallet-vault**: Secure wallet management with Clerk authentication
- **api-service**: REST API for frontend integration
- **discovery-service**: Token discovery and metadata
- **normalizer-service**: Swap normalization across DEXes

---

## Environment Setup

### Required Services (Docker)

**Location**: `infrastructure/docker-compose.yml`

```bash
cd infrastructure
docker-compose up -d
```

**Services Started**:
- PostgreSQL (port 5433)
- Redis (port 6380)
- Kafka + Zookeeper (ports 29092, 2181)
- Prometheus (port 9090)
- Grafana (port 3000)
- Adminer (port 8080)

### Database

**Connection String**: `postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb`

**Migrations**: Auto-applied on service startup via SQLx

**Schema**: See `docs/database/complete_schema.sql`

### Environment Variables

**Core Variables** (HotPath):
```bash
PAPER_TRADING=true                         # Enable paper trading (safe testing)
DATABASE_URL=postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb
GEYSER_ENDPOINT=https://grpc.ny.shyft.to  # Shyft RPC endpoint
X_TOKEN=<SHYFT_X_TOKEN>  # Auth token
METRICS_PORT=8088                          # Prometheus scrape endpoint
MULTI_ENDPOINT_MODE=true                   # Enable failover endpoints
RUST_LOG=info                              # Log level
```

**Optional Variables**:
```bash
ENABLE_JUPITER=true                        # Enable Jupiter v6 aggregator
ARB_MIN_PROFIT_BPS=25                      # Minimum profit (basis points)
ARB_MAX_SLIPPAGE_BPS=50                    # Max slippage tolerance
ORDERBOOK_HISTORY_ENABLED=true             # Store orderbook snapshots
```

### Production Ports

| Service | Port | Purpose |
|---------|------|---------|
| HotPath Metrics | 8088 | Prometheus metrics |
| Arbitrage WebSocket | 8085 | Real-time arb feed |
| Monitoring API | 8091 | Health/metrics API |
| PostgreSQL | 5433 | Database |
| Kafka | 29092 | Legacy event bus |
| Redis | 6380 | Caching |
| Grafana | 3000 | Dashboards |

---

## Common Tasks

### Code Quality (Run Before Commits)

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/ci/repo_hygiene.sh
```

### Testing Paper Trading

```bash
# Start infrastructure
cd infrastructure && docker-compose up -d

# Run HotPath with paper trading
PAPER_TRADING=true \
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="<SHYFT_X_TOKEN>" \
METRICS_PORT=8088 \
RUST_LOG=info \
cargo run -p hotpath_arb_engine

# Check metrics
curl -s localhost:8088/performance | jq .
```

### Debugging

**Service won't start**:
```bash
# Check port conflicts
lsof -i :8088 -i :8085 -i :5433

# Check Docker services
docker ps
docker logs postgres

# Test database connection
PGPASSWORD=<POSTGRES_PASSWORD> psql -h localhost -p 5433 -U mev -d mevdb -c "SELECT 1"
```

**No data flow**:
```bash
# Check Geyser connection
curl -s localhost:8088/performance | jq .

# View detailed logs
RUST_LOG=debug cargo run -p hotpath_arb_engine
```

**Build failures**:
```bash
# Clean and rebuild
cargo clean
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --workspace

# Check dependencies
cargo tree -p hotpath_arb_engine
```

### Adding New Features

1. **Create feature branch**: `git checkout -b feature/my-feature`
2. **Write tests**: Add tests to `tests/` directory
3. **Implement feature**: Follow patterns in existing code
4. **Run quality checks**: See "Code Quality" above
5. **Test end-to-end**: Use `PAPER_TRADING=true`
6. **Document**: Update relevant docs in `docs/`
7. **Commit**: Use conventional commits (`feat:`, `fix:`, `docs:`, etc.)

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Compile errors with SQLx | Missing DATABASE_URL | Set `DATABASE_URL` before `cargo` commands |
| Port already in use | Previous instance running | `pkill hotpath_arb_engine` or `lsof -i :8088` |
| No arbitrage opportunities | Wrong Geyser endpoint | Check `GEYSER_ENDPOINT` is correct |
| Database connection refused | PostgreSQL not running | `docker ps` and check postgres container |
| High latency | Network congestion | Switch to multi-endpoint mode |

### Debug Checklist

1. ✅ Docker services running: `docker ps`
2. ✅ Database accessible: `PGPASSWORD=<POSTGRES_PASSWORD> psql -h localhost -p 5433 -U mev -d mevdb -c "SELECT 1"`
3. ✅ No port conflicts: `lsof -i :8088`
4. ✅ Environment variables set: `echo $DATABASE_URL`
5. ✅ Code compiles: `cargo check --workspace`
6. ✅ Metrics endpoint live: `curl localhost:8088/performance`

---

## Deployment

### Self-Hosted Deployment

**Full Guide**: See `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md`

**Quick Deploy**:
```bash
./scripts/deployment/deploy-services.sh
```

### Production Configuration

1. **Set up production database**: Change password from defaults
2. **Configure Geyser endpoints**: Use production RPC endpoints
3. **Set wallet keys**: Use wallet-vault service for secure storage
4. **Enable monitoring**: Configure Prometheus + Grafana alerts
5. **Test with PAPER_TRADING**: Verify before enabling live trading
6. **Remove PAPER_TRADING**: Enable live execution

### Monitoring

**Grafana Dashboards**: `http://localhost:3000`

**Key Metrics**:
- End-to-end latency
- Arbitrage opportunities detected
- Successful/failed executions
- Profit per trade (BPS)
- Geyser connection health
- Database connection pool usage

---

## File Organization

### Project Structure

```
MEVana/
├── crates/
│   ├── hotpath_arb_engine/      # Main HotPath engine
│   ├── arb-engine/               # Legacy arbitrage engine
│   ├── executor-service/         # Trade execution
│   ├── monitoring-service/       # Metrics aggregation
│   ├── wallet-vault/             # Secure wallet management
│   ├── api-service/              # REST API
│   ├── normalizer-service/       # Swap normalization
│   ├── discovery-service/        # Token discovery
│   ├── edge-assembler/           # Edge detection (legacy)
│   └── shared-core/              # Shared utilities
├── infrastructure/
│   ├── docker-compose.yml        # Local infrastructure
│   ├── grafana/dashboards/       # Grafana dashboards
│   └── prometheus/               # Prometheus config
├── scripts/
│   ├── deployment/               # Deploy scripts
│   ├── lifecycle/                # Start/stop scripts
│   ├── monitoring/               # Monitoring tools
│   └── testing/                  # Test utilities
├── docs/
│   ├── operations/               # Runbooks
│   ├── deployment/               # Deploy guides
│   ├── architecture/             # Architecture docs
│   ├── ai/                       # AI assistant guides
│   └── reference/                # API references
└── CLAUDE.md                     # Claude Code instructions
```

### Key Files

- **CLAUDE.md**: Essential commands and guidance for Claude Code
- **docs/DOCUMENTATION_INDEX.md**: Master documentation index
- **docs/ai/HANDOFF.md**: AI handoff protocol
- **docs/HOT_PATH_DESIGN.md**: HotPath architecture details
- **infrastructure/docker-compose.yml**: Local infrastructure setup
- **Cargo.toml**: Workspace configuration

---

## Development Guidelines

### Code Style

- **Formatting**: Use `cargo fmt --all` (enforced)
- **Linting**: Fix all `cargo clippy` warnings (enforced)
- **Error Handling**: Use `anyhow` for applications, custom errors for libraries
- **Async**: Use Tokio for async runtime
- **Testing**: Write tests for new functionality

### Git Workflow

1. **Check status**: `git status`
2. **Run checks**: `cargo fmt`, `cargo clippy`, `cargo test`
3. **Commit**: Use conventional commits
   - `feat: Add adaptive position sizing`
   - `fix: Resolve borrow checker error in network.rs`
   - `docs: Update AI context guide`
4. **Push**: `git push`

### Conventional Commits

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation only
- `refactor:` Code refactor
- `test:` Add/update tests
- `chore:` Tooling/build changes

---

## Active Development Areas

1. **HotPath Optimization**: Primary focus on reducing latency
2. **Adaptive Intelligence**: ML-based position sizing and slippage prediction
3. **Multi-Region Support**: Expanding to global Geyser endpoints
4. **Jupiter Integration**: Adding Jupiter v6 aggregator support

---

## Known Issues

- Kafka consumer lag during high throughput (legacy pipeline only)
- PostgreSQL connection pool exhaustion under sustained load (mitigation: increase pool size)
- Geyser reconnection needs improvement for network interruptions (retry logic in place)

---

## Security Considerations

⚠️ **Critical Security Rules**:

1. **Never commit secrets**: Use `.env` files (gitignored)
2. **Always use PAPER_TRADING=true** for development/testing
3. **Wallet keys**: Store in `wallet-vault` service only, never in code
4. **Production configs**: Use `config/environments/` with secrets management
5. **Database passwords**: Change defaults for production

---

## Performance Targets

- **End-to-end latency**: <100ms (Geyser → Execution)
- **Transaction throughput**: 10,000+ TPS
- **Profit margin**: 5-25 bps minimum
- **System uptime**: 99.9%
- **Success rate**: >80% of viable arbitrage opportunities executed

---

## Additional Resources

- **Master Documentation**: `docs/DOCUMENTATION_INDEX.md`
- **Self-Hosted Deploy**: `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md`
- **HotPath Architecture**: `docs/HOT_PATH_DESIGN.md`
- **Contributing**: `CONTRIBUTING.md` and `AGENTS.md`
- **AI Handoff Protocol**: `docs/ai/HANDOFF.md`

---

## Quick Reference Commands

```bash
# Build
cargo build --workspace
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --release -p hotpath_arb_engine

# Test
cargo test --workspace
PAPER_TRADING=true cargo run -p hotpath_arb_engine

# Code Quality
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
./scripts/ci/repo_hygiene.sh

# Infrastructure
cd infrastructure && docker-compose up -d           # Start
docker-compose down                                  # Stop
docker ps                                            # Check status

# Deployment
./scripts/lifecycle/start_system.sh                  # Start all services
./scripts/lifecycle/shutdown_system.sh               # Graceful shutdown

# Monitoring
curl localhost:8088/performance                      # HotPath metrics
curl localhost:8091/health                           # Monitoring health
docker logs -f postgres                              # View logs
```

---

## Questions?

If you need more context:
1. Check `CLAUDE.md` for quick commands
2. See `docs/DOCUMENTATION_INDEX.md` for detailed docs
3. Review `docs/ai/HANDOFF.md` for AI collaboration protocol
4. Ask the user - they built this system!

---

**End of AI Context Guide**
