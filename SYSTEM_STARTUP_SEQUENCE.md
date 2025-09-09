# MEVana System Startup Sequence

## Overview

MEVana now includes a comprehensive **System Orchestrator** (`crates/system-orchestrator`) that manages the startup sequence of all required services. This addresses the previous gap where there was no single crate managing the coordinated startup of the entire system.

## System Orchestrator Features

### 🎯 Core Capabilities
- **Tiered Service Management**: Services organized into logical tiers (Infrastructure → Core → Supporting → HotPath → Monitoring)
- **Dependency Resolution**: Automatic dependency graph resolution and startup ordering
- **Health Monitoring**: Continuous health checking with automatic restart of failed critical services
- **Graceful Shutdown**: Coordinated shutdown in reverse tier order
- **Metrics & Monitoring**: Prometheus metrics and HTTP endpoints for system observability

### 📊 HTTP Endpoints
- `http://localhost:9999/metrics` - Prometheus metrics
- `http://localhost:9999/health` - Health check endpoint  
- `http://localhost:9999/status` - System status information

## Service Tier Architecture

### Tier 0: Infrastructure Services
**Purpose**: Foundation services that all other components depend on
- `infrastructure` - Docker Compose services (Kafka, Redis, PostgreSQL, Prometheus, Grafana)
- **Startup Time**: ~2 minutes (includes Docker container initialization)
- **Health Checks**: HTTP endpoints + TCP connectivity
- **Critical**: Yes - System cannot function without these

### Tier 1: Core Services  
**Purpose**: Essential trading pipeline components
1. `normalizer-service` (Port: 9901) - IDL-based transaction parsing
2. `edge-assembler` (Port: 9902) - Real-time transaction processing  
3. `arb-engine` (Port: 9903) - Arbitrage opportunity detection
4. `executor-service` (Port: 9904) - Trade execution and bundle building

**Startup Order**: Sequential with dependency resolution
**Health Checks**: HTTP `/health` endpoints
**Critical**: Yes - Core trading functionality

### Tier 2: Supporting Services
**Purpose**: Enhancement and utility services
- `discovery-service` (Port: 9102) - Service discovery and AI-ready program analysis
- `token-registry-service` (Port: 9898) - Token metadata management

**Startup**: Parallel execution
**Critical**: No - System can operate with degraded functionality

### Tier 3: HotPath Services
**Purpose**: Ultra-high-frequency trading optimization
- `hotpath-arb-engine` (Port: 8088) - Zero-latency MEV detection (~400μs)

**Dependencies**: Requires stable core system
**Health Checks**: Performance metrics endpoint
**Critical**: Yes - Primary profit generation

### Tier 4: Monitoring Services  
**Purpose**: System observability and performance tracking
- `monitoring-service` (Port: 9100) - Performance monitoring and APIs

**Startup**: After all other tiers are stable
**Critical**: No - Operational visibility only

## Startup Sequence Flow

### Phase 1: Infrastructure Initialization (0-120s)
```bash
# Start Docker infrastructure
docker-compose -f infrastructure/docker-compose.yml up -d

# Health checks for:
# - Kafka (localhost:9092)
# - Redis (localhost:6379) 
# - PostgreSQL (localhost:5432)
# - Prometheus (localhost:9090)

# Wait time: 30s + health verification
```

### Phase 2: Core Services Sequential Startup (120-240s)
```bash
# 1. Normalizer Service
RUST_LOG=info METRICS_PORT=9901 cargo run -p normalizer-service
# Health: http://localhost:9901/health
# Wait: Until healthy + 10s

# 2. Edge Assembler  
RUST_LOG=info METRICS_PORT=9902 cargo run -p edge-assembler
# Depends on: normalizer-service
# Wait: Until healthy + 5s

# 3. Arbitrage Engine
RUST_LOG=info METRICS_PORT=9903 ARB_SIZING_MODE=adaptive cargo run -p arb-engine
# Depends on: edge-assembler  
# Wait: Until healthy + 5s

# 4. Executor Service
RUST_LOG=info METRICS_PORT=9904 cargo run -p executor-service
# Depends on: arb-engine
# Wait: Until healthy + 5s
```

### Phase 3: Supporting Services Parallel Startup (240-285s)
```bash
# Discovery Service & Token Registry start in parallel
RUST_LOG=info DISCOVERY_METRICS_PORT=9102 cargo run -p discovery-service &
RUST_LOG=info TOKEN_REGISTRY_PORT=9898 cargo run -p token-registry-service &

# Health checks: 45s timeout each
```

