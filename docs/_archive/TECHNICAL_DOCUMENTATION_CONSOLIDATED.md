# MEVana Technical Documentation - Consolidated
**Complete Technical Architecture & Implementation Details**

*Last Updated: December 17, 2024*  
*Status: Production-Ready Infrastructure (90% Complete)*

---

## 🏗️ **SYSTEM ARCHITECTURE OVERVIEW**

### **High-Level Data Flow**
```
Solana Geyser → DEX Parsers → Normalizers → Quote Surfaces → Arbitrage Detection → Execution Engine
                    ↓                    ↓              ↓                    ↓                    ↓
                 Kafka Bus          Redis Cache    Price Calculation   Opportunity Scoring   Jito Bundles
```

### **Core Architecture Principles**
1. **Speed-First Design**: Sub-10ms hot path with integer-only mathematics
2. **Premium Infrastructure**: Shyft.to Geyser, RPC, and Jito bundle access
3. **Multi-Venue Coverage**: Comprehensive DEX integration (Orca, Jupiter, Raydium, Meteora, Pump.fun)
4. **Fault Tolerance**: Multi-provider flash loans, graceful degradation, comprehensive monitoring
5. **Real-Time Validation**: Hybrid approach combining speed (Geyser) with accuracy (Shyft APIs)

---

## 🔧 **CORE COMPONENTS & SERVICES**

### **Data Ingestion Layer**
- **`dex-stream-universal`**: Multi-DEX Geyser streaming with program ID filtering
- **`dex-stream-universal/Dockerfile.slim`**: Isolated builds for production
- **Program ID Filtering**: Per-DEX configuration for targeted data collection

### **Parsing & Processing**
- **`jupiter-parser-service`**: Runtime IDL decoding with SHA2 discriminators
- **`whirlpool-state-streamer`**: Account updates and tick array management to Redis
- **`raydium-quote-service`**: AMM price calculations and pool analysis
- **`pump-quote-service`**: Bonding curve analysis and token pricing
- **`meteora-quote-service`**: Dynamic AMM (DLMM) transaction processing

### **Quote Generation**
- **`whirlpool-quote-surface`**: CLMM math with TickWindow and liquidity-weighted pricing
- **Real-time Price Calculation**: Sub-10ms latency with Q64.64 precision
- **Token Metadata**: Decimals, symbols via RPC cache with TTL management

### **Infrastructure**
- **Docker Compose**: 17-service orchestration with health monitoring
- **Kafka Topics**: `quotes.jupiter.v1`, `quotes.whirlpool.v1`, `quotes.raydium.v1`, etc.
- **Redis**: Pool states, tick arrays, metadata cache with persistence
- **Health Checks**: `/health` endpoints on all services with Prometheus metrics

---

## 📊 **CURRENT SERVICE STATUS (17 Active)**

| Service | Status | Purpose | Port | Topic |
|---------|--------|---------|------|-------|
| **dex-stream-universal** | ✅ Running | Multi-DEX Geyser streaming | 8084 | `dex.universal.v1` |
| **jupiter-tx-stream** | ✅ Running | Jupiter transaction streaming | 8092 | `dex.jupiter.v1` |
| **jupiter-parser** | ✅ Running | Transaction parsing | 8082 | `dex.jupiter.parsed.v1` |
| **jupiter-integration** | ✅ Running | External API integration | 8083 | `quotes.jupiter.api.v1` |
| **whirlpool-tx-stream** | ✅ Running | Whirlpool transaction streaming | 8091 | `dex.whirlpool.v1` |
| **whirlpool-state-streamer** | ✅ Running | Account state management | 8093 | Redis cache |
| **whirlpool-quote-surface** | ✅ Running | CLMM price calculations | 8084 | `quotes.whirlpool.v1` |
| **pump-tx-stream** | ✅ Running | Pump.fun transaction streaming | 8094 | `dex.pump.v1` |
| **pump-quote-service** | ✅ Running | Bonding curve pricing | 8085 | `quotes.pump.v1` |
| **raydium-tx-stream** | ✅ Running | Raydium transaction streaming | 8095 | `dex.raydium.v1` |
| **raydium-quote-service** | ✅ Running | AMM price calculations | 8086 | `quotes.raydium.v1` |
| **meteora-tx-stream** | ✅ Running | Meteora transaction streaming | 8096 | `dex.meteora.v1` |
| **meteora-quote-service** | ✅ Running | DAMM pricing | 8087 | `quotes.meteora.v1` |
| **zookeeper** | ✅ Running | Kafka coordination | 2181 | - |
| **kafka** | ✅ Running | Message broker | 9092 | - |
| **redis** | ✅ Running | Data caching | 6379 | - |
| **postgres** | ✅ Running | Data persistence | 5432 | - |

---

## 🌐 **NETWORK ARCHITECTURE & PORT ALLOCATION**

### **Port Allocation Strategy**
- **8080-8089**: Main application services
- **9090-9099**: Metrics and monitoring
- **8090-8099**: Health checks and status

### **Critical Port Reservations**
- **Port 8084**: 🔒 **PERMANENTLY RESERVED** for `dex-stream-universal` (Primary data origin)
- **Port 8083**: Token registry service (Default assignment)
- **Port 8082**: UI gateway (WebSocket connections)
- **Port 8080**: Executor service (Arbitrage execution)

### **Network Configuration**
- **Custom Network**: `mevana_net` (external Docker network)
- **Internal Communication**: Docker DNS (e.g., `kafka:9092`)
- **External Access**: Localhost ports for development
- **Health Monitoring**: All services with `/health` endpoints

