# MEVana: Local Development to Production Deployment
Last updated: 2025-09-15

A complete strategy for seamless local development with reliable, secure production deployment.

## 🎯 Core Objectives

1. **Local Development Parity** - What works locally works in production
2. **Network/Port Consistency** - No confusion between environments  
3. **Secure Visibility** - Full system observability with tight security
4. **Unified Infrastructure** - Rust services integrated with monitoring stack

## 🏗️ Architecture Overview

```
┌─ Local Development ─────────────────┐    ┌─ Production Server ─────────────────┐
│                                     │    │                                     │
│  ┌─ Docker Network: mevana-net ─────│    │  ┌─ Docker Network: mevana-net ─────│
│  │                                 ││    │  │                                 ││
│  │  🦀 Rust Services               ││    │  │  🦀 Rust Services               ││
│  │  ├─ HotPath Arb  :8088         ││    │  │  ├─ HotPath Arb  :8088         ││
│  │  ├─ Normalizer   :8085         ││    │  │  ├─ Normalizer   :8085         ││
│  │  ├─ Edge Asm     :8087         ││    │  │  ├─ Edge Asm     :8087         ││
│  │  ├─ Arb Engine   :8089         ││    │  │  ├─ Arb Engine   :8089         ││
│  │  ├─ Wallet Vault :8090         ││    │  │  ├─ Wallet Vault :8090         ││
│  │  └─ API Service  :8091         ││    │  │  └─ API Service  :8091         ││
│  │                                 ││    │  │                                 ││
│  │  📊 Infrastructure              ││    │  │  📊 Infrastructure              ││
│  │  ├─ Kafka        :9092         ││    │  │  ├─ Kafka        :9092         ││
│  │  ├─ Redis        :6379         ││    │  │  ├─ Redis        :6379         ││
│  │  ├─ PostgreSQL   :5432         ││    │  │  ├─ PostgreSQL   :5432         ││
│  │  ├─ Prometheus   :9090         ││    │  │  ├─ Prometheus   :9090         ││
│  │  └─ Grafana      :3000         ││    │  │  └─ Grafana      :3000         ││
│  └─────────────────────────────────││    │  └─────────────────────────────────││
└─────────────────────────────────────┘    └─────────────────────────────────────┘
         │                                          │
         └─ Identical ports & networking ──────────┘
```

## 🔧 Unified Network Configuration

### Docker Compose - Network Foundation

Create `docker-compose.base.yml`:

```yaml
version: '3.8'

# Shared network configuration
networks:
  mevana-net:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16

# Shared volume configuration  
volumes:
  postgres-data:
  kafka-data:
  redis-data:
  grafana-data:
  prometheus-data:
  wallet-vault-data:

# Service defaults
x-logging: &default-logging
  logging:
    driver: json-file
    options:
      max-size: "10m"
      max-file: "3"

x-restart: &default-restart
  restart: unless-stopped

x-network: &default-network
  networks:
    - mevana-net
```

### Infrastructure Services

Create `docker-compose.infrastructure.yml`:

```yaml
version: '3.8'

services:
  # Message Broker
  kafka:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: confluentinc/cp-kafka:7.4.0
    container_name: mevana-kafka
    ports:
      - "9092:9092"
      - "9101:9101"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:9092
      KAFKA_LISTENERS: PLAINTEXT://0.0.0.0:9092
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_TRANSACTION_STATE_LOG_MIN_ISR: 1
      KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR: 1
      KAFKA_JMX_PORT: 9101
      KAFKA_AUTO_CREATE_TOPICS_ENABLE: true
    volumes:
      - kafka-data:/var/lib/kafka/data
    depends_on:
      - zookeeper
    healthcheck:
      test: ["CMD-SHELL", "kafka-topics --bootstrap-server localhost:9092 --list"]
      interval: 30s
      timeout: 10s
      retries: 3

  zookeeper:
    <<: *default-restart
    <<: *default-logging  
    <<: *default-network
    image: confluentinc/cp-zookeeper:7.4.0
    container_name: mevana-zookeeper
    ports:
      - "2181:2181"
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000

  # Cache & Session Store
  redis:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: redis:7-alpine
    container_name: mevana-redis
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Primary Database
  postgres:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: postgres:15-alpine
    container_name: mevana-postgres
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: mevana
      POSTGRES_USER: mevana_user
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-secure_password_change_me}
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./sql/init:/docker-entrypoint-initdb.d
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U mevana_user -d mevana"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Metrics Collection
  prometheus:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: prom/prometheus:latest
    container_name: mevana-prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus:/etc/prometheus
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=200h'
      - '--web.enable-lifecycle'
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:9090/"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Metrics Visualization  
  grafana:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: grafana/grafana:latest
    container_name: mevana-grafana
    ports:
      - "3000:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin123}
      GF_USERS_ALLOW_SIGN_UP: false
      GF_SERVER_DOMAIN: ${GRAFANA_DOMAIN:-localhost}
      GF_SMTP_ENABLED: ${SMTP_ENABLED:-false}
    volumes:
      - grafana-data:/var/lib/grafana
      - ./monitoring/grafana/provisioning:/etc/grafana/provisioning
      - ./monitoring/grafana/dashboards:/var/lib/grafana/dashboards
    depends_on:
      - prometheus
    healthcheck:
      test: ["CMD-SHELL", "curl -f localhost:3000/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### Rust Services Configuration

Create `docker-compose.services.yml`:

```yaml
version: '3.8'

