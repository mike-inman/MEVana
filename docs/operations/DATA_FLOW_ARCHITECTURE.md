# MEVana Data Flow Architecture
Last updated: 2025-09-15
**Date:** 2024-12-17  
**Version:** 1.0  
**Status:** Current System Architecture

## Overview
This document outlines the complete data flow architecture for the MEVana system, tracing data from its origin (Solana Geyser stream) through processing, validation, and arbitrage detection. The system is designed for sub-10ms latency arbitrage detection with real-time validation against Shyft APIs.

## System Architecture

```mermaid
graph TB
    %% Data Origin Layer
    subgraph "Data Origin Layer"
        GEYSER[Solana Geyser Stream<br/>yellowstone-grpc-client<br/>Sub-10ms latency]
        SHYFT_API[Shyft APIs<br/>GraphQL + REST<br/>Validation & Reference Data]
    end

    %% Data Ingestion Layer
    subgraph "Data Ingestion Layer"
        DEX_STREAM_ORCA[dex-stream-orca<br/>Whirlpool-specific parser<br/>Raw transaction processing]
        DEX_STREAM_UNIVERSAL[dex-stream-universal<br/>Universal DEX parser<br/>Multi-protocol support]
        DEX_BACKFILL[dex-backfill<br/>Historical data ingestion<br/>TickArray debugging]
    end

    %% Data Processing Layer
    subgraph "Data Processing Layer"
        NORMALIZER_SERVICE[normalizer-service<br/>Data normalization<br/>Protocol-agnostic format]
        NORMALIZER_ORCA[normalizer-orca<br/>Orca-specific normalization<br/>TickArray parsing]
        NORMALIZER_GENERATED[normalizer-generated<br/>Auto-generated parsers<br/>Protocol adapters]
    end

    %% Data Storage Layer
    subgraph "Data Storage Layer"
        SHARED_REDIS[shared-redis<br/>High-speed caching<br/>Stream processing]
        SHARED_CORE[shared-core<br/>Common data structures<br/>Type definitions]
    end

    %% Validation Layer
    subgraph "Validation Layer"
        SHYFT_VALIDATOR[shyft-validator<br/>API validation<br/>Data quality checks]
        SHYFT_POOL_DEPTH[shyft-pool-depth<br/>Pool depth validation<br/>Market data verification]
        DISCOVERY_SERVICE[discovery-service<br/>Unknown program detection<br/>AI analysis queue]
    end

    %% Business Logic Layer
    subgraph "Business Logic Layer"
        ARB_TEST_MODE[arb-test-mode<br/>Arbitrage detection<br/>Opportunity scoring]
        PRICE_AGGREGATOR[price-aggregator<br/>Price calculation<br/>Market data aggregation]
        GRAPHQL_BACKFILL[graphql-backfill<br/>Historical analysis<br/>Backtesting support]
    end

    %% UI Layer
    subgraph "UI Layer"
        UI_GATEWAY[ui-gateway<br/>API gateway<br/>Service orchestration]
        UI_WEB[ui-web<br/>Web dashboard<br/>Real-time monitoring]
    end

    %% Operations Layer
    subgraph "Operations Layer"
        OPS_TOOLS[ops-tools<br/>Deployment & monitoring<br/>Service management]
        INFRASTRUCTURE[infrastructure<br/>Prometheus + Grafana<br/>Observability]
    end

    %% Data Flow Connections
    GEYSER --> DEX_STREAM_ORCA
    GEYSER --> DEX_STREAM_UNIVERSAL
    
    DEX_STREAM_ORCA --> NORMALIZER_ORCA
    DEX_STREAM_UNIVERSAL --> NORMALIZER_SERVICE
    
    NORMALIZER_ORCA --> SHARED_REDIS
    NORMALIZER_SERVICE --> SHARED_REDIS
    NORMALIZER_GENERATED --> SHARED_REDIS
    
    SHARED_REDIS --> ARB_TEST_MODE
    SHARED_REDIS --> PRICE_AGGREGATOR
    
    SHYFT_API --> SHYFT_VALIDATOR
    SHYFT_API --> SHYFT_POOL_DEPTH
    
    SHARED_REDIS --> SHYFT_VALIDATOR
    SHARED_REDIS --> SHYFT_POOL_DEPTH
    
    ARB_TEST_MODE --> UI_GATEWAY
    PRICE_AGGREGATOR --> UI_GATEWAY
    
    UI_GATEWAY --> UI_WEB
    
    %% Validation feedback loops
    SHYFT_VALIDATOR -.-> SHARED_REDIS
    SHYFT_POOL_DEPTH -.-> SHARED_REDIS
    
    %% Discovery feedback
    DEX_STREAM_UNIVERSAL --> DISCOVERY_SERVICE
    DISCOVERY_SERVICE -.-> SHARED_REDIS
    
    %% Backfill connections
    DEX_BACKFILL --> SHARED_REDIS
    GRAPHQL_BACKFILL --> SHARED_REDIS
    
    %% Operations monitoring
    ARB_TEST_MODE --> INFRASTRUCTURE
    SHARED_REDIS --> INFRASTRUCTURE
    DEX_STREAM_ORCA --> INFRASTRUCTURE
```

