# CI/CD Pipeline Specification
Last updated: 2025-09-15
*Date: 2025-01-20*
*Priority: CRITICAL - Required for production deployment*

## 🚀 **AUTOMATED BUILD PIPELINE**

### **GitHub Actions Workflow**
```yaml
# .github/workflows/main.yml
name: MEVana Production Pipeline

on:
  push:
    branches: [main, develop, feature/*]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # Code Quality & Security
  quality-gate:
    name: Quality Gate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
          
      - name: Check formatting
        run: cargo fmt --all -- --check
        
      - name: Run Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
        
      - name: Security audit
        run: |
          cargo install cargo-audit
          cargo audit
          
      - name: License check
        run: |
          cargo install cargo-deny
          cargo deny check licenses

  # Unit & Integration Tests
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    needs: quality-gate
    services:
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
      kafka:
        image: confluentinc/cp-kafka:latest
        env:
          KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
          KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:9092
        ports:
          - 9092:9092
          
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          
      - name: Run unit tests
        run: cargo test --all-features --workspace
        env:
          REDIS_URL: redis://localhost:6379
          KAFKA_BROKERS: localhost:9092
          
      - name: Run integration tests
        run: ./scripts/run-integration-tests.sh
        env:
          TEST_ENV: ci
          
      - name: Generate test coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out xml --output-dir coverage/
          
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          file: coverage/cobertura.xml

  # Build & Package
  build:
    name: Build & Package
    runs-on: ubuntu-latest
    needs: test
    strategy:
      matrix:
        target: [x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu]
        
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          
      - name: Build release
        run: cargo build --release --target ${{ matrix.target }}
        
      - name: Package binaries
        run: |
          mkdir -p dist/${{ matrix.target }}
          cp target/${{ matrix.target }}/release/dex-stream-universal dist/${{ matrix.target }}/
          cp target/${{ matrix.target }}/release/normalizer-service dist/${{ matrix.target }}/
          cp target/${{ matrix.target }}/release/edge-assembler dist/${{ matrix.target }}/
          cp target/${{ matrix.target }}/release/searcher dist/${{ matrix.target }}/
          cp target/${{ matrix.target }}/release/executor dist/${{ matrix.target }}/
          tar -czf mevana-${{ matrix.target }}.tar.gz -C dist/${{ matrix.target }} .
          
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: mevana-${{ matrix.target }}
          path: mevana-${{ matrix.target }}.tar.gz

  # Docker Images
  docker:
    name: Build Docker Images
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2
        
      - name: Login to Container Registry
        uses: docker/login-action@v2
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
          
      - name: Build and push images
        run: |
          docker buildx build --platform linux/amd64,linux/arm64 \
            -t ghcr.io/${{ github.repository }}/dex-stream-universal:${{ github.sha }} \
            -t ghcr.io/${{ github.repository }}/dex-stream-universal:latest \
            --push \
            -f crates/dex-stream-universal/Dockerfile .
            
          docker buildx build --platform linux/amd64,linux/arm64 \
            -t ghcr.io/${{ github.repository }}/normalizer-service:${{ github.sha }} \
            -t ghcr.io/${{ github.repository }}/normalizer-service:latest \
            --push \
            -f crates/normalizer-service/Dockerfile .

  # Staging Deployment
  deploy-staging:
    name: Deploy to Staging
    runs-on: ubuntu-latest
    needs: [build, docker]
    if: github.ref == 'refs/heads/develop'
    environment: staging
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Deploy to staging
        run: ./scripts/deploy-staging.sh
        env:
          STAGING_HOST: ${{ secrets.STAGING_HOST }}
          STAGING_SSH_KEY: ${{ secrets.STAGING_SSH_KEY }}
          DOCKER_REGISTRY: ghcr.io
          IMAGE_TAG: ${{ github.sha }}
          
      - name: Run smoke tests
        run: ./scripts/staging-smoke-tests.sh
        env:
          STAGING_API_URL: ${{ secrets.STAGING_API_URL }}
          
      - name: Performance benchmarks
        run: ./scripts/performance-benchmarks.sh
        env:
          STAGING_API_URL: ${{ secrets.STAGING_API_URL }}

  # Production Deployment
  deploy-production:
    name: Deploy to Production
    runs-on: ubuntu-latest
    needs: [build, docker]
    if: github.ref == 'refs/heads/main'
    environment: production
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Production health check
        run: ./scripts/pre-deployment-health-check.sh
        env:
          PRODUCTION_API_URL: ${{ secrets.PRODUCTION_API_URL }}
          
      - name: Blue-green deployment
        run: ./scripts/deploy-blue-green.sh
        env:
          PRODUCTION_HOST: ${{ secrets.PRODUCTION_HOST }}
          PRODUCTION_SSH_KEY: ${{ secrets.PRODUCTION_SSH_KEY }}
          DOCKER_REGISTRY: ghcr.io
          IMAGE_TAG: ${{ github.sha }}
          
      - name: Post-deployment validation
        run: ./scripts/post-deployment-validation.sh
        env:
          PRODUCTION_API_URL: ${{ secrets.PRODUCTION_API_URL }}
          
      - name: Rollback on failure
        if: failure()
        run: ./scripts/rollback-deployment.sh
        env:
          PRODUCTION_HOST: ${{ secrets.PRODUCTION_HOST }}
```

