# MEVana Architecture Overview
Last updated: 2025-09-15

## 🏗️ System Architecture

MEVana is a comprehensive Solana AI trading platform designed to capture MEV (Maximal Extractable Value) opportunities through real-time blockchain data streaming, intelligent transaction analysis, and automated arbitrage execution.

## 📊 High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           MEVana Platform                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐        │
│  │   Data Sources  │    │  Processing     │    │   Execution     │        │
│  │                 │    │   Pipeline      │    │    Engine       │        │
│  │ • Solana Geyser │───▶│ • Normalizers   │───▶│ • Arbitrage     │        │
│  │ • DEX APIs      │    │ • Aggregators   │    │ • MEV Capture   │        │
│  │ • Price Feeds   │    │ • Analytics     │    │ • Risk Mgmt     │        │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘        │
│           │                       │                       │                │
│           ▼                       ▼                       ▼                │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                    Infrastructure Layer                              │  │
│  │  • Kafka (Message Queue)  • Redis (Cache)  • PostgreSQL (DB)       │  │
│  │  • Prometheus (Metrics)   • Grafana (Dashboards)                   │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🔄 Data Flow Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Solana     │───▶│  DEX        │───▶│  Normalizer │───▶│  Price      │
│  Geyser     │    │  Stream     │    │  Service    │    │  Aggregator │
│             │    │  (Orca)     │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
       │                   │                   │                   │
       ▼                   ▼                   ▼                   ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Kafka      │    │  Kafka      │    │  Kafka      │    │  Kafka      │
│  Topics     │    │  Topics     │    │  Topics     │    │  Topics     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
       │                   │                   │                   │
       └───────────────────┼───────────────────┼───────────────────┘
                           ▼
                    ┌─────────────┐
                    │  Arbitrage  │
                    │   Engine    │
                    └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Execution  │
                    │   Service   │
                    └─────────────┘
```

## 🧩 Crate Architecture & Dependencies

### Core Crates

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Workspace Root                                 │
│                              (Cargo.toml)                                  │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌────────────────┼────────────────┐
                    │                │                │
                    ▼                ▼                ▼
        ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
        │   Streaming     │ │   Normalizer    │ │   Shared        │
        │   Services      │ │   Services      │ │   Libraries     │
        └─────────────────┘ └─────────────────┘ └─────────────────┘
                    │                │                │
        ┌───────────┼───────────┐    │        ┌───────┼───────┐
        │           │           │    │        │       │       │
        ▼           ▼           ▼    ▼        ▼       ▼       ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│dex-stream-  │ │dex-backfill │ │graphql-     │ │normalizer-  │ │normalizer-  │
│orca         │ │             │ │backfill     │ │service      │ │runtime      │
└─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘
        │                │                │                │                │
        ▼                ▼                ▼                ▼                ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│normalizer-  │ │price-       │ │ops-tools    │ │shared-core  │ │shared-redis │
│orca         │ │aggregator   │ │             │ │             │ │             │
└─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘
```

## 📋 Detailed Crate Analysis

### 1. **dex-stream-orca** - Orca DEX Transaction Streaming
**Purpose**: Real-time streaming of Orca DEX transactions from Solana blockchain
**Key Features**:
- Yellowstone gRPC client for Solana Geyser integration
- Whirlpool interface parsing
- Kafka producer for downstream processing
- Transaction decoding and filtering

**Dependencies**:
- `yellowstone-grpc-client` (v4.2.1) - Solana streaming
- `rdkafka` - Message streaming
- `whirlpool_interface` - Local parser
- Solana SDK ecosystem

**Data Flow**:
```
Solana Geyser → Yellowstone gRPC → Transaction Decoding → Kafka Topics
```

### 2. **normalizer-orca** - Orca Transaction Normalization
**Purpose**: Normalize and standardize Orca DEX transaction data
**Key Features**:
- Consumes from Kafka topics
- Decodes transaction data using Yellowstone proto types
- Normalizes to standard format
- Produces normalized data to Kafka

**Dependencies**:
- `yellowstone-grpc-proto` (v4.2.1) - Message decoding
- `rdkafka` - Message consumption/production
- Solana SDK for account interpretation