### Phase 4: HotPath Services (285-300s)
```bash
# HotPath Arbitrage Engine (requires Geyser credentials)
RUST_LOG=info METRICS_PORT=8088 \
GEYSER_ENDPOINT=https://grpc.ny.shyft.to \
X_TOKEN=your_token_here \
cargo run -p hotpath_arb_engine --bin hotpath_arb_engine

# Health: http://localhost:8088/performance
# Wait: Until healthy + 15s
```

### Phase 5: Monitoring Services (300-310s)
```bash
# Monitoring Service (final tier)
RUST_LOG=info METRICS_PORT=9100 cargo run -p monitoring-service

# System startup complete - ready for trading!
```

## Usage Instructions

### Option 1: Automated Orchestrator (Recommended)
```bash
# Single command to start entire MEVana system
cargo run -p system-orchestrator

# Monitor startup progress via logs
# System will be ready when you see: "🎉 MEVana system is ready for arbitrage operations!"
```

### Option 2: Manual Sequential Startup
```bash
# Follow the 5-phase sequence above manually
# Useful for debugging or selective service startup
```

### Option 3: Custom Configuration
```bash
# Create custom orchestrator-config.toml
# Modify service configurations, health checks, timeouts
# Run: cargo run -p system-orchestrator
```

## Health Monitoring & Recovery

### Automatic Recovery Features
- **Process Monitoring**: Detects crashed processes every 10 seconds
- **Health Checking**: Verifies service health every 30 seconds  
- **Auto-Restart**: Critical services restart automatically (max 3 attempts)
- **System Restart**: If 2+ critical services crash, full system restart

### Manual Health Checks
```bash
# Check individual service health
curl http://localhost:9901/health  # Normalizer
curl http://localhost:9902/health  # Edge Assembler  
curl http://localhost:9903/health  # Arbitrage Engine
curl http://localhost:8088/performance  # HotPath Engine

# System-wide metrics
curl http://localhost:9999/metrics
```

## Troubleshooting

### Common Issues

#### Infrastructure Services Not Starting
```bash
# Check Docker services
docker-compose -f infrastructure/docker-compose.yml ps

# Restart infrastructure
docker-compose -f infrastructure/docker-compose.yml down
docker-compose -f infrastructure/docker-compose.yml up -d
```

#### Service Health Check Failures
```bash
# Check service logs
cargo run -p [service-name] 

# Verify ports are available
netstat -an | grep [port-number]
```

#### Geyser Connection Issues (HotPath)
```bash
# Verify credentials
export GEYSER_ENDPOINT=https://grpc.ny.shyft.to
export X_TOKEN=your_actual_token

# Test connection manually
curl -H "x-token: $X_TOKEN" $GEYSER_ENDPOINT
```

### Emergency Procedures

#### Graceful Shutdown
```bash
# Send CTRL+C to orchestrator
# Or: curl -X POST http://localhost:9999/shutdown
```

#### Force Stop All Services
```bash
# Kill all MEVana processes
pkill -f "cargo run -p"
pkill -f "normalizer-service\|edge-assembler\|arb-engine\|executor-service"

# Stop infrastructure
docker-compose -f infrastructure/docker-compose.yml down
```

## Configuration

The System Orchestrator supports custom configuration via `orchestrator-config.toml`. Default configuration includes:

```toml
startup_timeout = "300s"          # Total startup timeout
health_check_interval = "30s"     # Health check frequency  
max_restart_attempts = 3          # Service restart limit
restart_delay = "10s"             # Delay between restarts

[infrastructure]
docker_compose_path = "infrastructure/docker-compose.yml"
redis_url = "redis://localhost:6379"
kafka_bootstrap_servers = "localhost:9092"  
postgres_url = "postgresql://mevana:mevana_pass@localhost:5432/mevana_db"
check_infrastructure = true

# Individual service configurations with dependencies, health checks, etc.
```

## System Requirements

### Minimum Hardware
- **CPU**: 8+ cores (trading requires high concurrency)
- **RAM**: 16GB+ (Kafka + multiple Rust services)  
- **Disk**: 50GB+ SSD (fast I/O for high-frequency trading)
- **Network**: Stable internet with low latency to Solana RPC

### Required Environment Variables
```bash
# Optional - will use defaults if not provided
export GEYSER_ENDPOINT="https://grpc.ny.shyft.to"  
export X_TOKEN="your_geyser_token"
export REDIS_URL="redis://localhost:6379"
export POSTGRES_URL="postgresql://mevana:mevana_pass@localhost:5432/mevana_db"

# Port overrides (optional)
export ORCHESTRATOR_METRICS_PORT=9999
export NORMALIZER_METRICS_PORT=9901
# ... etc for other services
```

The System Orchestrator provides a robust, production-ready foundation for managing the complex MEVana arbitrage trading system with enterprise-grade reliability and observability.