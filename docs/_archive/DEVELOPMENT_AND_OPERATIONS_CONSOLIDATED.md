# MEVana Development & Operations - Consolidated
Last updated: 2025-09-15
**Complete Development Guide & Operational Procedures**

*Last Updated: December 17, 2024*  
*Status: Production-Ready Infrastructure (90% Complete)*

---

## 🚀 **QUICK START GUIDE**

### **Prerequisites**
- **Docker & Docker Compose**: Latest stable versions
- **System Requirements**: 16GB+ RAM, 4+ CPU cores recommended
- **Network Access**: Ports 8080-8096 available
- **Shyft API Key**: Optional for extended features

### **Launch Full Stack**
```bash
# Clone repository
git clone <repository-url>
cd MEVana

# Launch all services
cd infrastructure
docker compose up -d

# Check service health
docker compose ps
docker compose logs <service-name>
```

### **Verify Data Flow**
```bash
# Check Kafka topics
docker exec kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic quotes.jupiter.v1 \
  --max-messages 3 --from-beginning

# Check Redis data
docker exec redis redis-cli keys "*pool*" | head -10
```

---

## 🏗️ **DEVELOPMENT WORKFLOW**

### **Project Structure**
```
MEVana/
├── crates/                 # Core Rust libraries
│   ├── whirlpool-core/    # CLMM mathematics
│   ├── edge-assembler/    # Data pipeline
│   ├── geyser-streamer/   # Solana streaming
│   └── normalizer-service/ # Data normalization
├── infrastructure/         # Docker & deployment
├── docs/                  # Documentation
└── tests/                 # Integration tests
```

### **Development Environment Setup**
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install development dependencies
cargo install cargo-watch
cargo install cargo-audit

# Setup pre-commit hooks
cargo install cargo-husky
cargo husky install
```

### **Local Development (Outside Docker)**
```bash
# Build specific service
cd crates/whirlpool-core
cargo build --release

# Run with hot reload
cargo watch -x 'run --bin whirlpool-core'

# Run tests
cargo test --release
```

---

## 🔧 **SERVICE DEVELOPMENT & DEPLOYMENT**

### **Service Development Pattern**
1. **Core Logic**: Implement in `crates/` directory
2. **Service Wrapper**: Create service binary with health checks
3. **Docker Integration**: Add to docker-compose.yml
4. **Health Monitoring**: Implement `/health` endpoint
5. **Metrics**: Add Prometheus-compatible metrics

### **Adding New DEX Integration**
```rust
// 1. Create parser service
#[derive(Debug, Clone)]
pub struct DexParser {
    pub program_id: Pubkey,
    pub instruction_discriminator: [u8; 8],
}

// 2. Implement transaction parsing
impl DexParser {
    pub fn parse_transaction(&self, tx: &Transaction) -> Result<Vec<Quote>, Error> {
        // Parse transaction and extract quotes
    }
}

// 3. Add to docker-compose.yml
// 4. Configure health checks and monitoring
```

### **Service Configuration**
```yaml
# Example service configuration
services:
  whirlpool-quote-surface:
    build:
      context: ../crates/whirlpool-quote-surface
      dockerfile: Dockerfile.slim
    ports:
      - "8084:8080"
    environment:
      - KAFKA_BROKERS=kafka:9092
      - REDIS_URL=redis://redis:6379
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

---

## 📊 **MONITORING & OBSERVABILITY**

### **Prometheus Configuration**
```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'mevana-services'
    static_configs:
      - targets: ['whirlpool-quote-surface:8080', 'jupiter-parser:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s

  - job_name: 'infrastructure'
    static_configs:
      - targets: ['kafka:9092', 'redis:6379']
    scrape_interval: 15s
```

### **Grafana Dashboards**
- **Service Health**: Real-time service status monitoring
- **Performance Metrics**: Latency, throughput, error rates
- **Business Metrics**: Trade opportunities, profit calculations
- **Infrastructure**: Resource utilization, network performance

### **Health Check Endpoints**
```rust
// Example health check implementation
#[get("/health")]
async fn health_check() -> impl Responder {
    let status = HealthStatus {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    
    Json(status)
}
```

---

## 🔄 **CI/CD PIPELINE**

### **Build Pipeline**
```yaml
# .github/workflows/build.yml
name: Build and Test

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release
      - run: cargo test --release
      - run: cargo audit
```

### **Deployment Pipeline**
```yaml
# .github/workflows/deploy.yml
name: Deploy to Production

on:
  push:
    tags: [ 'v*' ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to production
        run: |
          ssh ${{ secrets.DEPLOY_HOST }} << 'EOF'
            cd /opt/mevana
            git pull origin main
            docker compose down
            docker compose up -d
          EOF
```

---

## 🧪 **TESTING FRAMEWORK**

### **Unit Testing**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_calculation() {
        let pool = PoolState::new_test_data();
        let quote = pool.calculate_quote(1000, true);
        assert!(quote.amount_out > 0);
        assert!(quote.fee_amount > 0);
    }
}
```

### **Integration Testing**
```rust
#[tokio::test]
async fn test_end_to_end_flow() {
    // Setup test environment
    let test_env = TestEnvironment::new().await;
    
    // Send test transaction
    let tx = create_test_transaction();
    test_env.send_transaction(tx).await;
    
    // Verify quote generation
    let quotes = test_env.get_quotes().await;
    assert!(!quotes.is_empty());
}
```

### **Performance Testing**
```rust
#[bench]
fn bench_quote_calculation(b: &mut Bencher) {
    let pool = PoolState::new_test_data();
    b.iter(|| {
        pool.calculate_quote(1000, true)
    });
}
```

---

## 🔐 **SECURITY & COMPLIANCE**

### **Secrets Management**
```bash
# Environment variables for sensitive data
export SHYFT_API_KEY="your_api_key_here"
export SOLANA_PRIVATE_KEY="your_private_key_here"
export JUPITER_API_KEY="your_jupiter_key_here"

