# MEVana Deployment Strategy
Last updated: 2025-09-15

A comprehensive guide for deploying Rust services with optimal performance and maximum portability.

## 🎯 Current Challenges

1. **Each service has its own Dockerfile** - lots of duplication
2. **No consistent CPU/performance optimization**
3. **Missing multi-stage build optimization**
4. **No deployment orchestration**
5. **CPU pinning portability issues** across different environments

## 🚀 Improved Deployment Strategy

### 1. Unified Multi-Service Dockerfile

Replace individual service Dockerfiles with a single, parameterized Dockerfile:

```dockerfile
# Single Dockerfile for all MEVana services
FROM rust:1.83-bookworm AS builder

# Install dependencies once
RUN apt-get update && apt-get install -y \
    cmake build-essential pkg-config \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build argument for which service to build
ARG SERVICE_NAME
ARG BINARY_NAME=${SERVICE_NAME}

# Optimized release build
ENV CARGO_TARGET_DIR=/app/target
RUN cargo build --release -p ${SERVICE_NAME} --bin ${BINARY_NAME}

# Runtime stage - optimized for performance
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates curl \
    htop procps \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
ARG SERVICE_NAME
ARG BINARY_NAME=${SERVICE_NAME}
COPY --from=builder /app/target/release/${BINARY_NAME} /usr/local/bin/service

# Performance optimizations
ENV RUST_LOG=info
ENV MALLOC_ARENA_MAX=2
ENV RUST_BACKTRACE=1

# CPU optimization environment
ENV CPU_CORES_AVAILABLE=""
ENV CPU_AFFINITY_ENABLED="false"
ENV THREAD_POOL_SIZE=""

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/service"]
```

### 2. Smart Build Script

Create `scripts/build-service.sh`:

```bash
#!/bin/bash
# scripts/build-service.sh

SERVICE_NAME=$1
TARGET_ENV=${2:-dev}

if [ -z "$SERVICE_NAME" ]; then
    echo "Usage: $0 <service-name> [dev|prod]"
    exit 1
fi

# Build with environment-specific optimizations
docker build \
    --build-arg SERVICE_NAME="$SERVICE_NAME" \
    --build-arg BINARY_NAME="$SERVICE_NAME" \
    --target runtime \
    --platform linux/amd64,linux/arm64 \
    -t "mevana/$SERVICE_NAME:latest" \
    -t "mevana/$SERVICE_NAME:$(git rev-parse --short HEAD)" \
    .

echo "Built mevana/$SERVICE_NAME for $TARGET_ENV"
```

Make executable:
```bash
chmod +x scripts/build-service.sh
```

### 3. Environment-Aware Docker Compose

Create `docker-compose.services.yml`:

```yaml
version: '3.8'

x-service-defaults: &service-defaults
  restart: unless-stopped
  logging:
    driver: "json-file"
    options:
      max-size: "10m"
      max-file: "3"
  environment:
    - RUST_LOG=info
    - RUST_BACKTRACE=1
  networks:
    - mevana-net

x-performance-defaults: &performance-defaults
  <<: *service-defaults
  deploy:
    resources:
      limits:
        memory: 512M
      reservations:
        memory: 256M

services:
  hotpath-arb-engine:
    <<: *performance-defaults
    image: mevana/hotpath_arb_engine:latest
    ports:
      - "8088:8080"
    environment:
      - CPU_AFFINITY_ENABLED=${CPU_PINNING_ENABLED:-false}
      - THREAD_POOL_SIZE=${HOTPATH_THREADS:-4}
      - GEYSER_ENDPOINT=${GEYSER_ENDPOINT}
      - X_TOKEN=${X_TOKEN}
      - METRICS_PORT=8088
    # Production CPU pinning (only if host supports it)
    cpuset: "${HOTPATH_CPU_SET:-}"
    
  normalizer-service:
    <<: *service-defaults
    image: mevana/normalizer-service:latest
    ports:
      - "8085:8080"
    environment:
      - KAFKA_BROKER=kafka:9092
      - REDIS_URL=redis://redis:6379/0
      - IN_TOPIC=dex-transactions
      - OUT_TOPIC=orca-swaps.parsed.v2
    depends_on:
      - kafka
      - redis

  edge-assembler:
    <<: *service-defaults
    image: mevana/edge-assembler:latest
    ports:
      - "8087:8080"
    depends_on:
      - kafka
      - redis

  arb-engine:
    <<: *service-defaults
    image: mevana/arb-engine:latest
    ports:
      - "8089:8080"
    environment:
      - ARB_SIZING_MODE=${ARB_SIZING_MODE:-adaptive}
      - ARB_MIN_PROFIT_BPS=${ARB_MIN_PROFIT_BPS:-25}
    depends_on:
      - kafka
      - redis

  wallet-vault:
    <<: *service-defaults
    image: mevana/wallet-vault:latest
    ports:
      - "8090:8080"
    environment:
      - DATABASE_URL=${DATABASE_URL:-postgresql://postgres:password@postgres:5432/mevana}
    volumes:
      - wallet-data:/app/data
    depends_on:
      - postgres

  api-service:
    <<: *service-defaults
    image: mevana/api-service:latest
    ports:
      - "8091:8080"
    environment:
      - DATABASE_URL=${DATABASE_URL:-postgresql://postgres:password@postgres:5432/mevana}
    depends_on:
      - postgres

volumes:
  wallet-data:

networks:
  mevana-net:
    driver: bridge
```

