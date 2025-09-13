# MEVana System Flow and Components Documentation

**Date:** 2025-09-11  
**Version:** 1.0  
**Status:** COMPREHENSIVE SYSTEM OVERVIEW

## 🏗️ System Architecture Overview

MEVana is a high-frequency Solana arbitrage platform designed for sub-millisecond MEV detection and execution. The system follows a tiered microservices architecture with specialized components for different aspects of the trading pipeline.

## 📊 Data Flow Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Geyser RPC    │────│  HotPath Engine  │────│  Smart Filter   │
│  (3 Streams)    │    │   (Port 8088)    │    │  (Real-time)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │                         │
                              ▼                         ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Event Bus     │    │  Normalizer      │    │  Execution      │
│  (Non-blocking) │    │   Service        │    │   Engine        │
└─────────────────┘    │  (Port 9901)     │    │  (Port 9904)    │
         │              └──────────────────┘    └─────────────────┘
         ▼                       │                       │
┌─────────────────┐              ▼                       ▼
│ Async History   │    ┌──────────────────┐    ┌─────────────────┐
│    Buffer       │    │  Edge Assembler  │    │    Jito         │
│   (Database)    │    │  (Port 9902)     │    │  Integration    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   PostgreSQL    │    │   Kafka Bus      │    │   Solana RPC    │
│   (Port 5433)   │    │  (Port 9092)     │    │   (External)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## 🔧 Component Analysis by Crate

### 1. Core Trading Engine Components

#### `hotpath_arb_engine` (Port 8088)
**Purpose:** Zero-latency MEV opportunity detection and filtering
**Key Features:**
- Multi-Geyser streaming (3 parallel connections to Solana)
- AI-powered opportunity scoring and prediction
- Enterprise wallet management (Hot/Warm/Cold tiers)
- Real-time smart filtering with adaptive learning
- Event bus for non-blocking database operations

**Critical Files:**
- `src/main.rs` (2800+ lines) - Core orchestration
- `src/smart_filter.rs` - Configurable opportunity filtering
- `src/wallet_security.rs` - Multi-tier wallet management
- `src/ai_strategy_engine.rs` - Neural network optimization
- `src/event_bus.rs` - Non-blocking reporting system

**Responsibilities:**
- Transaction stream monitoring
- Opportunity detection and scoring
- Risk assessment and filtering
- Performance analytics and reporting

#### `arb-engine` (Port 8085)
**Purpose:** Main arbitrage detection engine with WebSocket API
**Key Features:**
- Jupiter integration for multi-venue arbitrage
- Adaptive sizing based on historical performance
- WebSocket API for real-time updates
- Hot path signal queue for latency optimization

**Integration Points:**
- Consumed by: HotPath engine, monitoring services
- Produces to: Kafka topics, WebSocket clients
- Dependencies: Edge assembler, normalizer service

#### `executor-service` (Port 9904)
**Purpose:** Trade execution orchestration and wallet management
**Key Features:**
- Multi-wallet execution strategies
- Account registry management with versioning
- Transaction building and submission
- Flash loan integration and management

**Critical Components:**
- Bundle building with Jito optimization
- Account registry validation and diffing
- Execution result tracking and analysis
- Wallet security and rotation

### 2. Data Processing Pipeline

#### `normalizer-service` (Port 9901)
**Purpose:** Transaction parsing and IDL-based code generation
**Key Features:**
- Real-time transaction normalization
- IDL registry integration
- Kafka message processing
- Structured data extraction from raw transactions

**Data Flow:**
1. Receives raw transactions from Geyser streams
2. Parses using IDL definitions
3. Normalizes to standard format
4. Publishes to Kafka for downstream processing

#### `edge-assembler` (Port 9902)
**Purpose:** Real-time transaction processing and venue handling
**Key Features:**
- High-frequency transaction assembly
- Venue-specific logic handling
- Real-time edge detection
- Performance-optimized processing pipeline

**Integration:**
- Consumes: Normalized transaction data
- Produces: Assembled trading opportunities
- Dependencies: Normalizer service, shared protocol definitions

### 3. Supporting Infrastructure

#### `system-orchestrator` (Port 9999 - Reserved)
**Purpose:** Service lifecycle management and health monitoring
**Key Features:**
- Tiered startup sequence management
- Health check automation
- Graceful shutdown coordination
- Service dependency resolution

**Service Tiers:**
1. **Infrastructure** (Tier 0): Docker services
2. **Core** (Tier 1): normalizer → edge-assembler → arb-engine → executor
3. **Supporting** (Tier 2): discovery, token-registry
4. **HotPath** (Tier 3): hotpath_arb_engine
5. **Monitoring** (Tier 4): monitoring-service

#### `api-service` (Port 8092)
**Purpose:** Public REST API and external integration
**Key Features:**
- JWT authentication with Clerk integration
- Rate limiting and security middleware
- RESTful endpoints for trading data
- WebSocket support for real-time updates

**Recent Enhancements:**
- Kafka consumer integration for real-time updates
- Database logging for audit trails
- Enhanced middleware stack for production readiness

#### `monitoring-service` (Port 8091)
**Purpose:** System performance monitoring and analytics
**Key Features:**
- Real-time performance metrics collection
- Integration with HotPath engine analytics
- Prometheus metrics export
- Health check aggregation

### 4. Specialized Libraries and Utilities

#### `whirlpool-core`
**Purpose:** Orca Whirlpool protocol integration
**Key Features:**
- Pool state management and tracking
- Price calculation and slippage estimation
- Concentrated liquidity position handling
- Real-time pool monitoring