---

## 📦 **DEPLOYMENT SCRIPTS**

### **Blue-Green Deployment Script**
```bash
#!/bin/bash
# scripts/deploy-blue-green.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
BLUE_CLUSTER="mevana-blue"
GREEN_CLUSTER="mevana-green"
LOAD_BALANCER="mevana-lb"
HEALTH_CHECK_TIMEOUT=300

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[WARN] $1${NC}"
}

# Determine current active cluster
get_active_cluster() {
    kubectl get service $LOAD_BALANCER -o jsonpath='{.spec.selector.cluster}' 2>/dev/null || echo "blue"
}

# Deploy to inactive cluster
deploy_to_inactive_cluster() {
    local active_cluster=$1
    local inactive_cluster
    
    if [[ "$active_cluster" == "blue" ]]; then
        inactive_cluster="green"
    else
        inactive_cluster="blue"
    fi
    
    log "Deploying to inactive cluster: $inactive_cluster"
    
    # Update deployment with new image
    kubectl set image deployment/mevana-deployment \
        dex-stream-universal=ghcr.io/$GITHUB_REPOSITORY/dex-stream-universal:$IMAGE_TAG \
        normalizer-service=ghcr.io/$GITHUB_REPOSITORY/normalizer-service:$IMAGE_TAG \
        edge-assembler=ghcr.io/$GITHUB_REPOSITORY/edge-assembler:$IMAGE_TAG \
        searcher=ghcr.io/$GITHUB_REPOSITORY/searcher:$IMAGE_TAG \
        executor=ghcr.io/$GITHUB_REPOSITORY/executor:$IMAGE_TAG \
        -n $inactive_cluster
        
    # Wait for rollout to complete
    kubectl rollout status deployment/mevana-deployment -n $inactive_cluster --timeout=${HEALTH_CHECK_TIMEOUT}s
    
    echo $inactive_cluster
}

# Health check
health_check() {
    local cluster=$1
    local service_url="http://mevana-service.$cluster.svc.cluster.local/health"
    
    log "Running health check on $cluster cluster"
    
    for i in {1..30}; do
        if kubectl run health-check-$RANDOM --rm -i --restart=Never --image=curlimages/curl -- \
           curl -f -s "$service_url" > /dev/null 2>&1; then
            log "Health check passed for $cluster cluster"
            return 0
        fi
        
        warn "Health check attempt $i/30 failed, retrying in 10s..."
        sleep 10
    done
    
    error "Health check failed for $cluster cluster"
    return 1
}

# Switch traffic
switch_traffic() {
    local new_cluster=$1
    
    log "Switching traffic to $new_cluster cluster"
    
    kubectl patch service $LOAD_BALANCER -p '{"spec":{"selector":{"cluster":"'$new_cluster'"}}}'
    
    # Wait for traffic switch to complete
    sleep 30
    
    # Verify traffic switch
    local current_cluster=$(get_active_cluster)
    if [[ "$current_cluster" == "$new_cluster" ]]; then
        log "Traffic successfully switched to $new_cluster cluster"
        return 0
    else
        error "Failed to switch traffic to $new_cluster cluster"
        return 1
    fi
}

# Main deployment function
main() {
    log "Starting blue-green deployment"
    
    # Get current active cluster
    local active_cluster=$(get_active_cluster)
    log "Current active cluster: $active_cluster"
    
    # Deploy to inactive cluster
    local new_cluster=$(deploy_to_inactive_cluster $active_cluster)
    
    # Run health checks on new cluster
    if ! health_check $new_cluster; then
        error "Health check failed, aborting deployment"
        exit 1
    fi
    
    # Switch traffic to new cluster
    if ! switch_traffic $new_cluster; then
        error "Failed to switch traffic, rolling back"
        switch_traffic $active_cluster
        exit 1
    fi
    
    log "Blue-green deployment completed successfully"
    log "New active cluster: $new_cluster"
}

main "$@"
```

### **Rollback Script**
```bash
#!/bin/bash
# scripts/rollback-deployment.sh

set -euo pipefail

# Get previous deployment
PREVIOUS_DEPLOYMENT=$(kubectl rollout history deployment/mevana-deployment -o jsonpath='{.items[-2].metadata.name}')

log "Rolling back to previous deployment: $PREVIOUS_DEPLOYMENT"

# Rollback deployment
kubectl rollout undo deployment/mevana-deployment

# Wait for rollback to complete
kubectl rollout status deployment/mevana-deployment --timeout=300s

# Run health checks
./scripts/health-check.sh

log "Rollback completed successfully"
```

