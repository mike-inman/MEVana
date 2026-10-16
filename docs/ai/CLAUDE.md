# CLAUDE.md
Last updated: 2025-09-15

Start Here — Required Reading (confirm understanding before changes)
- AGENTS.md — Repo rules and file placement
- docs/ai/ONBOARDING_AI.md — 2‑minute orientation
- docs/DOCUMENTATION_INDEX.md — Critical runbooks and quick links
- docs/operations/CRITICAL_PRODUCTION_BLOCKERS.md — Safety constraints
- docs/development/RUST_STYLE.md — Rust naming, errors, async, logging
- scripts/ci/repo_hygiene.sh — What CI enforces (skim)

Run locally before proposing patches
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- bash scripts/ci/repo_hygiene.sh

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🔄 Current Session Context
<!-- Update this section at the start/end of each work session -->

**Last Updated:** 2025-09-12
**Current Focus:** CRITICAL DATABASE CONNECTION CONFLICTS RESOLVED
**Active Branch:** main

### ⚠️ CRITICAL SERVICE MANAGEMENT ISSUE - RESOLVED ⚠️

**FOR FUTURE CLAUDE HELPERS: READ THIS FIRST**

**Problem:** Multiple Claude Code sessions were spinning up services "helter skelter" causing:
- ❌ **Database connection conflicts**: Port 5432 (local Postgres.app) vs Port 5433 (Docker)
- ❌ **AsyncHistoryBuffer pipeline failures**: Database writes stopped working
- ❌ **Service port chaos**: Multiple hotpath_arb_engine instances with different configs
- ❌ **Data corruption**: Database counts jumping between 322 ↔ 6010 opportunities
- ❌ **Inconsistent environment variables**: Different DATABASE_URL values across processes
- ❌ **GEYSER ENDPOINT MONITORING**: Multiple instances trigger abuse detection, stuck at old slots

**Root Causes:**
1. Hardcoded database connections in archived code
2. **CRITICAL**: Background processes from testing cause endpoint throttling/blocking

**Solution Implemented:**
1. ✅ **Enhanced cleanup script**: `scripts/lifecycle/start_clean.sh` 
2. ✅ **Comprehensive process killing**: cargo run, pkill, lsof port cleanup
3. ✅ **Single instance policy**: Prevents Geyser endpoint monitoring issues
4. ✅ **Multi-endpoint distribution**: NY/Virginia/Frankfurt to avoid congestion

### 🚨 MANDATORY FOR ALL FUTURE SESSIONS 🚨

```bash
# ALWAYS use this clean startup - KILLS ALL BACKGROUND PROCESSES!
./scripts/lifecycle/start_clean.sh
```

**Critical Rules:**
- ❌ **NEVER start multiple hotpath_arb_engine instances** (triggers endpoint monitoring)
- ❌ **NEVER use background processes** without cleanup first
- ❌ **NEVER start services manually** without using the clean startup script
- ❌ Connect to port 5432 (local Postgres.app)
- ❌ **NEVER use `docker-compose` (hyphen)** - User has mentioned multiple times
- ✅ **ALWAYS use comprehensive cleanup** before any service startup
- ✅ **ALWAYS verify single instance** running after startup
- ✅ **ALWAYS use `docker compose` (space)** - Modern Docker command format

### 🛠️ **CONFLICT DETECTION TOOLS**

```bash
# Check for multiple instance conflicts (run anytime)
./scripts/lifecycle/check_conflicts.sh

# Clean startup with built-in conflict detection
./scripts/lifecycle/start_clean.sh

# Full system shutdown
./scripts/lifecycle/shutdown_system.sh
```

**The enhanced startup script now includes:**
- ✅ **Comprehensive process cleanup** (cargo, pkill, lsof)
- ✅ **Pre-startup conflict detection** (aborts if conflicts found)
- ✅ **Port occupation checks** (prevents dual endpoint access)
- ✅ **Single instance enforcement** (protects against Geyser monitoring)

### Recent Achievements
- ✅ **Smart Filter Real-Time Configuration System** - COMPLETED
- ✅ **Smart Filter UI Control Panel** - COMPLETED  
- ✅ **CRITICAL: Database connection conflicts** - RESOLVED
- ✅ **Service management chaos** - FIXED with centralized startup
- ✅ **AsyncHistoryBuffer pipeline** - RESTORED