#### `shared-core`
**Purpose:** Common utilities and shared types
**Key Features:**
- Cross-service data structures
- Utility functions and helpers
- Common error handling patterns
- Shared configuration management

#### `resilience-framework`
**Purpose:** Error handling, retry logic, and system resilience
**Key Features:**
- Circuit breaker patterns
- Retry mechanisms with backoff
- Database connection management
- Graceful degradation strategies

#### `discovery-service` (Port 9102)
**Purpose:** Service discovery and registration
**Key Features:**
- Dynamic service registration
- Health status propagation
- Service endpoint discovery
- Load balancing support

## 🚀 Service Startup Sequence

### Phase 1: Infrastructure (0-30 seconds)
1. **Docker Compose Up**
   - Zookeeper (2181) - Kafka coordination
   - Kafka (9092/29092) - Message broker
   - Redis (6379) - Caching layer
   - PostgreSQL (5433) - Data persistence
   - Prometheus (9090) - Metrics collection
   - Grafana (3000) - Visualization

### Phase 2: Core Services (30-60 seconds)
1. **normalizer-service** (9901) - Transaction parsing foundation
2. **edge-assembler** (9902) - Real-time processing
3. **arb-engine** (8085) - Arbitrage detection
4. **executor-service** (9904) - Trade execution

### Phase 3: Supporting Services (60-90 seconds)
1. **discovery-service** (9102) - Service registry
2. **token-registry-service** (9898) - Token metadata
3. **api-service** (8092) - External API

### Phase 4: HotPath Services (90-120 seconds)
1. **hotpath_arb_engine** (8088) - High-frequency MEV detection

### Phase 5: Monitoring (120+ seconds)
1. **monitoring-service** (8091) - Performance analytics

## 🔄 Data Processing Flow

### 1. Transaction Ingestion
```
Geyser Streams → HotPath Engine → Smart Filter → Event Bus
                      ↓               ↓           ↓
              Normalizer Service → Edge Assembler → Kafka
```

### 2. Opportunity Detection
```
Normalized Transactions → Arbitrage Engine → Opportunity Scoring
                              ↓                    ↓
                        Execution Engine ← Smart Filter Decision
```

### 3. Execution Pipeline
```
Approved Opportunity → Wallet Manager → Transaction Builder
                           ↓                 ↓
                    Jito Integration → Solana Network
                           ↓                 ↓
                    Result Tracking ← Transaction Status
```

### 4. Data Persistence
```
All Operations → Event Bus → Async History Buffer → PostgreSQL
                     ↓            ↓                    ↓
              Redis Cache → Metrics Publisher → Prometheus
```

## 📈 Performance Characteristics

### Latency Targets
- **HotPath Detection:** 400-600μs (current performance)
- **Smart Filter Processing:** < 100μs per opportunity
- **Database Write:** < 10ms P95 (non-blocking)
- **API Response:** < 50ms P95
- **WebSocket Update:** < 5ms propagation

### Throughput Capacity
- **Transaction Processing:** 10,000+ TPS
- **Opportunity Detection:** 100+ per minute
- **Concurrent Users:** 1,000+ API connections
- **Database Operations:** 50,000+ writes/minute

### Resource Utilization
- **Memory:** 4-8GB total system footprint
- **CPU:** Optimized for multi-core parallel processing  
- **Network:** High-bandwidth Geyser connections
- **Storage:** PostgreSQL with optimized indexing

## 🔍 Integration Points

### External APIs
- **Shyft Geyser:** Primary transaction stream
- **Jupiter:** Routing and price discovery
- **Jito:** Bundle optimization and MEV protection
- **Solana RPC:** Transaction submission and confirmation

### Internal Communication
- **Kafka:** Async message passing between services
- **Redis:** Fast caching and session management
- **PostgreSQL:** Persistent data storage and analytics
- **WebSocket:** Real-time client updates

## 🛡️ Security and Resilience

### Wallet Management
- **Hot Wallets:** Active trading (0.1 SOL limit each)
- **Warm Wallets:** Reserve funds (10 SOL limit)
- **Cold Wallets:** Long-term storage (unlimited)
- **HSM Integration:** Hardware security module support

### Error Handling
- **Circuit Breakers:** Database and external API protection
- **Retry Logic:** Exponential backoff for transient failures
- **Graceful Degradation:** Continued operation during partial failures
- **Health Monitoring:** Proactive issue detection and alerting

## 📝 Recent Enhancements

### Queue System Addition
- **Event Bus Implementation:** Non-blocking database operations
- **Async History Buffer:** Batched database writes
- **Performance Optimization:** Reduced hot path latency

### API Revision
- **Enhanced Middleware:** Authentication, rate limiting, CORS
- **Kafka Integration:** Real-time data streaming
- **Database Logging:** Comprehensive audit trails
- **WebSocket Support:** Real-time client updates

## 🎯 Operational Focus Areas

### Runtime Configuration
- **Smart Filter Parameters:** Currently startup-only, needs runtime adjustment
- **Port Management:** Scattered across services, needs centralization
- **Environment Configuration:** Hardcoded values need externalization

### System Visibility
- **Performance Monitoring:** Real-time parameter impact analysis needed
- **Configuration State:** Centralized management and versioning required
- **Debug Capabilities:** Enhanced logging and tracing for production

### Scalability Preparation
- **Service Discovery:** Dynamic endpoint management
- **Load Balancing:** Multi-instance deployment support
- **Resource Management:** Auto-scaling and resource optimization

---

**Next Actions:**
1. Implement runtime configuration management
2. Centralize port and environment management  
3. Enhance system visibility and monitoring
4. Optimize database performance and resilience