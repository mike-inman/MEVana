# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MEVana is a comprehensive Solana AI trading platform designed to capture MEV (Maximal Extractable Value) opportunities through real-time blockchain data streaming, intelligent transaction analysis, and automated arbitrage execution.

## Architecture

The platform consists of several key components organized in a Cargo workspace:

### Core Services
- **Arbitrage Engine** (`crates/arb-engine`) - MEV opportunity detection and execution with adaptive sizing
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

## Monitoring & Introspection

### HTTP Endpoints
- `/metrics` - Prometheus metrics (all services)
- `/ai/config` - Current configuration (arb-engine)
- `/ai/sizing` - Adaptive sizing state and history (arb-engine)
- `/token/{mint}` - Token metadata lookup (executor-service)

### Docker Infrastructure Ports
- **Kafka UI**: http://localhost:8080
- **Redis UI**: http://localhost:8083 (admin/admin123)
- **Grafana**: http://localhost:3000 (admin/admin123)
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