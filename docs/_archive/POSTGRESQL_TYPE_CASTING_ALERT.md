# 🚨 CRITICAL: PostgreSQL Type Casting Issues in Monitoring APIs

## Problem
PostgreSQL `AVG()` function on `BIGINT` columns returns `NUMERIC` type, but Rust SQLx expects `f64` (DOUBLE PRECISION).

## Symptoms
- APIs return zeros instead of real data
- Service panics with: "mismatched types; Rust type `f64` (as SQL type `FLOAT8`) is not compatible with SQL type `NUMERIC`"

## Solution
**ALWAYS cast PostgreSQL aggregates to DOUBLE PRECISION in SQL queries:**

```sql
-- ❌ WRONG - Causes panic
COALESCE(AVG(detection_time_us), 0) as avg_detection_time_us,

-- ✅ CORRECT - Cast to DOUBLE PRECISION  
COALESCE(AVG(detection_time_us)::DOUBLE PRECISION, 0) as avg_detection_time_us,
```

## Affected Tables
- `arbitrage_opportunities` - ✅ FIXED
- `execution_results` - ⚠️  NEEDS CHECKING
- `orderbook_snapshots` - ⚠️  NEEDS CHECKING  
- `venue_performance` - ⚠️  NEEDS CHECKING

## For Claude Code
When working with PostgreSQL AVG/SUM aggregates in Rust SQLx:
1. **ALWAYS** cast numeric aggregates with `::DOUBLE PRECISION` 
2. Check ALL APIs that use AVG(), SUM(), or mathematical functions
3. Test with real database data, not just schema validation
4. Look for "mismatched types NUMERIC vs f64" errors in logs

**This affects ALL monitoring APIs - check every aggregate query!**