### Current System State
- ✅ **Single Database**: Docker PostgreSQL port 5433 only
- ✅ **System Orchestrator**: Running with proper configuration
- ✅ **Clean Startup Script**: `./scripts/start_clean.sh`
- ⚠️ **AsyncHistoryBuffer**: Still investigating write pipeline issues

### ✅ RESOLVED: Memory Leak and Database Write Issues - 2025-09-12

**CRITICAL DISCOVERY**: The "memory leak" degradation pattern was NOT a traditional memory leak but a **configuration-induced queue management failure cascade**.

#### Root Cause Analysis
1. **Primary Issue**: `PAPER_TRADING=false` triggered safety shutdown → execution queue closed
2. **Memory Pressure Cascade**: 1000+ failed queue operations/minute → error handling overhead → apparent "memory leak" 
3. **Queue Blocking**: Every arbitrage opportunity tried to send to closed execution queue → error: "Failed to trigger lethal execution: Execution queue closed"

#### The Real "Memory Leak" Pattern
- **AI Q-table HashMap growth**: Fixed with 10,000 entry limits + LRU eviction
- **O(n) Vec operations**: Fixed by changing Vec::remove(0) to VecDeque::pop_front() for O(1) performance
- **Async task accumulation**: Fixed by preventing queue closure with proper configuration

#### ✅ SOLUTION IMPLEMENTED
**Use `PAPER_TRADING=true` for arbitrage engines:**
```bash
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" RUST_LOG=info GEYSER_ENDPOINT="https://grpc.ny.shyft.to" X_TOKEN="<SHYFT_X_TOKEN>" ORDERBOOK_HISTORY_ENABLED=true PAPER_TRADING=true cargo run -p hotpath_arb_engine
```

#### Results After Fix
- ✅ **Sustained opportunity detection** (1000+/minute without degradation)
- ✅ **Execution queue stays open** (no more "Execution queue closed" errors)
- ✅ **AI learning system functioning** (continuous profit learning without memory issues)  
- ✅ **Database records flowing** through AsyncHistoryBuffer pipeline
- ✅ **No 7-minute degradation pattern** (system runs continuously)

#### Code Changes Made
- **ai_strategy_engine.rs:348**: Added Q-table HashMap bounds (10,000 entries)
- **ai_strategy_engine.rs:380**: Added success_patterns HashMap bounds (5,000 entries)
- **main.rs**: Changed Vec to VecDeque with capacity limits for O(1) operations
- **smart_filter.rs**: Fixed O(n) operations in VenueStats collections

#### For Future Troubleshooting
- **ALWAYS use PAPER_TRADING=true** until production deployment is explicitly approved
- **Monitor execution queue status** - if closed, check for safety violations
- **Check for "Failed to trigger lethal execution" errors** - indicates configuration issue
- **Database write stops**: Usually queue management, not AsyncHistoryBuffer itself

### Current System Status  
- ✅ **Memory leaks**: RESOLVED (configuration + HashMap bounds + O(1) operations)
- ✅ **Database writes**: STABLE (AsyncHistoryBuffer working with proper queue management)
- ✅ **Queue management**: FIXED (PAPER_TRADING=true prevents safety shutdown)

### Next Priorities
1. Advanced configuration management features (A/B testing, presets)
2. WebSocket integration for true real-time updates
3. Configuration impact analytics and machine learning insights
4. Performance optimization and monitoring enhancements

## 📚 **DOCUMENTATION REFERENCE**

> **⚠️ CRITICAL** - Always reference the master documentation index first

**📋 [DOCUMENTATION_INDEX.md](./DOCUMENTATION_INDEX.md)** - Master reference for all docs

### **Essential Operational Documents**
- **[STARTUP_PROGRESS.md](./STARTUP_PROGRESS.md)** - System startup procedures
- **[SHUTDOWN_PROCEDURES.md](./SHUTDOWN_PROCEDURES.md)** - Production shutdown guide  
- **[PORT_MAPPING.md](./PORT_MAPPING.md)** - Service configuration
- **[PRODUCTION_LESSONS_LEARNED.md](./PRODUCTION_LESSONS_LEARNED.md)** - Critical findings
- **[SMART_FILTER_IMPLEMENTATION_PLAN.md](./docs/SMART_FILTER_IMPLEMENTATION_PLAN.md)** - Real-time configuration system