---

## 🔍 **PARSER VALIDATION & QUALITY ASSURANCE**

### **Validation Framework (Most Recent: August 2025)**
- **Hybrid Validation Strategy**: Geyser streams for speed, Shyft APIs for accuracy
- **Real-time Validation**: Continuous data quality monitoring
- **Cross-Reference Validation**: Compare our parsing with Shyft API results
- **Error Recovery**: Graceful handling of parsing failures

### **Quality Metrics**
- **Parsing Success Rate**: >99% with hybrid approach
- **Validation Latency**: <100ms for Shyft API validation
- **Data Freshness**: Real-time updates with age filtering
- **Accuracy Verification**: Continuous validation against external sources

### **Validation Services**
- **`shyft-validator`**: Standalone validation service for AI agents
- **`shyft-pool-depth`**: Pool depth validation and market data verification
- **`discovery-service`**: Unknown program detection for AI analysis

---

## 🚀 **PERFORMANCE OPTIMIZATION & HOT PATH**

### **Hot Path Rules (Non-Negotiable)**
- **No Cross-Process Calls**: All operations within single process
- **Integer-Only Math**: Q64.64 fixed-point arithmetic for quote-critical calculations
- **Precomputed Instructions**: Meta instructions prepared, hot path fills amounts only
- **Single-Threaded Detection**: Pinned thread for arbitrage detection loop
- **Freshness Enforcement**: Slot and write_version validation with age thresholds

### **Low-Latency Configuration**
- **Kafka Producer**: `linger.ms=0`, `acks=1`, `compression=lz4`
- **Kafka Consumer**: `fetch.min.bytes=1`, `fetch.wait.max.ms=1-2`
- **Redis**: Optimized memory policies with TTL-based expiration
- **Tokio Runtime**: CPU pinning and tuned worker configuration

### **Build Optimization**
- **LTO Thin**: Link-time optimization for performance
- **Opt Level 3**: Maximum optimization for production
- **Cargo Workspace**: Shared dependency management
- **Docker Multi-stage**: Optimized container builds

---

## 🔐 **SECURITY & INFRASTRUCTURE HARDENING**

### **Security Implementation Plan (Most Recent: August 2025)**
- **Secrets Management**: Docker secrets and environment variable protection
- **Image Provenance**: Pinned digests and security scanning
- **Network Policy**: Restricted inter-service traffic to `mevana_net`
- **Authentication**: Basic auth for UI services, API key management

### **Infrastructure Reliability**
- **Health Checks**: Comprehensive service monitoring
- **Graceful Degradation**: Fallback mechanisms for critical services
- **Resource Limits**: Memory and CPU constraints per service
- **Log Management**: Rotating logs with size limits

---

## 📈 **OBSERVABILITY & MONITORING**

### **Metrics Collection (Most Recent: August 2025)**
- **Prometheus**: Time-series metrics database with custom exporters
- **Grafana**: Real-time dashboards and visualization
- **Custom Metrics**: Service-specific performance indicators
- **Scrape Intervals**: 1s (performance), 5s (services), 15s (infrastructure)

### **Health Monitoring**
- **Service Health**: `/health` endpoints on all services
- **Docker Health Checks**: Container-level monitoring
- **Dependency Validation**: Startup sequence with health checks
- **Alert Rules**: Performance degradation and error condition alerts

### **Performance Baselines**
- **Latency Histograms**: Geyser→extractor, extractor→arb, arb decision time
- **Throughput Counters**: Messages per second, processing rates
- **Error Tracking**: Decode failures, validation errors, system issues
- **Resource Utilization**: CPU, memory, network usage per service

---

## 🔄 **CI/CD & DEPLOYMENT PIPELINE**

### **Deployment Strategy**
- **Docker Compose**: Multi-service orchestration with health checks
- **Environment Profiles**: Development, testing, and production configurations
- **Service Dependencies**: Health-check based startup sequencing
- **Rolling Updates**: Zero-downtime service updates

### **Testing Framework**
- **Integration Testing**: End-to-end data flow validation
- **Performance Testing**: Latency and throughput measurement
- **Load Testing**: High-volume transaction processing validation
- **Automated Validation**: CI/CD pipeline with observability integration

---

## 🔮 **FUTURE TECHNICAL ROADMAP**

### **Phase 5: Production Readiness (September 2024)**
- **Wallet Management**: Secure key management and transaction signing
- **Trade Execution**: Automated arbitrage execution with risk management
- **Advanced Analytics**: Historical analysis and pattern recognition
- **Performance Optimization**: Latency reduction and throughput improvement

### **Phase 6: AI-Driven Enhancement (Q4 2024)**
- **Self-Learning Strategies**: AI agents optimizing arbitrage algorithms
- **Predictive Analytics**: Market condition anticipation and opportunity prediction
- **Automated Risk Management**: AI-driven risk assessment and mitigation
- **Strategy Evolution**: Continuous strategy improvement through AI learning

### **Technology Evolution**
- **WebAssembly**: Client-side processing capabilities
- **GraphQL Federation**: Unified data access patterns
- **Event Sourcing**: Temporal data management
- **CQRS**: Command-query responsibility separation

---

*This consolidated technical documentation merges information from: ARCHITECTURE_OVERVIEW.md, PARSER_VALIDATION_FRAMEWORK.md, SYSTEMATIC_VALIDATION_PLAN.md, CI_CD_PIPELINE_SPEC.md, SECURITY_IMPLEMENTATION_PLAN.md, and other technical files. Recent updates (August 2025) are given priority and detailed treatment.*
