# MEVana Service Status & Management Plan
Last updated: 2025-09-15

## 🚀 CRITICAL SERVICES (Always Required)
- **hotpath-arb-engine**: ✅ PRIMARY - Core opportunity detection engine
- **discovery-service**: ✅ RUNNING - Multiple healthy instances  
- **infrastructure**: ✅ HEALTHY - Docker containers (postgres, kafka, redis)

## 🔧 LIBRARY DEPENDENCIES (Code Dependencies, No Binary)
- **arb-engine**: Library used by hotpath-arb-engine
- **normalizer-runtime**: Library used by hotpath-arb-engine  
- **edge-assembler**: Library used by hotpath-arb-engine

## 🔄 FUTURE-READY SERVICES (Not Currently Needed, But Keep Available)
- **normalizer-service**: Data processing pipeline (may be used later)
- **executor-service**: Trade execution service (integrated into hotpath)
- **token-registry-service**: Token metadata service (may be used later)
- **monitoring-service**: Metrics collection service (nice to have)

## 📋 CURRENT STRATEGY
Instead of crashing, these services should:
1. **Start with proper error handling**
2. **Gracefully exit if not needed**  
3. **Log clear status messages**
4. **Be easily re-enabled when needed**

## ✅ RESILIENT STARTUP APPROACH
- Make services **optional but available**
- Add proper **health checks** where relevant
- Use **graceful degradation** rather than crashes
- Keep configurations ready for **future activation**
