# MEVana Observability Configuration Guide

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Services      │───▶│   Prometheus    │───▶│   Grafana       │
│   (/metrics)    │    │   (scraping)    │    │   (dashboards)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Exporters     │    │   Alertmanager  │    │   Notebooks     │
│   (infra data)  │    │   (notifications)│    │   (analysis)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Configuration Methods

### Method 1: Docker Compose (Current - Development)

**Pros:**
- Simple setup and teardown
- Good for development and testing
- Automatic service discovery within Docker network

**Cons:**
- Not suitable for production scale
- Single point of failure
- Limited to single host

**Configuration:**
```yaml
# infrastructure/docker-compose.yml
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    ports:
      - "9090:9090"

  grafana:
    image: grafana/grafana:latest
    volumes:
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources
      - grafana_data:/var/lib/grafana
    ports:
      - "3000:3000"
```

### Method 2: Kubernetes (Production Recommended)

**Pros:**
- Auto-scaling and high availability
- Service mesh integration
- Advanced networking and security
- GitOps workflow compatibility

**Example Configuration:**
```yaml
# k8s/monitoring/prometheus.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
    scrape_configs:
      - job_name: 'mevana-services'
        kubernetes_sd_configs:
          - role: pod
        relabel_configs:
          - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
            action: keep
            regex: true
```

### Method 3: Managed Services (Cloud Production)

**Pros:**
- Fully managed, no infrastructure overhead
- Built-in alerting and automation
- Enterprise security and compliance
- Integrated with cloud provider services

**Options:**
- **AWS**: CloudWatch + Managed Grafana + Managed Prometheus
- **GCP**: Cloud Monitoring + Cloud Logging + Managed Prometheus
- **Azure**: Azure Monitor + Application Insights + Managed Grafana

## Service Configuration Best Practices

### 1. Metrics Endpoint Pattern

```rust
// crates/shared-core/src/metrics.rs
use metrics_exporter_prometheus::PrometheusBuilder;

pub fn init_metrics(port: u16) -> Result<()> {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], port))
        .install()
        .expect("Failed to install Prometheus recorder");
    Ok(())
}

// Each service main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize metrics FIRST
    shared_core::metrics::init_metrics(8080)?;
    
    // Start your service logic
    // ...
}
```

### 2. Environment-Based Configuration

```bash
# .env.development
METRICS_PORT=8086
PROMETHEUS_ENDPOINT=http://localhost:9090
GRAFANA_ENDPOINT=http://localhost:3000

# .env.production  
METRICS_PORT=8080
PROMETHEUS_ENDPOINT=http://prometheus.monitoring.svc.cluster.local:9090
GRAFANA_ENDPOINT=https://grafana.example.com
```

### 3. Service Discovery Configuration

#### Docker Compose (Current)
```yaml
# infrastructure/prometheus.yml
scrape_configs:
  - job_name: 'mevana-services'
    static_configs:
      - targets: 
        - 'host.docker.internal:8086'  # multi-geyser-streams
        - 'host.docker.internal:8081'  # arb-engine
        - 'host.docker.internal:8082'  # executor-service
```

#### Kubernetes (Production)
```yaml
scrape_configs:
  - job_name: 'mevana-services'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_mevana_metrics]
        action: keep
        regex: true
```

## Dashboard Management

### Method 1: File-based Provisioning (Current)
```yaml
# infrastructure/grafana/dashboards/dashboard.yml
apiVersion: 1
providers:
  - name: 'mevana-dashboards'
    type: file
    updateIntervalSeconds: 10
    options:
      path: /etc/grafana/provisioning/dashboards
```

**Benefits:**
- Version controlled
- Automatic updates
- Consistent across environments

### Method 2: API-based Deployment
```bash
# Deploy dashboard via API
curl -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $GRAFANA_API_KEY" \
  -d @dashboard.json \
  http://grafana.example.com/api/dashboards/db
```

### Method 3: GitOps with ArgoCD/Flux
```yaml
# charts/grafana/templates/dashboard-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mevana-dashboards
data:
  multi-geyser-streams.json: |
{{ .Files.Get "dashboards/multi-geyser-streams.json" | indent 4 }}
```

## CI/CD Integration

### 1. Automated Dashboard Updates
```yaml
# .github/workflows/observability.yml
name: Deploy Observability

on:
  push:
    paths: ['infrastructure/grafana/dashboards/**']

jobs:
  deploy-dashboards:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to Grafana
        run: |
          for dashboard in infrastructure/grafana/dashboards/*.json; do
            curl -X POST \
              -H "Content-Type: application/json" \
              -H "Authorization: Bearer ${{ secrets.GRAFANA_API_KEY }}" \
              -d @$dashboard \
              ${{ secrets.GRAFANA_URL }}/api/dashboards/db
          done
```

### 2. Configuration Validation
```bash
# scripts/validate-observability.sh
#!/bin/bash

# Validate Prometheus config
promtool check config infrastructure/prometheus.yml

# Validate dashboard JSONs
for dashboard in infrastructure/grafana/dashboards/*.json; do
  jq empty "$dashboard" || exit 1
done

# Test metrics endpoints
for service in multi-geyser-streams arb-engine; do
  curl -f "http://localhost:$(get_port $service)/metrics" > /dev/null
done
```

## Configuration Recommendations by Environment

### Development (Current Setup)
- ✅ **Docker Compose** with file-based provisioning
- ✅ **Static targets** in prometheus.yml
- ✅ **Local storage** for metrics and dashboards

### Staging
- **Docker Compose** or **Docker Swarm**
- **Service discovery** via Docker DNS
- **External volumes** for persistence
- **Basic alerting** to Slack/email

### Production
- **Kubernetes** with operators (Prometheus Operator, Grafana Operator)
- **Service mesh integration** (Istio/Linkerd)
- **External storage** (S3, GCS, Azure Blob)
- **Full alerting** with PagerDuty integration
- **High availability** with multiple replicas

## Migration Path

### Phase 1: Standardize Current Setup ✅
- [x] Fix dashboard provisioning
- [x] Standardize metrics patterns
- [x] Document configuration

### Phase 2: Add Missing Components
- [ ] Add exporters (Redis, Postgres, Kafka)
- [ ] Configure alerting rules
- [ ] Add log aggregation (Loki)

### Phase 3: Production Readiness
- [ ] Move to Kubernetes
- [ ] Add GitOps workflow
- [ ] External storage integration
- [ ] Multi-environment support

### Phase 4: Advanced Features
- [ ] Distributed tracing (Jaeger/Tempo)
- [ ] SLI/SLO monitoring
- [ ] Cost optimization
- [ ] AI-powered anomaly detection

## Quick Commands

### Development
```bash
# Start full stack
cd infrastructure && docker compose up -d

# Restart with new config
docker compose restart prometheus grafana

# View metrics
curl http://localhost:8086/metrics
```

### Validation
```bash
# Test Prometheus targets
curl http://localhost:9090/api/v1/targets

# Test Grafana API
curl -H "Authorization: Bearer $API_KEY" http://localhost:3000/api/health
```

### Troubleshooting
```bash
# Check service logs
docker compose logs prometheus grafana

# Validate configs
promtool check config prometheus.yml
jq empty grafana/dashboards/*.json
```