## Data Flow Sequence

### 1. **Data Ingestion (Sub-10ms Latency)**
- **Source**: Solana Geyser stream via `yellowstone-grpc-client`
- **Services**: `dex-stream-orca`, `dex-stream-universal`
- **Output**: Raw transaction data to Kafka topics

### 2. **Data Processing & Normalization**
- **Input**: Raw transaction data from Kafka
- **Services**: `normalizer-service`, `normalizer-orca`, `normalizer-generated`
- **Output**: Normalized swap events in protocol-agnostic format

### 3. **Data Storage & Caching**
- **Service**: `shared-redis`
- **Purpose**: High-speed caching for arbitrage detection
- **Format**: Stream-based storage for real-time processing

### 4. **Validation & Quality Assurance**
- **Services**: `shyft-validator`, `shyft-pool-depth`
- **Purpose**: Validate our parsing against Shyft APIs
- **Output**: Data quality metrics and validation results

### 5. **Arbitrage Detection**
- **Service**: `arb-test-mode`
- **Input**: Normalized swap events from Redis
- **Output**: Identified arbitrage opportunities

### 6. **Market Data & Price Calculation**
- **Service**: `price-aggregator`
- **Purpose**: Calculate accurate prices and market data
- **Input**: Pool depth data, swap events

### 7. **Discovery & AI Analysis**
- **Service**: `discovery-service`
- **Purpose**: Detect unknown programs for AI analysis
- **Output**: Queued programs for AI investigation

## Service Responsibilities

### Core Data Pipeline
- **`dex-stream-orca`**: Whirlpool-specific transaction parsing
- **`dex-stream-universal`**: Multi-protocol transaction parsing
- **`normalizer-service`**: Protocol-agnostic data normalization
- **`shared-redis`**: High-speed data caching and streaming

### Validation & Quality
- **`shyft-validator`**: API-based data validation
- **`shyFT-pool-depth`**: Pool depth validation
- **`discovery-service`**: Unknown program detection

### Business Logic
- **`arb-test-mode`**: Arbitrage opportunity detection
- **`price-aggregator`**: Price calculation and market data
- **`graphql-backfill`**: Historical data analysis

### Infrastructure
- **`ui-gateway`**: API gateway and service orchestration
- **`ops-tools`**: Deployment and monitoring
- **`infrastructure`**: Prometheus + Grafana observability

## Data Quality Assurance

### Real-time Validation
1. **Geyser Stream**: Primary data source (sub-10ms latency)
2. **Shyft APIs**: Validation and reference data
3. **Cross-validation**: Compare our parsing with Shyft results
4. **Market data**: Verify against live market prices

### Data Quality Metrics
- **Latency**: Sub-10ms from Geyser to processing
- **Accuracy**: Validated against Shyft APIs
- **Completeness**: Full transaction parsing coverage
- **Freshness**: Real-time market data updates

## Testing Strategy

### End-to-End Testing
1. **Data Pipeline Flow**: Test complete data flow from Geyser to arbitrage detection
2. **Validation Pipeline**: Test Shyft API validation accuracy
3. **Performance Testing**: Verify sub-10ms latency requirements
4. **Data Quality Testing**: Ensure accuracy and completeness

### Component Testing
1. **Parser Testing**: Test each normalizer and parser
2. **Redis Operations**: Test caching and streaming
3. **API Integration**: Test Shyft API connections
4. **Arbitrage Logic**: Test opportunity detection algorithms

## Performance Requirements

- **Latency**: Sub-10ms from Geyser to processing
- **Throughput**: Handle high-volume transaction streams
- **Reliability**: 99.9% uptime for arbitrage detection
- **Accuracy**: 99.5% data parsing accuracy

## Monitoring & Observability

- **Prometheus**: Metrics collection and alerting
- **Grafana**: Real-time dashboards and visualization
- **Custom Metrics**: Arbitrage opportunities, latency, accuracy
- **Alerting**: Data quality degradation, latency spikes

## Next Steps

1. **Implement comprehensive testing suite** for data pipeline validation
2. **Add real-time market data validation** against CoinGecko/CoinMarketCap
3. **Implement automated data quality monitoring** with alerting
4. **Add performance benchmarking** for latency and throughput
5. **Create data lineage tracking** for audit and debugging

---

**Note**: This architecture prioritizes speed (sub-10ms latency) while maintaining data quality through real-time validation against Shyft APIs. The hybrid approach ensures we get the best of both worlds: speed from Geyser streams and accuracy from API validation.