### 4. Production Overrides

Create `docker-compose.prod.yml`:

```yaml
version: '3.8'

services:
  hotpath-arb-engine:
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '4'
        reservations:
          memory: 512M
          cpus: '2'
    environment:
      - RUST_LOG=warn
      - CPU_AFFINITY_ENABLED=true
    # Dedicated cores for ultra-low latency
    cpuset: "2-5"

  normalizer-service:
    deploy:
      replicas: 2
      resources:
        limits:
          memory: 512M
        reservations:
          memory: 256M

  edge-assembler:
    deploy:
      replicas: 3
      resources:
        limits:
          memory: 512M
        reservations:
          memory: 256M
```

### 5. Deployment Orchestration

Create `scripts/deploy.sh`:

```bash
#!/bin/bash
# scripts/deploy.sh

ENVIRONMENT=${1:-dev}
SERVICES=${2:-"all"}

echo "🚀 Deploying MEVana services to $ENVIRONMENT"

# Build services
if [ "$SERVICES" = "all" ]; then
    SERVICES="hotpath_arb_engine normalizer-service edge-assembler arb-engine wallet-vault api-service"
fi

for service in $SERVICES; do
    echo "Building $service..."
    ./scripts/build-service.sh "$service" "$ENVIRONMENT"
done

# Deploy based on environment
case $ENVIRONMENT in
    "dev")
        echo "🔧 Starting development environment..."
        docker-compose \
            -f infrastructure/docker-compose.yml \
            -f docker-compose.services.yml \
            up -d
        ;;
    "prod")
        echo "🚀 Starting production environment..."
        # Use environment-specific overrides
        export CPU_PINNING_ENABLED=true
        export HOTPATH_CPU_SET="2-5"
        export HOTPATH_THREADS=4
        export ARB_SIZING_MODE=adaptive
        export ARB_MIN_PROFIT_BPS=25
        
        docker-compose \
            -f infrastructure/docker-compose.yml \
            -f docker-compose.services.yml \
            -f docker-compose.prod.yml \
            up -d
        ;;
    "test")
        echo "🧪 Starting test environment..."
        export CPU_PINNING_ENABLED=false
        docker-compose \
            -f infrastructure/docker-compose.yml \
            -f docker-compose.services.yml \
            up -d --scale normalizer-service=1 --scale edge-assembler=1
        ;;
esac

echo "✅ Deployment complete!"
echo "📊 Monitor services: docker-compose logs -f"
echo "🔍 Health checks: ./scripts/health-check.sh"
```

Make executable:
```bash
chmod +x scripts/deploy.sh
```

### 6. Health Check Script

Create `scripts/health-check.sh`:

```bash
#!/bin/bash
# scripts/health-check.sh

SERVICES=(
    "hotpath-arb-engine:8088"
    "normalizer-service:8085"
    "edge-assembler:8087"
    "arb-engine:8089"
    "wallet-vault:8090"
    "api-service:8091"
)

echo "🏥 Checking service health..."

for service_port in "${SERVICES[@]}"; do
    service=${service_port%:*}
    port=${service_port#*:}
    
    if curl -f -s "http://localhost:$port/health" > /dev/null; then
        echo "✅ $service (port $port) - Healthy"
    else
        echo "❌ $service (port $port) - Unhealthy"
    fi
done

echo "🔍 Container status:"
docker-compose ps
```

Make executable:
```bash
chmod +x scripts/health-check.sh
```

### 7. Environment Configuration

Create `.env.example`:

