# MEVana Monitoring Stack Setup

## Overview
Complete monitoring solution for the MEVana DEX Stream Universal V2 service using Prometheus, Grafana, and PostgreSQL.

## Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ dex-stream-     │    │ Prometheus      │    │ Grafana         │
│ universal-v2    │───▶│ (Port 9090)     │───▶│ (Port 3000)     │
│ (Port 8084)     │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │ PostgreSQL      │
                       │ (Port 5432)     │
                       └─────────────────┘
```

## Prerequisites
- Docker and Docker Compose installed
- `mevana_net` Docker network exists
- Ports 8084, 9090, 3000, 5432 available

## Quick Start

### 1. Deploy Monitoring Stack
```bash
./scripts/deploy-monitoring.sh
```

### 2. Deploy DEX Stream Service
```bash
./scripts/deploy-dex-stream.sh
```

### 3. Access Services
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **DEX Stream Health**: http://localhost:8084/health
- **DEX Stream Metrics**: http://localhost:8084/metrics

## Configuration

### Environment Variables
All network settings are configurable via environment variables:

```bash
# Load from env.monitoring
POSTGRES_PORT=5432
PROMETHEUS_PORT=9090
GRAFANA_PORT=3000
HEALTH_PORT=8084
```

### Network Configuration
- **External Network**: `mevana_net`
- **Service Discovery**: Uses Docker service names
- **No Hardcoded IPs**: All configurable

## Monitoring Features

### Prometheus Metrics
- `dex_stream_v2_transactions_processed_total`
- `dex_stream_v2_transaction_processing_seconds`
- `dex_stream_v2_health_checks_total`
- `dex_stream_v2_health_check_failures_total`

### Grafana Dashboard
- Real-time transaction throughput
- Processing latency histograms
- Health check status
- Auto-refresh every 5 seconds

## Troubleshooting

### Check Service Status
```bash
# Monitoring stack
docker-compose -f docker-compose.monitoring.yml ps

# DEX Stream service
docker-compose ps
```

### View Logs
```bash
# Monitoring stack
docker-compose -f docker-compose.monitoring.yml logs -f

# DEX Stream service
docker-compose logs -f dex_stream_universal_dev
```

### Verify Metrics Flow
1. Check Prometheus targets: http://localhost:9090/targets
2. Query metrics: http://localhost:9090/graph
3. View dashboard: http://localhost:3000

## Security Notes
- Default passwords in development only
- Change credentials for production
- Network isolation via Docker networks
- No hardcoded network addresses

## Next Steps
1. Implement metrics collection in the service
2. Add alerting rules
3. Create custom dashboards
4. Set up production monitoring
