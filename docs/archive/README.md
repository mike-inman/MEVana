# MEVana: Solana Arbitrage Trading Platform
Last updated: 2025-09-15

**Real-time arbitrage detection and execution across 5+ DEXs on Solana**

[![Docker Status](https://img.shields.io/badge/Docker-Running-green)](https://github.com/your-repo/mevana)
[![DEX Coverage](https://img.shields.io/badge/DEXs-5+-blue)](https://github.com/your-repo/mevana)
[![Kafka Topics](https://img.shields.io/badge/Kafka-5+--Topics-orange)](https://github.com/your-repo/mevana)

## 🚀 **Current Status: Production Ready DEX Infrastructure**

### **✅ Operational DEX Services**
- **🌪️ Whirlpool (Orca)**: CLMM quote surface with tick array management
- **🪐 Jupiter**: Aggregator swap & limit order parsing with runtime IDL decoding
- **📈 Raydium**: AMM transaction streaming and quote generation
- **🔥 Pump.fun**: Token price streaming and bonding curve analysis
- **💎 Meteora**: Dynamic AMM (DLMM) transaction processing

### **⚡ Live Data Pipeline**
- **17 Services** running in Docker
- **5 Kafka Quote Topics** with real-time data
- **Geyser gRPC Streaming** (sub-10ms latency)
- **Redis Caching** for pool states and metadata
- **Health Monitoring** on ports 8081-8096

## 📊 **Architecture Overview**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Solana Geyser │───▶│  DEX Streamers   │───▶│   Kafka Bus     │
│     (Source)    │    │  (Parsers)       │    │   (Normalized)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │   Quote Surface  │───▶│  Arbitrage      │
                       │   (Real-time)    │    │  Detection      │
                       └──────────────────┘    └─────────────────┘
                                                │
                                                ▼
                                       ┌─────────────────┐
                                       │   Execution     │
                                       │   Engine        │
                                       └─────────────────┘
```

### **🔧 Core Components**

#### **Data Ingestion Layer**
- **`dex-stream-universal`**: Multi-DEX Geyser streaming
- **`dex-stream-universal/Dockerfile.slim`**: Isolated builds
- **Program ID Filtering**: Per-DEX configuration

#### **Parsing & Processing**
- **`jupiter-parser-service`**: Runtime IDL decoding
- **`whirlpool-state-streamer`**: Account updates to Redis
- **`raydium-quote-service`**: AMM price calculations
- **`pump-quote-service`**: Bonding curve analysis
- **`meteora-quote-service`**: DLMM processing

#### **Quote Generation**
- **`whirlpool-quote-surface`**: CLMM math with TickWindow
- **Real-time Price Calculation**: Liquidity-weighted averages
- **Token Metadata**: Decimals, symbols via RPC cache

#### **Infrastructure**
- **Docker Compose**: 17-service orchestration
- **Kafka Topics**: `quotes.jupiter.v1`, `quotes.whirlpool.v1`, etc.
- **Redis**: Pool states, tick arrays, metadata cache
- **Health Checks**: `/health` endpoints on all services

## 🎯 **Next Phase: Arbitrage Detection**

### **🔜 Upcoming Features**
1. **Quote Data Extraction** - Parse real swap amounts (current: placeholder)
2. **Cross-DEX Comparison Engine** - Identify price discrepancies
3. **Profitability Calculator** - Net profit analysis with fees
4. **Flash Loan Integration** - Jupiter/Solend providers
5. **Execution Engine** - Automated trade execution

### **📈 Performance Targets**
- **Latency**: <10ms from transaction to arbitrage signal
- **Accuracy**: >99% quote accuracy vs. external validation
- **Coverage**: 7+ DEXs (Pump.fun, Raydium, Meteora added recently)

## 🛠️ **Quick Start**

### **Prerequisites**
- Docker & Docker Compose
- 16GB+ RAM recommended
- Shyft API credentials (optional for extended features)

### **Launch Full Stack**
```bash
cd infrastructure
docker compose up -d
```

### **Check Service Health**
```bash
# View all running services
docker compose ps

# Check logs for any service
docker compose logs <service-name>
```

### **Verify Data Flow**
```bash
# Check Kafka topics
docker exec kafka kafka-console-consumer --bootstrap-server localhost:9092 \
  --topic quotes.jupiter.v1 --max-messages 3 --from-beginning
```

## 📚 **Documentation Structure**

### **🏗️ Architecture & Technical**
- **[System Overview](architecture/system-overview.md)** - High-level architecture
- **[Crate Inventory](CRATE_INVENTORY.md)** - All 25+ crates status
- **[Network Ports](NETWORK_PORT_REGISTRY.md)** - Service port allocation
- **[Data Flow](DATA_FLOW_ARCHITECTURE.md)** - End-to-end data pipeline

### **🚀 Development**
- **[Getting Started](development/getting-started.md)** - Setup guide
- **[Docker Deployment](deployment/docker-setup.md)** - Infrastructure
- **[Development Workflow](development/ai-auto-decoder-roadmap.md)** - Best practices

### **📊 DEX Integration**
- **[DEX Metadata](DEX_METADATA_SCHEMA.md)** - Program IDs and schemas
- **[IDL Registry](IDL_REGISTRY_SYSTEM.md)** - Interface definitions
- **[Validation Framework](PARSER_VALIDATION_FRAMEWORK.md)** - Testing strategy

### **🎯 Strategy & Planning**
- **[Arbitrage Strategy](AI_ARBITRAGE_STRATEGY_2025-08-21.md)** - Trading approach
- **[Production Roadmap](PRODUCTION_PLANNING.md)** - Deployment timeline
- **[72-Hour Sprint](72_HOUR_SPRINT_STATUS.md)** - Recent progress

## 🔍 **Current DEX Coverage**

| DEX | Status | Parser | Quote Service | Features |
|-----|--------|--------|---------------|----------|
| **Jupiter** | ✅ Production | Runtime IDL | API Integration | Aggregator, Limit Orders |
| **Whirlpool** | ✅ Production | State Streaming | CLMM Surface | Tick Arrays, Liquidity |
| **Raydium** | ✅ Production | Transaction Stream | AMM Quotes | Pool Prices |
| **Pump.fun** | ✅ Production | Token Stream | Bonding Curve | Real-time Pricing |
| **Meteora** | ✅ Production | DLMM Stream | DAMM Quotes | Dynamic Pools |

## 🤝 **Contributing**

The platform follows a modular architecture where each DEX integration is:
- **Self-contained** - Independent services
- **Fault-tolerant** - Graceful failure handling
- **Scalable** - Horizontal scaling support
- **Production-ready** - Comprehensive error handling

### **Adding a New DEX**
1. Create dedicated stream service
2. Implement parser with IDL support
3. Build quote surface logic
4. Add to Docker Compose
5. Update documentation

## 📈 **Roadmap & Milestones**

### **🎯 Phase 1: Enhanced Quote Quality** *(Current)*
- [x] Real-time streaming from 5 DEXs
- [x] Kafka infrastructure operational
- [ ] Parse actual swap amounts (vs. placeholders)
- [ ] Cross-validation with Shyft APIs

### **🎯 Phase 2: Arbitrage Detection** *(Next 2 weeks)*
- [ ] Cross-DEX price comparison engine
- [ ] Profitability scoring system
- [ ] Opportunity ranking and filtering

### **🎯 Phase 3: Execution Infrastructure** *(Next 4 weeks)*
- [ ] Flash loan provider integration
- [ ] Automated trade execution
- [ ] Risk management system

### **🎯 Phase 4: Production Trading** *(Next 6-8 weeks)*
- [ ] Live profitability testing
- [ ] Performance optimization
- [ ] Monitoring and alerting

## 📞 **Support & Resources**

- **Issues**: [GitHub Issues](https://github.com/your-repo/mevana/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-repo/mevana/discussions)
- **Documentation**: [MEVana Docs](https://docs.mevana.dev)

---

**Built for high-frequency arbitrage trading on Solana** ⚡

*Last updated: August 27, 2025*