services:
  # Ultra-low latency arbitrage detection
  hotpath-arb-engine:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: mevana/hotpath_arb_engine:latest
    container_name: mevana-hotpath-arb
    ports:
      - "8088:8080"
    environment:
      # Core configuration
      - RUST_LOG=${HOTPATH_LOG_LEVEL:-info}
      - METRICS_PORT=8080
      - BIND_ADDRESS=0.0.0.0
      
      # External connections
      - GEYSER_ENDPOINT=${GEYSER_ENDPOINT}
      - X_TOKEN=${X_TOKEN}
      
      # Performance tuning
      - CPU_AFFINITY_ENABLED=${CPU_PINNING_ENABLED:-false}
      - THREAD_POOL_SIZE=${HOTPATH_THREADS:-4}
      - MALLOC_ARENA_MAX=2
      
      # Business logic
      - ARB_MIN_PROFIT_BPS=${ARB_MIN_PROFIT_BPS:-25}
      - ARB_MAX_SLIPPAGE_BPS=${ARB_MAX_SLIPPAGE_BPS:-100}
      
      # Internal services
      - KAFKA_BROKER=kafka:9092
      - REDIS_URL=redis://redis:6379/1
    depends_on:
      - kafka
      - redis
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    # Production CPU pinning (only applied if host supports it)
    cpuset: "${HOTPATH_CPU_SET:-}"

  # Transaction parsing and normalization
  normalizer-service:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: mevana/normalizer-service:latest
    container_name: mevana-normalizer
    ports:
      - "8085:8080"
    environment:
      - RUST_LOG=${NORMALIZER_LOG_LEVEL:-info}
      - HEALTH_PORT=8080
      - BIND_ADDRESS=0.0.0.0
      - KAFKA_BROKER=kafka:9092
      - REDIS_URL=redis://redis:6379/2
      - IN_TOPIC=dex-transactions
      - OUT_TOPIC=orca-swaps.parsed.v2
      - GROUP_ID=tap-orca-extract
      - FAST_MODE=1
    depends_on:
      - kafka
      - redis
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Real-time transaction processing
  edge-assembler:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: mevana/edge-assembler:latest
    container_name: mevana-edge-assembler
    ports:
      - "8087:8080"
    environment:
      - RUST_LOG=${EDGE_LOG_LEVEL:-info}
      - KAFKA_BROKER=kafka:9092
      - REDIS_URL=redis://redis:6379/3
      - IN_TOPIC=orca-swaps.parsed.v2
      - OUT_TOPIC=arbitrage-opportunities
    depends_on:
      - kafka
      - redis
      - normalizer-service
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Arbitrage opportunity detection
  arb-engine:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: mevana/arb-engine:latest
    container_name: mevana-arb-engine
    ports:
      - "8089:8080"
    environment:
      - RUST_LOG=${ARB_LOG_LEVEL:-info}
      - KAFKA_BROKER=kafka:9092
      - REDIS_URL=redis://redis:6379/4
      - ARB_SIZING_MODE=${ARB_SIZING_MODE:-adaptive}
      - ARB_MIN_PROFIT_BPS=${ARB_MIN_PROFIT_BPS:-25}
    depends_on:
      - kafka
      - redis
      - edge-assembler
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Secure wallet management
  wallet-vault:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: mevana/wallet-vault:latest
    container_name: mevana-wallet-vault
    ports:
      - "8090:8080"
    environment:
      - RUST_LOG=${WALLET_LOG_LEVEL:-info}
      - DATABASE_URL=postgresql://mevana_user:${POSTGRES_PASSWORD:-secure_password_change_me}@postgres:5432/mevana
      - ENCRYPTION_KEY=${WALLET_ENCRYPTION_KEY}
      - BACKUP_ENABLED=${WALLET_BACKUP_ENABLED:-true}
    volumes:
      - wallet-vault-data:/app/data
    depends_on:
      - postgres
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # Public API and dashboard
  api-service:
    <<: *default-restart
    <<: *default-logging
    <<: *default-network
    image: mevana/api-service:latest
    container_name: mevana-api-service
    ports:
      - "8091:8080"
    environment:
      - RUST_LOG=${API_LOG_LEVEL:-info}
      - DATABASE_URL=postgresql://mevana_user:${POSTGRES_PASSWORD:-secure_password_change_me}@postgres:5432/mevana
      - JWT_SECRET=${JWT_SECRET}
      - CORS_ORIGIN=${CORS_ORIGIN:-http://localhost:3000}
    depends_on:
      - postgres
      - wallet-vault
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

## 🔐 Security & Monitoring Integration

### Prometheus Configuration

Create `monitoring/prometheus/prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yml"

scrape_configs:
  # Infrastructure monitoring
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'redis'
    static_configs:
      - targets: ['redis:6379']
    metrics_path: /metrics

  # Rust services - unified metrics endpoint
  - job_name: 'mevana-services'
    scrape_interval: 5s  # High frequency for trading metrics
    static_configs:
      - targets:
        - 'hotpath-arb-engine:8080'
        - 'normalizer-service:8080'  
        - 'edge-assembler:8080'
        - 'arb-engine:8080'
        - 'wallet-vault:8080'
        - 'api-service:8080'
    metrics_path: /metrics
    scrape_timeout: 10s

  # JVM metrics from Kafka
  - job_name: 'kafka'
    static_configs:
      - targets: ['kafka:9101']

  # System metrics (if deployed with node_exporter)  
  - job_name: 'node'
    static_configs:
      - targets: ['node-exporter:9100']

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093
```

### Security Configuration

Create `security/network-policy.yml`:

```yaml
# Network security policies
version: '3.8'

# Production security overrides
services:
  # Secure Grafana for production
  grafana:
    environment:
      # Disable signup, enable auth
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_AUTH_ANONYMOUS_ENABLED=false
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD}
      - GF_SECURITY_SECRET_KEY=${GRAFANA_SECRET_KEY}
      
      # HTTPS and security headers
      - GF_SERVER_PROTOCOL=${GRAFANA_PROTOCOL:-http}
      - GF_SERVER_CERT_FILE=${GRAFANA_CERT_FILE:-}
      - GF_SERVER_CERT_KEY=${GRAFANA_CERT_KEY:-}
      
      # Secure session
      - GF_SESSION_COOKIE_SECURE=${GRAFANA_COOKIE_SECURE:-false}
      - GF_SESSION_COOKIE_SAME_SITE=strict

  # Secure PostgreSQL  
  postgres:
    environment:
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
    # Only expose to internal network in production
    ports: []
    expose:
      - "5432"

  # Secure Redis
  redis:
    command: >
      redis-server 
      --appendonly yes 
      --requirepass ${REDIS_PASSWORD:-}
      --maxmemory 512mb
      --maxmemory-policy allkeys-lru
    environment:
      - REDIS_PASSWORD=${REDIS_PASSWORD:-}

  # Wallet vault - extra security
  wallet-vault:
    environment:
      - WALLET_ENCRYPTION_KEY=${WALLET_ENCRYPTION_KEY}
      - BACKUP_ENCRYPTION_KEY=${BACKUP_ENCRYPTION_KEY}
      - VAULT_SEAL_TYPE=${VAULT_SEAL_TYPE:-shamir}
    # No external port exposure in production
    ports: []
    expose:
      - "8080"
```

## 🚀 Deployment Scripts

### Local Development Script

Create `scripts/dev-start.sh`:

```bash
#!/bin/bash
# scripts/dev-start.sh - Start local development environment

set -e

echo "🔧 Starting MEVana Development Environment"

# Load development environment
if [ -f .env.dev ]; then
    echo "📝 Loading development configuration..."
    export $(cat .env.dev | grep -v '^#' | xargs)
fi

# Build all services if they don't exist
echo "🔨 Building Rust services..."
services=(
    "hotpath_arb_engine"
    "normalizer-service" 
    "edge-assembler"
    "arb-engine"
    "wallet-vault"
    "api-service"
)

for service in "${services[@]}"; do
    if [[ "$(docker images -q mevana/${service}:latest 2> /dev/null)" == "" ]]; then
        echo "Building ${service}..."
        ./scripts/build-service.sh "$service" dev
    else
        echo "✅ ${service} image exists"
    fi
done

# Start infrastructure first
echo "🏗️ Starting infrastructure services..."
docker-compose \
    -f docker-compose.base.yml \
    -f docker-compose.infrastructure.yml \
    up -d

# Wait for infrastructure to be ready
echo "⏳ Waiting for infrastructure..."
sleep 30

# Start Rust services
echo "🦀 Starting Rust services..."
docker-compose \
    -f docker-compose.base.yml \
    -f docker-compose.services.yml \
    up -d

# Health check
echo "🏥 Checking service health..."
sleep 20
./scripts/health-check.sh

echo "✅ Development environment ready!"
echo ""
echo "🔍 Service URLs:"
echo "  • HotPath Arbitrage:  http://localhost:8088/metrics"
echo "  • Normalizer:         http://localhost:8085/health"
echo "  • Edge Assembler:     http://localhost:8087/health"
echo "  • Arbitrage Engine:   http://localhost:8089/health"
echo "  • Wallet Vault:       http://localhost:8090/health"
echo "  • API Service:        http://localhost:8091/health"
echo ""
echo "📊 Monitoring:"
echo "  • Prometheus:         http://localhost:9090"
echo "  • Grafana:            http://localhost:3000 (admin/admin123)"
echo ""
echo "🗄️ Infrastructure:"
echo "  • Kafka UI:           http://localhost:8080"
echo "  • Redis:              localhost:6379"
echo "  • PostgreSQL:         localhost:5432"
```

### Production Deployment Script

Create `scripts/prod-deploy.sh`:

```bash
#!/bin/bash
# scripts/prod-deploy.sh - Deploy to production server

set -e

SERVER_HOST=${1:-"your-server.com"}
DEPLOY_USER=${2:-"deploy"}

echo "🚀 Deploying MEVana to Production: $SERVER_HOST"

# Verify we have all required environment variables
required_vars=(
    "GEYSER_ENDPOINT"
    "X_TOKEN"
    "POSTGRES_PASSWORD"
    "JWT_SECRET"
    "WALLET_ENCRYPTION_KEY"
    "GRAFANA_ADMIN_PASSWORD"
)

for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "❌ Missing required environment variable: $var"
        echo "Please set in .env.prod or export manually"
        exit 1
    fi
done

# Build services for production
echo "🔨 Building production images..."
services=(
    "hotpath_arb_engine"
    "normalizer-service"
    "edge-assembler" 
    "arb-engine"
    "wallet-vault"
    "api-service"
)

for service in "${services[@]}"; do
    ./scripts/build-service.sh "$service" prod
    
    # Tag for registry if configured
    if [ ! -z "$DOCKER_REGISTRY" ]; then
        docker tag "mevana/${service}:latest" "${DOCKER_REGISTRY}/mevana/${service}:latest"
        docker push "${DOCKER_REGISTRY}/mevana/${service}:latest"
    fi
done

# Secure copy deployment files to server
echo "📁 Copying deployment files to server..."
rsync -avz --exclude='.git' \
    --exclude='target' \
    --exclude='node_modules' \
    --include='docker-compose*.yml' \
    --include='monitoring/' \
    --include='scripts/' \
    --include='sql/' \
    ./ ${DEPLOY_USER}@${SERVER_HOST}:/opt/mevana/

# Copy environment configuration
scp .env.prod ${DEPLOY_USER}@${SERVER_HOST}:/opt/mevana/.env

# Execute deployment on server
echo "🚀 Executing deployment on server..."
ssh ${DEPLOY_USER}@${SERVER_HOST} << 'EOF'
cd /opt/mevana

# Load production environment
export $(cat .env | grep -v '^#' | xargs)

# Set production-specific overrides
export CPU_PINNING_ENABLED=true
export HOTPATH_CPU_SET="2-5"
export ENVIRONMENT=production

# Create secure directory permissions
sudo chown -R deploy:deploy /opt/mevana
chmod 600 .env

# Start services with production configuration
docker-compose \
    -f docker-compose.base.yml \
    -f docker-compose.infrastructure.yml \
    -f docker-compose.services.yml \
    -f security/network-policy.yml \
    up -d

# Wait for services to start
sleep 60

# Verify deployment
./scripts/health-check.sh

echo "✅ Production deployment complete!"
EOF

echo "🎉 Deployment to $SERVER_HOST completed successfully!"
```

### Environment Files

Create `.env.dev`:

```bash
# Development Environment Configuration
ENVIRONMENT=development

# External Services (use test endpoints for dev)
GEYSER_ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=your_dev_token_here

# Database
POSTGRES_PASSWORD=dev_password_123

# Security (less strict for dev)
JWT_SECRET=dev_jwt_secret_change_for_prod
WALLET_ENCRYPTION_KEY=dev_wallet_key_32_chars_long!!
GRAFANA_PASSWORD=admin123

# Performance (no CPU pinning for dev)
CPU_PINNING_ENABLED=false
HOTPATH_THREADS=2

# Logging (verbose for development)  
HOTPATH_LOG_LEVEL=debug
NORMALIZER_LOG_LEVEL=info
EDGE_LOG_LEVEL=info
ARB_LOG_LEVEL=debug
WALLET_LOG_LEVEL=info
API_LOG_LEVEL=debug

# Business Logic
ARB_MIN_PROFIT_BPS=10
ARB_SIZING_MODE=adaptive
```

Create `.env.prod.example`:

```bash
# Production Environment Configuration
ENVIRONMENT=production

# External Services (CHANGE THESE!)
GEYSER_ENDPOINT=https://your-production-geyser-endpoint
X_TOKEN=your_production_token_here

# Database Security
POSTGRES_PASSWORD=super_secure_production_password_change_me
REDIS_PASSWORD=redis_production_password_change_me

# Application Security  
JWT_SECRET=production_jwt_secret_64_chars_long_change_this_immediately!!
WALLET_ENCRYPTION_KEY=wallet_encryption_key_32_chars_!!
BACKUP_ENCRYPTION_KEY=backup_encryption_key_32_chars_!!
GRAFANA_ADMIN_PASSWORD=secure_grafana_password
GRAFANA_SECRET_KEY=grafana_secret_key_change_me

# SSL/TLS (if using HTTPS)
GRAFANA_PROTOCOL=https
GRAFANA_CERT_FILE=/certs/grafana.crt
GRAFANA_CERT_KEY=/certs/grafana.key
GRAFANA_COOKIE_SECURE=true

# Performance Optimization
CPU_PINNING_ENABLED=true
HOTPATH_CPU_SET=2-5
HOTPATH_THREADS=4

# Logging (less verbose for production)
HOTPATH_LOG_LEVEL=warn
NORMALIZER_LOG_LEVEL=info
EDGE_LOG_LEVEL=info
ARB_LOG_LEVEL=info
WALLET_LOG_LEVEL=warn
API_LOG_LEVEL=info

# Business Logic
ARB_MIN_PROFIT_BPS=25
ARB_SIZING_MODE=adaptive

# Monitoring
SMTP_ENABLED=true
SMTP_HOST=your-smtp-server.com
SMTP_USER=alerts@yourdomain.com
SMTP_PASSWORD=smtp_password
```

## 📊 Unified Monitoring Dashboard

All services (Rust + infrastructure) report to the same Prometheus instance with consistent metrics:

### Service Health Dashboard
```
┌─ System Overview ────────────────────────┐
│                                          │
│ ✅ HotPath Arb    ⚡ 456μs avg latency   │
│ ✅ Normalizer     📊 1.2k tx/sec         │  
│ ✅ Edge Assembler 🔄 145 arb/min         │
│ ✅ Arb Engine     💰 $2.3k profit/day    │
│ ✅ Wallet Vault   🔒 3 wallets healthy   │
│ ✅ API Service    🌐 42 requests/min     │
│                                          │
│ 🏗️ Infrastructure                        │
│ ✅ Kafka          📬 98% delivery        │
│ ✅ Redis          ⚡ 0.1ms avg           │
│ ✅ PostgreSQL     🗄️ 45% storage         │ 
│ ✅ Prometheus     📊 1.2M samples        │
└──────────────────────────────────────────┘
```

### Security Monitoring
- Failed authentication attempts
- Unusual wallet access patterns  
- Network traffic anomalies
- CPU/memory resource exhaustion
- Service crash/restart events

## 🎯 Key Benefits

✅ **Perfect Local-to-Prod Parity** - Identical networking, ports, and service discovery
✅ **Zero Port Conflicts** - Consistent port allocation across environments  
✅ **Unified Monitoring** - Single Grafana dashboard for everything
✅ **Secure by Default** - Production security with development usability
✅ **One-Command Deployment** - `./scripts/dev-start.sh` or `./scripts/prod-deploy.sh`
✅ **Integrated Security** - Rust services seamlessly monitored with infrastructure
✅ **Environment Parity** - Same Docker networks, service discovery, and communication patterns

This approach eliminates the confusion between environments while maintaining the security and performance requirements for production MEV trading.
