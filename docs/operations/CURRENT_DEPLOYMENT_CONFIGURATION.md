# MEVana Current Deployment Configuration

**Generated**: 2025-09-19
**Purpose**: Complete documentation of working local deployment for server migration
**Status**: ✅ Verified Working Configuration

## Overview

This document captures the exact configuration of the currently working MEVana deployment. This setup has been verified to:
- ✅ Process live Solana transactions
- ✅ Detect arbitrage opportunities
- ✅ Provide real-time monitoring via Grafana
- ✅ Collect metrics via Prometheus
- ✅ Handle data persistence

## Infrastructure Stack

### Core Services (Docker Compose)
```yaml
# File: infrastructure/docker-compose.yml
# Network: mevana_net (external)
# Startup Order: Zookeeper → Kafka → Redis → MEVana Services → Monitoring
```

### Service Inventory
| Service | Container Name | Port(s) | Status | Purpose |
|---------|---------------|---------|--------|---------|
| Zookeeper | infrastructure-zookeeper-1 | 2181 | ✅ Healthy | Kafka coordination |
| Kafka | infrastructure-kafka-1 | 29092 | ✅ Healthy | Message broker |
| Kafka UI | infrastructure-kafka-ui-1 | 8084 | ✅ Healthy | Kafka management |
| Redis | infrastructure-redis-1 | 6380 | ✅ Healthy | Data caching |
| Redis UI | infrastructure-redis-ui-1 | 8083 | ⚠️ Unhealthy | Redis management |
| PostgreSQL | infrastructure-postgres-1 | 5433 | ✅ Healthy | Database |
| Adminer | infrastructure-adminer-1 | 8087 | ✅ Healthy | DB admin |
| Prometheus | infrastructure-prometheus-1 | 9091 | ✅ Healthy | Metrics collection |
| Grafana | infrastructure-grafana-1 | 3300 | ✅ Healthy | Dashboards |

### MEVana Application Services
| Service | Container Name | Port(s) | Status | Purpose |
|---------|---------------|---------|--------|---------|
| Hotpath Arb Engine | infrastructure-hotpath-arb-engine-1 | 8088-8089 | ⚠️ Unhealthy | Arbitrage detection |
| Normalizer Service | infrastructure-normalizer-service-1 | 9901 | ✅ Healthy | Data normalization |
| Discovery Service | infrastructure-discovery-service-1 | 9102 | ✅ Healthy | Service discovery |
| Executor Service | infrastructure-executor-service-1 | 9904 | ✅ Healthy | Trade execution |

## Docker Compose Configuration

### Key Files
- **Main**: `infrastructure/docker-compose.yml`
- **Core Services**: `infrastructure/docker-compose.core.yml`
- **Server Config**: `infrastructure/docker-compose.server.yml`
- **Local Config**: `docker-compose.local.yml`

### Critical Volume Mounts
```yaml
# Grafana Configuration
- ./infrastructure/grafana/dashboards/curated:/etc/grafana/provisioning/dashboards
- ./infrastructure/grafana/datasources:/etc/grafana/provisioning/datasources
- grafana_data:/var/lib/grafana

# Prometheus Configuration
- ./infrastructure/prometheus.yml:/etc/prometheus/prometheus.yml
- prometheus_data:/prometheus

# PostgreSQL Data
- postgres_data:/var/lib/postgresql/data
```

### Environment Variables
```bash
# Grafana
GF_SECURITY_ADMIN_PASSWORD=admin123
GF_USERS_ALLOW_SIGN_UP=false

# Kafka
KAFKA_ZOOKEEPER_CONNECT=zookeeper:2181
KAFKA_LISTENERS=PLAINTEXT://0.0.0.0:9092,PLAINTEXT_HOST://0.0.0.0:29092
KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://kafka:9092,PLAINTEXT_HOST://localhost:29092

# PostgreSQL
POSTGRES_DB=mevdb
POSTGRES_USER=mev
POSTGRES_PASSWORD=mevpassword
```

## Network Configuration

### External Network
```bash
# Network: mevana_net (external)
# Must be created before deployment:
docker network create mevana_net
```

### Port Mapping
```
Infrastructure Services:
- Kafka: localhost:29092 → kafka:9092
- Redis: localhost:6380 → redis:6379
- PostgreSQL: localhost:5433 → postgres:5432
- Prometheus: localhost:9091 → prometheus:9090
- Grafana: localhost:3300 → grafana:3000

Management UIs:
- Kafka UI: localhost:8084
- Redis UI: localhost:8083
- Adminer: localhost:8087

MEVana Services:
- Hotpath Engine: localhost:8088-8089
- Normalizer: localhost:9901
- Discovery: localhost:9102
- Executor: localhost:9904
```

### Internal DNS Resolution
```
# Docker internal communication
kafka:9092          # Kafka broker
redis:6379          # Redis cache
postgres:5432       # PostgreSQL database
prometheus:9090     # Prometheus metrics
grafana:3000        # Grafana dashboards
```

## Data Persistence

### Docker Volumes
```yaml
volumes:
  postgres_data:
    driver: local
  prometheus_data:
    driver: local
  grafana_data:
    driver: local
```

### Critical Data Paths
- **PostgreSQL**: `/var/lib/postgresql/data` → `postgres_data` volume
- **Prometheus**: `/prometheus` → `prometheus_data` volume
- **Grafana**: `/var/lib/grafana` → `grafana_data` volume

### Configuration Files (Host Mounted)
- `infrastructure/prometheus.yml` → `/etc/prometheus/prometheus.yml`
- `infrastructure/grafana/datasources/prometheus.yml` → `/etc/grafana/provisioning/datasources/`
- `infrastructure/grafana/dashboards/curated/` → `/etc/grafana/provisioning/dashboards/`