**Data Flow**:
```
Kafka Input → Transaction Decoding → Normalization → Kafka Output
```

### 3. **price-aggregator** - Price Data Aggregation
**Purpose**: Aggregate and normalize price data from multiple sources
**Key Features**:
- Multi-source price feed aggregation
- Price normalization and validation
- Real-time price updates
- Kafka integration for distribution

**Dependencies**:
- `rdkafka` - Message streaming
- `tokio` - Async runtime
- `serde` - Data serialization

**Data Flow**:
```
Price Sources → Aggregation → Normalization → Kafka Distribution
```

### 4. **normalizer-service** - Core Normalization Engine
**Purpose**: Central service for IDL-based code generation and transaction parsing
**Key Features**:
- Build-time code generation
- Profile-based IDL matching
- Runtime transaction parsing
- Extensible architecture

**Dependencies**:
- `normalizer-codegen` - Build-time generation
- `normalizer-runtime` - Runtime support
- `once_cell` - Lazy initialization

**Data Flow**:
```
Profiles + IDLs → Code Generation → Runtime Parsing → Normalized Output
```

### 5. **normalizer-codegen** - Code Generation Engine
**Purpose**: Generate Rust code from IDL definitions and profiles
**Key Features**:
- YAML profile parsing
- IDL JSON processing
- Rust code generation
- Template-based rendering

**Dependencies**:
- `serde_yaml` - Profile parsing
- `regex` - Pattern matching
- `sha2` - Hash generation

**Data Flow**:
```
Profiles (YAML) + IDLs (JSON) → Code Generation → Rust Modules
```

### 6. **normalizer-runtime** - Runtime Support Library
**Purpose**: Runtime support for normalizer services
**Key Features**:
- Common data structures
- Error handling
- Serialization support
- Trait definitions

**Dependencies**:
- `serde` - Serialization
- `thiserror` - Error handling
- `anyhow` - Error propagation

### 7. **shared-core** - Common Utilities
**Purpose**: Shared utilities and common functionality
**Key Features**:
- Hash functions
- Common data types
- Utility functions

**Dependencies**:
- `sha2` - Cryptographic hashing

### 8. **shared-redis** - Redis Integration
**Purpose**: Redis client and utilities
**Key Features**:
- Redis connection management
- Common Redis operations
- Serialization helpers

**Dependencies**:
- Minimal dependencies (likely Redis client)

### 9. **ops-tools** - Operational Tools
**Purpose**: Command-line tools and utilities
**Key Features**:
- Multiple binary targets
- GraphQL backfill tools
- Operational utilities

**Dependencies**:
- Workspace dependencies
- Multiple utility libraries

### 10. **dex-backfill** - Historical Data Processing
**Purpose**: Process historical DEX transaction data
**Key Features**:
- Historical data ingestion
- Whirlpool interface parsing
- Redis integration
- Batch processing

**Dependencies**:
- `yellowstone-grpc-client` (v4.0.0)
- `whirlpool_interface` - Local parser
- `redis` - Data storage

### 11. **graphql-backfill** - GraphQL Data Processing
**Purpose**: GraphQL-based data backfilling
**Key Features**:
- GraphQL client
- Data backfilling
- LRU caching
- Redis integration

**Dependencies**:
- `reqwest` - HTTP client
- `redis` - Data storage
- `lru` - Caching

## 🔧 Infrastructure Components

### Docker Services
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Docker Infrastructure                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │  Zookeeper  │    │    Kafka    │    │   Kafka UI  │    │   Redis     │  │
│  │             │    │             │    │             │    │             │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│           │                   │                   │                   │    │
│           └───────────────────┼───────────────────┼───────────────────┘    │
│                               │                   │                        │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │ PostgreSQL  │    │   Adminer   │    │ Prometheus  │    │   Grafana   │  │
│  │             │    │             │    │             │    │             │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Kafka Topics Architecture
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Kafka Topics                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │   Raw       │    │ Normalized  │    │   Price     │    │   Arbitrage │  │
│  │Transactions │    │Transactions │    │   Data      │    │Opportunities│  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
│           │                   │                   │                   │    │
│           ▼                   ▼                   ▼                   ▼    │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │  dex-stream │    │ normalizer  │    │price-       │    │ arbitrage   │  │
│  │  -orca      │    │ -orca       │    │aggregator   │    │ engine      │  │
│  └─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 🎯 System Integration Points