### Performance Baseline
- HotPath detection latency: ~400-600μs (Sub-millisecond)
- Arbitrage opportunities detected: 291+ (Active detection)
- Execution success rate: 18.28%
- Configuration update latency: <50ms (Event-driven)
- Atomic cache access: ~1-2ns (Zero-latency hotpath)

### Key Decisions/Notes
- Using `cargo clippy -- -D warnings` locally to match CI
- Web interface planned: React + WebSockets + PostgreSQL encrypted vault
- Wallet tiers: Hot (trading) → Warm (reserve) → Cold (storage)

---

## Project Overview

MEVana is a comprehensive Solana AI trading platform designed to capture MEV (Maximal Extractable Value) opportunities through real-time blockchain data streaming, intelligent transaction analysis, and automated arbitrage execution.

## Architecture

The platform consists of several key components organized in a Cargo workspace:

### Core Services
- **Arbitrage Engine** (`crates/arb-engine`) - MEV opportunity detection and execution with adaptive sizing
- **HotPath Arbitrage Engine** (`crates/hotpath_arb_engine`) - Zero-latency MEV detection with AI optimization and enterprise wallet management
- **Normalizer Service** (`crates/normalizer-service`) - IDL-based code generation and transaction parsing  
- **Edge Assembler** (`crates/edge-assembler`) - Real-time transaction processing and venue handling
- **Executor Service** (`executor-service`) - Bundle building, execution, and wallet management

### Supporting Crates
- **Shared Core** (`crates/shared-core`) - Common utilities and types
- **Whirlpool Core** (`crates/whirlpool-core`) - Orca Whirlpool protocol integration
- **Token Registry Service** (`crates/token-registry-service`) - Token metadata management
- **Discovery Service** (`crates/discovery-service`) - Service discovery and registration

### Infrastructure
- **Streaming Services** - Real-time DEX transaction monitoring (Orca, Raydium, Meteora)
- **Message Broker** - Kafka for inter-service communication
- **Data Storage** - Redis for caching, PostgreSQL for persistence
- **Monitoring** - Prometheus metrics, Grafana dashboards

## Development Commands

### Building
```bash
# Build the entire workspace
cargo build --workspace

# Build specific package
cargo build -p arb-engine
cargo build -p executor-service
```

### Testing
```bash
# Run all tests
cargo test --workspace

# Run tests for specific package
cargo test -p shared-core
```

### Code Quality
```bash
# Check code with Clippy
cargo clippy --workspace

# Format code
cargo fmt --all
```

### Running Services

#### Infrastructure Setup
```bash
# Start infrastructure services
cd infrastructure
docker-compose up -d

# Create Docker network (first time only)
docker network create mevana_net
```

#### Core Services
```bash
# Start normalizer service
cargo run -p normalizer-service

# Start edge assembler
cargo run -p edge-assembler

# Start arbitrage engine
cargo run -p arb-engine

# Start executor service
cargo run -p executor-service

# Start hotpath arbitrage engine (enterprise MEV system)
GEYSER_ENDPOINT=https://grpc.ny.shyft.to X_TOKEN=your_token METRICS_PORT=8088 RUST_LOG=info cargo run -p hotpath_arb_engine --bin hotpath_arb_engine
```

#### Streaming Services
```bash
# Start DEX streaming (various options)
cargo run -p dex-stream-integrated
cargo run -p dex-stream-universal
```

### Account Registry Management

The executor service uses versioned account registries in `profiles/`:

```bash
# Validate account registry files
cargo run -p executor-service --bin account-registry-validate -- --strict profiles/account_registry.v1.json

# Generate diff between registry versions  
cargo run -p executor-service --bin account-registry-validate -- --diff --patch-json diff.json profiles/account_registry.v1.json profiles/account_registry.v2.json
```

## Key Environment Variables