---

## 🧪 **TESTING FRAMEWORK**

### **Integration Test Suite**
```bash
#!/bin/bash
# scripts/run-integration-tests.sh

set -euo pipefail

# Start test infrastructure
docker-compose -f docker-compose.test.yml up -d

# Wait for services to be ready
./scripts/wait-for-services.sh

# Run API tests
cargo test --test api_integration_tests -- --test-threads=1

# Run data pipeline tests
cargo test --test pipeline_integration_tests -- --test-threads=1

# Run arbitrage detection tests
cargo test --test arbitrage_integration_tests -- --test-threads=1

# Run performance tests
cargo test --test performance_tests -- --test-threads=1

# Cleanup
docker-compose -f docker-compose.test.yml down

echo "All integration tests passed!"
```

### **Performance Benchmarks**
```rust
// tests/performance_benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_parser_performance(c: &mut Criterion) {
    c.bench_function("parse_whirlpool_swap", |b| {
        b.iter(|| {
            // Benchmark parser performance
        });
    });
}

fn benchmark_opportunity_detection(c: &mut Criterion) {
    c.bench_function("detect_arbitrage_opportunity", |b| {
        b.iter(|| {
            // Benchmark opportunity detection
        });
    });
}

fn benchmark_end_to_end_latency(c: &mut Criterion) {
    c.bench_function("end_to_end_latency", |b| {
        b.iter(|| {
            // Benchmark complete pipeline
        });
    });
}

criterion_group!(
    benches,
    benchmark_parser_performance,
    benchmark_opportunity_detection,
    benchmark_end_to_end_latency
);
criterion_main!(benches);
```

---

## 📊 **MONITORING & OBSERVABILITY**

### **Deployment Monitoring**
```yaml
# monitoring/deployment-dashboard.yml
apiVersion: v1
kind: ConfigMap
metadata:
  name: deployment-dashboard
data:
  dashboard.json: |
    {
      "dashboard": {
        "title": "MEVana Deployment Dashboard",
        "panels": [
          {
            "title": "Deployment Status",
            "type": "stat",
            "targets": [
              {
                "expr": "up{job=\"mevana\"}"
              }
            ]
          },
          {
            "title": "Response Time",
            "type": "graph",
            "targets": [
              {
                "expr": "histogram_quantile(0.95, http_request_duration_seconds_bucket{job=\"mevana\"})"
              }
            ]
          },
          {
            "title": "Error Rate",
            "type": "graph",
            "targets": [
              {
                "expr": "rate(http_requests_total{job=\"mevana\",code!~\"2..\"}[5m])"
              }
            ]
          }
        ]
      }
    }
```

### **Alerting Rules**
```yaml
# monitoring/alerting-rules.yml
groups:
  - name: deployment.rules
    rules:
      - alert: DeploymentFailed
        expr: kube_deployment_status_replicas_unavailable > 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Deployment {{ $labels.deployment }} has unavailable replicas"
          
      - alert: HighErrorRate
        expr: rate(http_requests_total{code!~"2.."}[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected: {{ $value }}"
          
      - alert: HighLatency
        expr: histogram_quantile(0.95, http_request_duration_seconds_bucket) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected: {{ $value }}s"
```

---

## 📋 **CI/CD IMPLEMENTATION CHECKLIST**

### **Phase 1: Basic Pipeline (Days 1-2)**
- [ ] **Repository Setup**
  - [ ] Create GitHub Actions workflows
  - [ ] Set up branch protection rules
  - [ ] Configure repository secrets
  - [ ] Set up environments (staging, production)

- [ ] **Build Pipeline**
  - [ ] Implement automated builds
  - [ ] Set up code quality checks
  - [ ] Configure security scanning
  - [ ] Set up test execution

### **Phase 2: Testing & Quality (Days 3-4)**
- [ ] **Test Suite**
  - [ ] Implement unit tests
  - [ ] Create integration tests
  - [ ] Set up performance benchmarks
  - [ ] Configure test coverage reporting

- [ ] **Quality Gates**
  - [ ] Implement code formatting checks
  - [ ] Set up linting rules
  - [ ] Configure security audits
  - [ ] Set up dependency scanning

### **Phase 3: Deployment (Days 5-7)**
- [ ] **Infrastructure**
  - [ ] Set up staging environment
  - [ ] Configure production environment
  - [ ] Implement blue-green deployment
  - [ ] Set up monitoring and alerting

- [ ] **Deployment Scripts**
  - [ ] Create deployment automation
  - [ ] Implement health checks
  - [ ] Set up rollback procedures
  - [ ] Configure monitoring dashboards

This CI/CD pipeline ensures reliable, automated deployment with comprehensive testing and monitoring.