```bash
# MEVana Environment Configuration

# === Deployment Settings ===
ENVIRONMENT=dev
CPU_PINNING_ENABLED=false
HOTPATH_CPU_SET=""
HOTPATH_THREADS=4

# === External Services ===
GEYSER_ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=your_shyft_token_here

# === Database ===
DATABASE_URL=postgresql://postgres:password@postgres:5432/mevana

# === Arbitrage Settings ===
ARB_SIZING_MODE=adaptive
ARB_MIN_PROFIT_BPS=25
ARB_MAX_SLIPPAGE_BPS=100

# === Kafka Settings ===
KAFKA_BROKER=kafka:9092

# === Redis Settings ===
REDIS_URL=redis://redis:6379/0
```

## 🎯 Usage Examples

### Development Deployment
```bash
# Copy and configure environment
cp .env.example .env
# Edit .env with your settings

# Deploy all services for development
./scripts/deploy.sh dev

# Deploy specific services
./scripts/deploy.sh dev "hotpath_arb_engine normalizer-service"

# Check health
./scripts/health-check.sh
```

### Production Deployment
```bash
# Set production environment variables
export ENVIRONMENT=prod
export CPU_PINNING_ENABLED=true
export HOTPATH_CPU_SET="2-5"
export GEYSER_ENDPOINT="your-prod-endpoint"
export X_TOKEN="your-prod-token"

# Deploy to production
./scripts/deploy.sh prod

# Monitor logs
docker-compose logs -f hotpath-arb-engine
```

### Building Individual Services
```bash
# Build a specific service
./scripts/build-service.sh hotpath_arb_engine prod

# Build for multiple architectures
docker buildx build \
    --build-arg SERVICE_NAME=hotpath_arb_engine \
    --platform linux/amd64,linux/arm64 \
    -t mevana/hotpath_arb_engine:latest \
    .
```

## 🔧 CPU Pinning Strategy

### Graceful Degradation
Services automatically detect available CPU resources:

```rust
// Example implementation in services
fn optimize_cpu_affinity() -> Result<()> {
    let cpu_pinning_enabled = env::var("CPU_AFFINITY_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()?;
    
    if !cpu_pinning_enabled {
        info!("CPU pinning disabled - running with default scheduler");
        return Ok(());
    }
    
    let cores = num_cpus::get();
    let thread_pool_size = env::var("THREAD_POOL_SIZE")
        .unwrap_or_else(|_| cores.to_string())
        .parse::<usize>()?;
    
    if cores >= 4 {
        // Pin to specific cores for isolation
        set_affinity(&[2, 3])?;
        info!("CPU pinning enabled: cores 2-3");
    } else {
        warn!("Insufficient cores for optimal pinning (available: {})", cores);
    }
    
    Ok(())
}
```

### Environment-Specific CPU Settings

| Environment | CPU Pinning | Core Assignment | Purpose |
|-------------|-------------|-----------------|---------|
| **Development** | Disabled | Default scheduler | Flexibility |
| **Testing** | Disabled | Limited resources | CI/CD compatibility |
| **Production** | Enabled | Cores 2-5 | Ultra-low latency |
| **Cloud** | Conditional | Auto-detect | Adapt to instance type |

## 📊 Monitoring & Observability

### Prometheus Metrics
Each service exposes metrics at `/metrics`:
- Request latency (P50, P95, P99)
- CPU utilization per core
- Memory usage
- Arbitrage opportunities detected
- Transaction processing rates

### Grafana Dashboards
Import dashboards from `infrastructure/grafana/dashboards/`:
- Service health overview
- Performance metrics
- CPU pinning effectiveness
- Arbitrage profitability

## 🎯 Key Benefits

✅ **Single build process** for all services
✅ **Environment-aware CPU optimization** 
✅ **Multi-architecture support** (x86_64/ARM64)
✅ **Graceful degradation** when CPU pinning not available
✅ **Consistent logging/monitoring**
✅ **Easy scaling** per environment
✅ **Automated health checking**
✅ **Infrastructure as Code**

## 🔄 Migration from Current Setup

1. **Backup current configuration**:
   ```bash
   cp -r crates/*/Dockerfile docker-backup/
   ```

2. **Create unified Dockerfile** at repository root

3. **Update existing services** to use environment variables for configuration

4. **Test deployment**:
   ```bash
   ./scripts/deploy.sh test
   ./scripts/health-check.sh
   ```

5. **Gradually migrate** service by service

This strategy provides **maximum portability** while preserving **performance optimizations** where available. Services automatically adapt to their deployment environment, making the same container work seamlessly from development laptops to high-performance production servers.
