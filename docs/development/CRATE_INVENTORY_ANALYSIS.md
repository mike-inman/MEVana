# MEVana Crates Inventory & Analysis
*Date: 2025-01-20*
*Status: Active Development - Pre-Production Testing Phase*

## 🏗️ **CORE ARCHITECTURE CRATES**

### **Data Ingestion & Processing**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `dex-stream-universal` | **PRIMARY DATA ORIGIN** - Geyser stream ingestion | ✅ Production Ready | Sub-10ms latency, Kafka output, Prometheus metrics |
| `normalizer-service` | **TRANSACTION PARSER** - Multi-DEX instruction decoder | ✅ Production Ready | 10 DEX plugins, standardized output, Redis caching |
| `normalizer-runtime` | **PARSER FRAMEWORK** - Common traits and interfaces | ✅ Stable | SwapExtractor trait, normalized data structures |
| `normalizer-codegen` | **CODE GENERATOR** - Auto-generates DEX parsers | ✅ Stable | IDL → Rust code, profile-driven generation |
| `normalizer-generated` | **GENERATED PARSERS** - Auto-generated DEX plugins | ✅ Active | 10 DEX plugins, registry system |
| `mevana-idl-registry` | **IDL MANAGER** - Automated IDL fetching/updating | ✅ Production Ready | 80% auto-fetch success, fallback strategies |

### **Arbitrage Detection & Execution**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `edge-assembler` | **LIQUIDITY GRAPH** - Builds real-time price edges | 🟡 Active Development | CLMM quoter integration, edge delta computation |
| `searcher` | **OPPORTUNITY FINDER** - Graph-based arbitrage detection | 🟡 Active Development | Multi-hop paths, risk assessment, protobuf messaging |
| `executor` | **TRADE EXECUTOR** - Executes profitable trades | 🟡 Active Development | Flash loan integration, Jito bundle submission |
| `arb-test-mode` | **TESTING HARNESS** - Safe arbitrage testing environment | ✅ Ready | Kafka monitoring, metrics collection, dry-run mode |

---

## 🔧 **SPECIALIZED SERVICES**

### **Data Enhancement & Validation**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `shyft-validator` | **DATA VALIDATION** - Compares parsed data vs Shyft APIs | ✅ Production Ready | Accuracy verification, validation reporting |
| `shyft-pool-depth` | **LIQUIDITY DEPTH** - Pool depth analysis via Shyft | ✅ Production Ready | Depth tracking, liquidity monitoring |
| `shyft-transaction-fetcher` | **TRANSACTION SYNC** - Fetches transaction details | ✅ Production Ready | RPC fallback, transaction enrichment |
| `dex-backfill` | **HISTORICAL DATA** - Backfills missing pool states | ✅ Production Ready | Geyser streaming, TickArray debugging |
| `graphql-backfill` | **BULK HISTORICAL** - GraphQL-based bulk data retrieval | ✅ Production Ready | Shyft GraphQL integration |

### **Registry & Discovery**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `dex-registry` | **DEX CATALOG** - Registry of DEX programs and metadata | ✅ Production Ready | SQLite storage, API endpoints, plugin discovery |
| `token-registry-service` | **TOKEN CATALOG** - Token metadata and symbol registry | 🟡 Basic Implementation | Needs metadata enrichment, logo integration |
| `discovery-service` | **PROGRAM DISCOVERY** - Discovers new DEX programs | ✅ Basic Implementation | Program ID detection, automatic registry updates |

---

## 📊 **MONITORING & OBSERVABILITY**

### **Data Processing & Aggregation**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `price-aggregator` | **PRICE FEEDS** - Aggregates prices across DEXs | ✅ Production Ready | Multi-source pricing, weighted averages |
| `shyft-raw-price-feed` | **RAW PRICING** - Direct Shyft price feeds | ✅ Production Ready | Real-time price streaming |

---

## 🌐 **USER INTERFACE & API**

### **Web Interface**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `ui-web` | **FRONTEND** - React dashboard for monitoring | ✅ Production Ready | Real-time updates, WebSocket integration, TailwindCSS |
| `ui-gateway` | **API GATEWAY** - Backend API for UI | ✅ Production Ready | WebSocket server, REST endpoints |

---

## 🧪 **DEVELOPMENT & TESTING**

### **Testing & Development Tools**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `dev-fixtures` | **TEST DATA** - Generates synthetic test data | ✅ Ready | Kafka test messages, swap simulations |
| `ops-tools` | **OPERATIONS** - Operational tools and utilities | ✅ Basic | Shyft GraphQL backfill tools |

---

## 📚 **LEGACY & SPECIALIZED**

### **Legacy Components**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `dex-stream-orca` | **LEGACY** - Orca-specific stream parser | 🔄 Superseded | Replaced by `dex-stream-universal` |
| `normalizer-orca` | **LEGACY** - Orca-specific normalizer | 🔄 Superseded | Replaced by generated plugins |
| `stream_and_parse_whirlpool_account_updates` | **LEGACY** - Whirlpool account streaming | 🔄 Superseded | Replaced by universal stream |

### **Core Libraries**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `whirlpool-core` | **CLMM MATH** - Concentrated liquidity mathematics | ✅ Production Ready | Price quoter, tick calculations, sqrt pricing |
| `shared-core` | **COMMON TYPES** - Shared data structures | ✅ Stable | Common types, utilities |
| `shared-redis` | **REDIS CLIENT** - Shared Redis client | ✅ Stable | Connection management, serialization |

### **Integration Services**
| Crate | Purpose | Status | Key Features |
|-------|---------|--------|--------------|
| `jupiter-integration` | **JUPITER API** - Jupiter aggregator integration | 🟡 Basic Implementation | Route finding, swap execution |

---

## 🎯 **ARCHITECTURE ANALYSIS**

### **✅ STRENGTHS**
- **Complete Data Pipeline**: Geyser → Normalizer → Arbitrage Detection → Execution
- **Multi-DEX Coverage**: 10 major DEXs with 98%+ volume coverage
- **Production Monitoring**: Prometheus metrics, Grafana dashboards
- **Automated IDL Management**: 80% auto-fetch success rate
- **Validation Framework**: Shyft API validation for accuracy
- **Modular Design**: Clean separation of concerns, testable components

### **🔄 OVERLAPS & REDUNDANCIES**
- **Stream Processing**: `dex-stream-orca` superseded by `dex-stream-universal`
- **Normalizers**: `normalizer-orca` superseded by generated plugins
- **Price Feeds**: Multiple price aggregation services (`price-aggregator`, `shyft-raw-price-feed`)
- **Backfill Services**: `dex-backfill` and `graphql-backfill` serve different use cases

### **🚧 GAPS & MISSING COMPONENTS**
- **Flash Loan Providers**: Multiple provider integration needed
- **Gas Fee Calculator**: Dynamic fee estimation for profitability
- **Slippage Predictor**: Advanced slippage calculation models
- **Risk Management**: Position sizing, exposure limits
- **Performance Analytics**: Trade outcome tracking, P&L analysis

---

## 📈 **CURRENT STATE SUMMARY**

**Total Crates**: 31  
**Production Ready**: 18 (58%)  
**Active Development**: 8 (26%)  
**Legacy/Superseded**: 5 (16%)  

**Critical Path**: `dex-stream-universal` → `normalizer-service` → `edge-assembler` → `searcher` → `executor`

**Ready for Testing**: ✅ Data ingestion and parsing pipeline  
**Needs Implementation**: ⚠️ Advanced arbitrage execution and risk management