### 1. **Data Ingestion Layer**
- **Solana Geyser**: Real-time blockchain data
- **DEX APIs**: Historical and real-time DEX data
- **Price Feeds**: External price sources

### 2. **Processing Layer**
- **Streaming Services**: Real-time data processing
- **Normalizer Services**: Data standardization
- **Aggregation Services**: Data consolidation

### 3. **Storage Layer**
- **Kafka**: Message queuing and streaming
- **Redis**: Caching and real-time data
- **PostgreSQL**: Persistent storage and analytics

### 4. **Execution Layer**
- **Arbitrage Engine**: MEV opportunity detection
- **Risk Management**: Position sizing and limits
- **Execution Service**: Order placement and management

## 🚀 Development Workflow

### 1. **Local Development**
```bash
# Build entire workspace
cargo build --workspace

# Run specific service
cargo run -p dex-stream-orca
cargo run -p normalizer-orca
cargo run -p price-aggregator

# Run tests
cargo test --workspace
```

### 2. **Infrastructure Setup**
```bash
# Start infrastructure services
cd infrastructure
docker-compose up -d

# Create Kafka topics
./create-topics.sh

# Verify services
./test-heartbeats.sh
```

### 3. **Code Generation**
```bash
# Normalizer service builds automatically
cargo build -p normalizer-service

# Check generated code
ls target/debug/build/normalizer-service-*/out/
```

## 🔒 Security & Configuration

### Environment Variables
- **GEYSER_ENDPOINT**: Solana Geyser endpoint
- **GEYSER_X_TOKEN**: Authentication token
- **KAFKA_BROKERS**: Kafka broker list
- **REDIS_URL**: Redis connection string
- **DATABASE_URL**: PostgreSQL connection string

### Configuration Files
- **profiles/**: YAML configuration for different DEX protocols
- **idls/**: JSON interface definitions for Solana programs
- **docker-compose.yml**: Infrastructure configuration

## 📈 Monitoring & Observability

### Metrics Collection
- **Prometheus**: System metrics and business KPIs
- **Grafana**: Dashboards and visualization
- **Custom Metrics**: Transaction volume, latency, success rates

### Logging
- **Structured Logging**: JSON-formatted logs
- **Log Levels**: DEBUG, INFO, WARN, ERROR
- **Centralized Logging**: Aggregated log collection

## 🔮 Future Roadmap

### Phase 1: Core Infrastructure ✅
- [x] Basic streaming services
- [x] Normalizer framework
- [x] Infrastructure setup

### Phase 2: Enhanced Processing 🚧
- [ ] Advanced analytics
- [ ] Machine learning integration
- [ ] Risk management system

### Phase 3: Execution Engine 📋
- [ ] Automated arbitrage execution
- [ ] Position management
- [ ] Performance optimization

### Phase 4: Advanced Features 📋
- [ ] Multi-DEX support
- [ ] Advanced MEV strategies
- [ ] Institutional features

## 🚨 Critical Notes

### 1. **Dependency Management**
- All crates use compatible Solana SDK versions (2.1.21)
- Yellowstone gRPC packages are version-locked for stability
- Workspace dependencies are properly configured

### 2. **Build Process**
- Normalizer service requires build-time code generation
- All crates build successfully with current configuration
- Workspace builds are tested and verified

### 3. **Data Flow**
- Kafka topics must be created before running services
- Redis must be available for caching services
- PostgreSQL must be initialized with proper schema

### 4. **Configuration**
- Environment variables must be set for production
- Profile files must match IDL definitions
- Infrastructure services must be running

---

**Last Updated**: August 14, 2025  
**Version**: 1.0.0  
**Status**: Production Ready - Core Infrastructure Complete