# Docker secrets for production
echo "your_secret_here" | docker secret create shyft_api_key -
```

### **Security Best Practices**
- **Image Scanning**: Regular vulnerability scanning
- **Network Policies**: Restricted inter-service communication
- **Access Control**: Role-based access to services
- **Audit Logging**: Comprehensive activity logging

### **Compliance Requirements**
- **Data Privacy**: GDPR compliance for EU users
- **Financial Regulations**: KYC/AML compliance
- **Audit Trails**: Complete transaction logging
- **Risk Management**: Position limits and stop losses

---

## 🚨 **OPERATIONAL PROCEDURES**

### **Service Monitoring**
```bash
# Check service health
docker compose ps

# View service logs
docker compose logs -f <service-name>

# Check resource usage
docker stats

# Monitor Kafka topics
docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list
```

### **Troubleshooting Common Issues**
```bash
# Service won't start
docker compose logs <service-name>
docker exec <service-name> /bin/sh -c "ps aux"

# Kafka connection issues
docker exec kafka kafka-broker-api-versions --bootstrap-server localhost:9092

# Redis connection issues
docker exec redis redis-cli ping
docker exec redis redis-cli info memory
```

### **Backup and Recovery**
```bash
# Database backup
docker exec postgres pg_dump -U postgres mevana > backup.sql

# Redis backup
docker exec redis redis-cli BGSAVE
docker cp redis:/data/dump.rdb ./redis_backup.rdb

# Configuration backup
cp -r infrastructure/ ./backup/infrastructure_$(date +%Y%m%d)
```

---

## 📈 **PERFORMANCE OPTIMIZATION**

### **Build Optimization**
```toml
# Cargo.toml optimization
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.release.build-override]
opt-level = 3
```

### **Runtime Optimization**
```rust
// CPU pinning for critical threads
use core_affinity;

fn pin_critical_thread() {
    let core_ids = core_affinity::get_core_ids().unwrap();
    if let Some(core_id) = core_ids.first() {
        core_affinity::set_for_current(*core_id);
    }
}

// Memory optimization
use dashmap::DashMap;
use ahash::AHashMap;

let pool_states: DashMap<Pubkey, PoolState> = DashMap::new();
let metadata_cache: AHashMap<String, TokenMetadata> = AHashMap::new();
```

### **Network Optimization**
```rust
// Kafka producer optimization
let producer: FutureProducer = ClientConfig::new()
    .set("bootstrap.servers", "kafka:9092")
    .set("linger.ms", "0")
    .set("acks", "1")
    .set("compression.type", "lz4")
    .create()
    .expect("Producer creation failed");

// Redis optimization
let redis_client = redis::Client::open("redis://redis:6379").unwrap();
let mut conn = redis_client.get_async_connection().await.unwrap();
```

---

## 🔮 **FUTURE DEVELOPMENT ROADMAP**

### **Phase 5: Production Readiness (September 2024)**
- **Wallet Management**: Secure key management system
- **Trade Execution**: Automated arbitrage execution
- **Advanced Analytics**: Historical data analysis
- **Performance Monitoring**: Real-time optimization

### **Phase 6: AI Integration (Q4 2024)**
- **AI Agents**: Automated strategy optimization
- **Machine Learning**: Pattern recognition and prediction
- **Natural Language**: Query interface for non-technical users
- **Automated Testing**: AI-driven test case generation

### **Technology Evolution**
- **WebAssembly**: Client-side processing
- **GraphQL**: Unified data access
- **Event Sourcing**: Temporal data management
- **Microservices**: Service mesh architecture

---

## 📚 **RESOURCES & REFERENCES**

### **Documentation**
- **[MEVANA_DOCUMENTATION_SUMMARY.md](MEVANA_DOCUMENTATION_SUMMARY.md)** - Complete project overview
- **[project_context.md](project_context.md)** - Problem statement and solution
- **[tech_stack.md](tech_stack.md)** - Technology inventory
- **[STATUS_AND_PROGRESS_CONSOLIDATED.md](STATUS_AND_PROGRESS_CONSOLIDATED.md)** - Current status

### **External Resources**
- **Solana Documentation**: https://docs.solana.com/
- **Anchor Framework**: https://www.anchor-lang.com/
- **Shyft.to API**: https://docs.shyft.to/
- **Jupiter API**: https://station.jup.ag/docs/apis/swap-api

### **Community & Support**
- **Discord**: MEVana development community
- **GitHub Issues**: Bug reports and feature requests
- **Documentation**: Continuous updates and improvements
- **AI Agents**: Automated development assistance

---

*This consolidated development guide merges information from: development/getting-started.md, deployment/docker-setup.md, observability/observability.md, ci/ci-cd-pipeline.md, and other development files. Recent updates (August 2025) are given priority and detailed treatment.*