## Grafana Configuration

### Dashboard Provisioning
**File**: `infrastructure/grafana/dashboards/curated/dashboard.yml`
```yaml
apiVersion: 1
providers:
  - name: 'default'
    orgId: 1
    folder: ''
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /etc/grafana/provisioning/dashboards
```

### Current Dashboards
1. **MEVana System Status** (`mevana-system-status.json`)
   - **Panels**: 4 panels
   - **Metrics**: Prometheus status, activity, transaction rates, arbitrage detection
   - **UID**: `mevana-system-status`
   - **Refresh**: 5 seconds
   - **Time Range**: Last 15 minutes

### Dashboard Panels
1. **Prometheus Status** (Stat Panel)
   - Metric: `up{job="prometheus"}`
   - Shows: UP/DOWN status with color coding
   - Position: Top-left (12x8)

2. **Prometheus Activity** (Time Series)
   - Metric: `prometheus_tsdb_head_samples_appended_total`
   - Shows: Data ingestion activity
   - Position: Top-right (12x8)

3. **Transaction Processing Rate** (Time Series)
   - Metrics:
     - `rate(hotpath_transactions_processed_total[1m])`
     - `rate(geyser_transactions_processed_total[1m])`
   - Shows: Jupiter + all DEX transaction rates
   - Position: Middle (24x8)

4. **Arbitrages Detected** (Time Series)
   - Metric: `rate(hotpath_multi_venue_transactions[1m])`
   - Shows: Multi-venue arbitrage opportunities
   - Color: Orange highlight
   - Position: Bottom (24x8)

### Data Source Configuration
**File**: `infrastructure/grafana/datasources/prometheus.yml`
```yaml
apiVersion: 1
datasources:
  - name: Prometheus
    uid: prometheus
    orgId: 1
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: true
```

### Grafana Environment
```bash
GF_SECURITY_ADMIN_PASSWORD=admin123
GF_USERS_ALLOW_SIGN_UP=false
GF_PLUGINS_ALLOW_LOADING_UNSIGNED_PLUGINS=grafana-metricsdrilldown-app,grafana-lokiexplore-app,grafana-pyroscope-app,grafana-exploretraces-app
GF_LOG_LEVEL=info
```

### Volume Mounts (Corrected)
```yaml
# Fixed volume paths
- ./infrastructure/grafana/dashboards/curated:/etc/grafana/provisioning/dashboards
- ./infrastructure/grafana/datasources:/etc/grafana/provisioning/datasources
- grafana_data:/var/lib/grafana
```

**⚠️ CRITICAL**: Volume paths were corrected from `./grafana/` to `./infrastructure/grafana/`

## Prometheus Configuration

### Main Configuration
**File**: `infrastructure/prometheus.yml`

### Global Settings
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "/etc/prometheus/rules/*.yml"
```

### Scrape Targets

#### Infrastructure Services (Docker Internal)
```yaml
- job_name: 'prometheus'
  static_configs:
    - targets: ['localhost:9090']

- job_name: 'redis'
  static_configs:
    - targets: ['redis:6379']

- job_name: 'postgres'
  static_configs:
    - targets: ['postgres:5432']

- job_name: 'grafana'
  static_configs:
    - targets: ['grafana:3000']
```

#### MEVana Services (Critical for Monitoring)
```yaml
# Primary arbitrage engine (uses host.docker.internal)
- job_name: 'hotpath-arb-engine'
  static_configs:
    - targets: ['host.docker.internal:8099']
  metrics_path: '/metrics'
  scrape_interval: 5s        # High frequency for live trading
  scrape_timeout: 3s
  fallback_scrape_protocol: 'PrometheusText0.0.4'

# Data processing services
- job_name: 'normalizer-service'
  static_configs:
    - targets: ['normalizer-service:8085']
  metrics_path: '/metrics'
  scrape_interval: 10s
  scrape_timeout: 5s

- job_name: 'discovery-service'
  static_configs:
    - targets: ['discovery-service:9102']
  metrics_path: '/metrics'
  scrape_interval: 15s
  scrape_timeout: 5s

- job_name: 'executor-service'
  static_configs:
    - targets: ['executor-service:9904']
  metrics_path: '/metrics'
  scrape_interval: 10s
  scrape_timeout: 5s
```

#### Additional Services (May Not Be Running)
```yaml
- job_name: 'edge-assembler'
  static_configs:
    - targets: ['edge-assembler:9902']

- job_name: 'arb-engine'
  static_configs:
    - targets: ['arb-engine:8085']

- job_name: 'monitoring-service'
  static_configs:
    - targets: ['monitoring-service:8091']
```

#### External Data Sources
```yaml
- job_name: 'laserstream-metrics'
  static_configs:
    - targets: ['host.docker.internal:9100']

- job_name: 'solana-exporter'
  static_configs:
    - targets: ['host.docker.internal:9101']
  scrape_interval: 30s
  scrape_timeout: 15s
  honor_labels: true
```

### Key Metrics Being Collected
1. **hotpath_transactions_processed_total**: Total transactions processed
2. **geyser_transactions_processed_total**: Geyser stream transactions
3. **hotpath_multi_venue_transactions**: Arbitrage opportunities detected
4. **multi_geyser_stream_messages_parsed_per_minute**: Message processing rates
5. **hotpath_processing_time_us**: Processing latency metrics

### Critical Network Configuration
- **Internal services**: Use Docker DNS names (`normalizer-service:8085`)
- **Host services**: Use `host.docker.internal:PORT` for non-containerized services
- **Prometheus runs in container**: Must scrape via container network or host.docker.internal