### Arbitrage Engine
- `ARB_SIZING_MODE` - `adaptive` or `fixed` sizing strategy
- `ARB_ADAPT_HISTORY` - Number of recent profit samples for median calculation
- `ARB_ADAPT_MIN_MULTIPLIER` / `ARB_ADAPT_MAX_MULTIPLIER` - Bounds on adaptive size multiplier
- `ARB_MIN_PROFIT_BPS` - Minimum net profit threshold after adjustments
- `ARB_MAX_SLIPPAGE_BPS` - Maximum allowed slippage before rejecting opportunity

### Token Registry
- `TOKEN_REGISTRY_PORT` - HTTP port for token metadata endpoint (default: 9898)

## HotPath Arbitrage Engine

The HotPath Arbitrage Engine is our flagship enterprise MEV detection and execution system with zero-latency performance and institutional-grade security.

### Key Features

- **Zero-Latency Detection**: Sub-millisecond arbitrage opportunity detection (~400-600μs)
- **Multi-Strategy Arbitrage**: Cross-venue, impact-based, and concentrated liquidity strategies
- **AI-Powered Optimization**: Neural network-based parameter tuning and profit maximization
- **Enterprise Wallet Management**: Multi-tier security (Hot/Warm/Cold wallets) with HSM integration
- **Live Trading Integration**: Jito bundle optimization and flash loan aggregation
- **Comprehensive Analytics**: Real-time performance monitoring and risk assessment

### HTTP Endpoints (Port 8088)

- `/metrics` - Prometheus metrics
- `/performance` - Comprehensive performance analytics
- `/ai/config` - AI optimization configuration
- `/ai/prediction` - Current AI predictions
- `/ai/optimization` - Optimization recommendations
- `/wallet/security` - Wallet security dashboard
- `/wallet/generate` - Generate new wallet (POST)
- `/wallet/list` - List all wallets
- `/wallet/audit` - Full security audit report

### Environment Variables

- `GEYSER_ENDPOINT` - Solana Geyser gRPC endpoint
- `X_TOKEN` - Authentication token for Geyser service
- `METRICS_PORT` - HTTP metrics server port (default: 8088)
- `RUST_LOG` - Log level configuration
- ⚠️ **`PAPER_TRADING=true`** - **CRITICAL**: Must be set to prevent safety shutdown and execution queue closure

### Wallet Management

The system automatically generates demo wallets on startup:
- 2 Hot wallets (0.1 SOL limit each) - Active trading
- 1 Warm wallet (10 SOL limit) - Reserve funds
- 1 Cold wallet (unlimited) - Long-term storage

### Performance Metrics

Real-time monitoring of:
- Arbitrage opportunities detected per minute
- Execution success rates
- Profit capture by venue and strategy
- System latency (P50, P95, P99)
- AI optimization effectiveness
- Wallet security status

## Monitoring & Introspection

### HTTP Endpoints
- `/metrics` - Prometheus metrics (all services)
- `/ai/config` - Current configuration (arb-engine)
- `/ai/sizing` - Adaptive sizing state and history (arb-engine)
- `/token/{mint}` - Token metadata lookup (executor-service)

### Docker Infrastructure Ports
- **Kafka UI**: http://localhost:8080
- **Redis UI**: http://localhost:8083 (admin/<ADMIN_PASSWORD>)
- **Grafana**: http://localhost:3000 (admin/<ADMIN_PASSWORD>)
- **Prometheus**: http://localhost:9090
- **Adminer (PostgreSQL)**: http://localhost:8082

## Workspace Structure

The project uses Cargo workspace with selective default members. Most development should focus on these key packages:
- `normalizer-generated` - Generated code from IDLs
- `normalizer-service` - Transaction parsing service
- `edge-assembler` - Real-time transaction processing

Archived components are excluded from workspace builds and located in `_archive/` or listed in workspace exclusions.

## Development Notes

- The platform uses extensive Kafka messaging between services
- All services include Prometheus metrics for monitoring
- Docker Compose provides complete infrastructure stack
- Account registries use JSON Schema validation with decimal decrease protection
- Adaptive arbitrage sizing includes crash-safe state persistence
- Token metadata is cached in Redis with HTTP API access
