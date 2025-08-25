# DEX Metadata Schema v1.0

## Overview
This document defines the standardized metadata format for each DEX plugin in the MEVana arbitrage system. This metadata serves multiple purposes:

1. **Human Interface**: Rich information display in UI dashboards
2. **AI Analysis**: Structured data for trend detection and strategy optimization  
3. **Operational Intelligence**: Performance characteristics and trade-offs
4. **Strategic Planning**: Liquidity models and arbitrage opportunities

## Schema Definition

Each DEX plugin includes a `{dex_name}_metadata.json` file with the following structure:

```json
{
  "dex_info": {
    "name": "string",
    "display_name": "string", 
    "protocol_type": "AMM|CLMM|CLOB|BondingCurve|Aggregator|Hybrid",
    "program_ids": ["string"],
    "website": "string",
    "documentation": "string",
    "twitter": "string",
    "launch_date": "YYYY-MM-DD",
    "total_value_locked": "number|null",
    "market_share_percent": "number|null"
  },
  "technical_specs": {
    "pricing_model": "ConstantProduct|ConcentratedLiquidity|OrderBook|Linear|Hybrid",
    "fee_structure": {
      "trading_fee_bps": "number",
      "protocol_fee_bps": "number", 
      "lp_fee_bps": "number",
      "variable_fees": "boolean",
      "fee_tiers": ["number"]
    },
    "supported_tokens": {
      "native_tokens": ["string"],
      "supports_spl_tokens": "boolean",
      "supports_nfts": "boolean", 
      "minimum_liquidity": "number|null"
    },
    "transaction_finality": {
      "average_confirmation_time_ms": "number",
      "supports_partial_fills": "boolean",
      "mev_protection": "boolean",
      "requires_seat": "boolean"
    }
  },
  "arbitrage_profile": {
    "liquidity_depth": "Low|Medium|High|VeryHigh",
    "price_impact": "Low|Medium|High", 
    "optimal_trade_sizes": {
      "min_profitable_usd": "number",
      "max_efficient_usd": "number",
      "sweet_spot_usd": "number"
    },
    "latency_characteristics": {
      "quote_time_ms": "number",
      "execution_time_ms": "number", 
      "settlement_time_ms": "number"
    },
    "arbitrage_advantages": ["string"],
    "arbitrage_considerations": ["string"],
    "cross_dex_opportunities": ["string"]
  },
  "market_dynamics": {
    "primary_use_cases": ["string"],
    "target_audience": "Retail|Institution|MEV|Aggregators|All",
    "peak_activity_hours": "string",
    "dominant_pairs": ["string"],
    "seasonal_patterns": "string|null",
    "competitor_analysis": {
      "main_competitors": ["string"],
      "unique_advantages": ["string"],
      "market_position": "Leader|Challenger|Niche|Emerging"
    }
  },
  "integration_data": {
    "plugin_version": "string",
    "last_updated": "YYYY-MM-DD",
    "instruction_coverage": {
      "total_instructions": "number",
      "parsed_instructions": "number", 
      "coverage_percentage": "number"
    },
    "testing_status": {
      "unit_tests": "boolean",
      "integration_tests": "boolean", 
      "production_verified": "boolean"
    },
    "performance_metrics": {
      "parse_success_rate": "number",
      "average_parse_time_us": "number",
      "error_rate": "number"
    }
  },
  "ai_insights": {
    "trend_indicators": ["string"],
    "anomaly_patterns": ["string"], 
    "correlation_signals": ["string"],
    "strategic_recommendations": ["string"],
    "risk_factors": ["string"],
    "opportunity_indicators": ["string"]
  }
}
```

## Field Descriptions

### DEX Info
- **name**: Internal identifier (matches profile name)
- **display_name**: Human-readable name for UI
- **protocol_type**: Core architecture classification
- **program_ids**: All associated Solana program addresses
- **Total Value Locked**: Current TVL if available

### Technical Specs  
- **pricing_model**: How prices are determined
- **fee_structure**: Complete fee breakdown
- **transaction_finality**: Speed and reliability characteristics

### Arbitrage Profile
- **liquidity_depth**: Available trading depth
- **optimal_trade_sizes**: Profitability ranges
- **latency_characteristics**: Speed measurements
- **arbitrage_advantages**: Unique strengths
- **arbitrage_considerations**: Limitations or requirements

### Market Dynamics
- **primary_use_cases**: Main trading scenarios
- **target_audience**: User demographics
- **competitor_analysis**: Market positioning

### Integration Data
- **instruction_coverage**: Parsing completeness
- **performance_metrics**: Runtime statistics
- **testing_status**: Validation levels

### AI Insights
- **trend_indicators**: Signals for AI pattern detection
- **correlation_signals**: Cross-DEX relationships
- **strategic_recommendations**: AI-driven optimization suggestions

## Usage in UI

The metadata enables rich dashboard features:
- **DEX Comparison Matrix**: Side-by-side technical comparison
- **Arbitrage Opportunity Heatmap**: Real-time profitability by DEX pair
- **Performance Analytics**: Plugin health and effectiveness
- **Market Intelligence**: Trend analysis and predictions
- **Strategic Dashboard**: AI-recommended optimizations

## AI Analysis Applications

This structured data enables:
- **Pattern Recognition**: Cross-DEX behavior analysis
- **Predictive Modeling**: Volume and volatility forecasting  
- **Strategy Optimization**: Dynamic parameter tuning
- **Risk Assessment**: Automated risk factor monitoring
- **Opportunity Discovery**: Emerging arbitrage detection

## Maintenance

Metadata files should be updated:
- **Weekly**: Market data (TVL, volume, fees)
- **Monthly**: Performance metrics and AI insights
- **As needed**: Technical specifications and strategic analysis
- **Major releases**: Plugin versions and testing status

This system creates a comprehensive knowledge base that grows smarter over time while providing immediate operational value.
