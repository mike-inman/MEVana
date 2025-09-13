# MEVana API Reference

**Document Version:** 2.0  
**Last Updated:** September 12, 2025  
**API Status:** ✅ Verified against current codebase  
**Base URL:** `http://localhost:8088` (HotPath Engine)

---

## 🔍 API Overview

MEVana exposes multiple HTTP APIs for monitoring, configuration, and system management. All APIs are RESTful and return JSON responses unless otherwise specified.

### **Service Endpoints Summary**
| Service | Base URL | Purpose |
|---------|----------|---------|
| **HotPath Engine** | `localhost:8088` | Core arbitrage engine APIs |
| **Smart Filter Config** | `localhost:8088/api/v1` | Real-time configuration management |
| **History API** | `localhost:8088/api` | Historical data and analytics |
| **Monitoring** | `localhost:8091` | Performance monitoring (separate service) |

---

## 🚀 HotPath Arbitrage Engine API

**Base URL**: `http://localhost:8088`

### **System Monitoring**

#### **GET /metrics**
Prometheus metrics endpoint for system monitoring.

**Response Format**: Prometheus text format
```
# HELP opportunities_detected Total arbitrage opportunities detected
# TYPE opportunities_detected counter
opportunities_detected 1247

# HELP processing_latency_us Processing latency in microseconds  
# TYPE processing_latency_us histogram
processing_latency_us_bucket{le="500"} 892
processing_latency_us_bucket{le="1000"} 1156
```

#### **GET /performance**
Comprehensive human-readable performance report.

**Response Format**: HTML
```html
<!DOCTYPE html>
<html>
<head><title>MEVana HotPath Performance Report</title></head>
<body>
  <h1>🚀 MEVana HotPath Arbitrage Engine Performance Report</h1>
  <h2>📊 Real-Time Processing Performance</h2>
  <!-- Detailed performance metrics in HTML format -->
</body>
</html>
```

#### **GET /performance/json**
Machine-readable performance metrics in JSON format.

**Response Format**: JSON
```json
{
  "timestamp": "2025-09-12T21:47:00Z",
  "system_health": {
    "uptime_seconds": 123456,
    "memory_usage_mb": 256.7,
    "cpu_usage_percent": 15.2
  },
  "stream_performance": {
    "stream_1": {
      "messages_per_minute": 1247.5,
      "parsed_per_minute": 891.2,
      "health_status": "healthy"
    }
  },
  "arbitrage_metrics": {
    "opportunities_detected": 291,
    "detection_rate_per_minute": 42.3,
    "avg_processing_latency_us": 456
  }
}
```

---

## 🤖 AI & Strategy Management

### **GET /ai/config**
Current AI strategy engine configuration.

**Response Format**: JSON
```json
{
  "enabled": true,
  "learning_rate": 0.1,
  "strategies": ["BlitzkriegArb", "StealthMode", "PrecisionSniper"],
  "current_strategy": "StealthMode",
  "market_conditions": ["HighVolatility", "LowLiquidity"]
}
```

### **GET /ai/prediction**
Current AI predictions and confidence levels.

**Response Format**: JSON
```json
{
  "timestamp": "2025-09-12T21:47:00Z",
  "predictions": {
    "next_opportunity_likelihood": 0.78,
    "optimal_strategy": "BlitzkriegArb",
    "confidence_score": 0.85,
    "market_conditions": {
      "volatility": 0.67,
      "liquidity_score": 0.43
    }
  }
}
```

### **GET /ai/optimization**
AI optimization recommendations.

**Response Format**: JSON
```json
{
  "recommendations": [
    {
      "parameter": "min_profit_bps",
      "current_value": 5,
      "recommended_value": 8,
      "reason": "Market volatility increased",
      "confidence": 0.82
    }
  ],
  "strategy_performance": {
    "BlitzkriegArb": 0.73,
    "StealthMode": 0.89,
    "PrecisionSniper": 0.61
  }
}
```

### **GET /ai/training**
AI training status and learning progress.

**Response Format**: JSON
```json
{
  "training_status": "active",
  "samples_processed": 1247,
  "q_table_size": 8934,
  "learning_rate": 0.1,
  "last_update": "2025-09-12T21:46:30Z",
  "performance_improvement": 0.15
}
```

---

## 🔐 Wallet Management API

### **GET /wallet/security**
Wallet security status and health check.

**Response Format**: JSON
```json
{
  "security_status": "healthy",
  "wallets": {
    "hot_wallets": 2,
    "warm_wallets": 1,
    "cold_wallets": 1
  },
  "security_metrics": {
    "encryption_enabled": true,
    "access_attempts": 0,
    "last_audit": "2025-09-12T20:00:00Z"
  }
}
```

### **GET /wallet/audit**
Comprehensive wallet security audit report.

**Response Format**: JSON
```json
{
  "audit_timestamp": "2025-09-12T21:47:00Z",
  "security_score": 95,
  "findings": [],
  "wallet_details": [
    {
      "type": "hot",
      "address": "AocM7RGckY8Pas5vPFvsZMTUWQbrW5JmvprtkVLkArKe",
      "balance_sol": 0.05,
      "limit_sol": 0.1,
      "status": "active"
    }
  ],
  "recommendations": []
}
```

### **POST /wallet/generate**
Generate new wallet with specified tier.

**Request Body**:
```json
{
  "wallet_tier": "hot",
  "limit_sol": 0.1,
  "label": "trading-wallet-3"
}
```

**Response**:
```json
{
  "success": true,
  "wallet": {
    "address": "NewWalletAddressHere",
    "tier": "hot",
    "limit_sol": 0.1,
    "created_at": "2025-09-12T21:47:00Z"
  }
}
```

### **GET /wallet/list**
List all managed wallets with status.

**Response Format**: JSON
```json
{
  "wallets": [
    {
      "address": "AocM7RGckY8Pas5vPFvsZMTUWQbrW5JmvprtkVLkArKe",
      "tier": "hot",
      "balance_sol": 0.045,
      "limit_sol": 0.1,
      "status": "active",
      "last_used": "2025-09-12T21:40:00Z"
    }
  ],
  "summary": {
    "total_wallets": 4,
    "total_balance_sol": 12.195,
    "active_wallets": 2
  }
}
```

---

## ⚙️ Smart Filter Configuration API

**Base URL**: `http://localhost:8088/api/v1`

### **GET /smart-filter/config**
Get current smart filter configuration.

**Response Format**: JSON
```json
{
  "min_profit_bps": 5,
  "max_profit_bps": 100,
  "min_net_profit_sol": 0.001,
  "min_venue_success_rate": 0.0,
  "max_gas_budget_pct": 30.0,
  "max_concurrent_opportunities": 20,
  "competition_lookback_ms": 1000,
  "learning_rate": 0.1,
  "min_samples_for_learning": 10
}
```

### **PUT /smart-filter/config**
Update smart filter configuration parameters.

**Request Body**:
```json
{
  "parameter": "min_profit_bps",
  "value": 8,
  "applied_by": "operator"
}
```

**Response**:
```json
{
  "success": true,
  "parameter": "min_profit_bps",
  "old_value": 5,
  "new_value": 8,
  "applied_at": "2025-09-12T21:47:00Z",
  "propagation_time_ms": 23
}
```

### **GET /smart-filter/stats**
Smart filter performance statistics.

**Response Format**: JSON
```json
{
  "opportunities_filtered": 1247,
  "opportunities_passed": 291,
  "filter_rate": 0.233,
  "avg_profit_bps": 12.7,
  "venue_performance": {
    "raydium->orca": {
      "success_rate": 0.78,
      "attempts": 45
    }
  }
}
```

### **GET /smart-filter/performance**
Performance impact analysis of configuration changes.

**Response Format**: JSON
```json
{
  "impact_analysis": {
    "parameter_changes": [
      {
        "parameter": "min_profit_bps",
        "timestamp": "2025-09-12T21:30:00Z",
        "before_value": 5,
        "after_value": 8,
        "opportunities_before": 42.3,
        "opportunities_after": 38.7,
        "impact_percentage": -8.5
      }
    ]
  }
}
```

### **PUT /smart-filter/reset**
Reset smart filter configuration to defaults.

**Request Body**:
```json
{
  "applied_by": "admin",
  "reason": "system_reset"
}
```

**Response**:
```json
{
  "success": true,
  "reset_at": "2025-09-12T21:47:00Z",
  "parameters_reset": 9
}
```

---

## 📊 Historical Data & Analytics API

**Base URL**: `http://localhost:8088/api`

### **GET /arbitrage/opportunities**
Retrieve historical arbitrage opportunities.

**Query Parameters**:
- `limit`: Number of records (default: 100, max: 1000)
- `offset`: Record offset for pagination
- `from`: Start timestamp (ISO 8601)
- `to`: End timestamp (ISO 8601)
- `venue`: Filter by venue pair (e.g., "raydium->orca")

**Response Format**: JSON
```json
{
  "opportunities": [
    {
      "id": "opp_123456",
      "timestamp": "2025-09-12T21:45:00Z",
      "venue_sell": "raydium",
      "venue_buy": "orca", 
      "token_pair": "SOL/USDC",
      "profit_bps": 12,
      "amount_sol": 1.5,
      "estimated_profit_sol": 0.018,
      "status": "detected",
      "processing_time_us": 456
    }
  ],
  "pagination": {
    "total": 13980,
    "limit": 100,
    "offset": 0
  }
}
```

### **GET /arbitrage/performance**
Venue performance analytics.

**Response Format**: JSON
```json
{
  "venue_performance": {
    "raydium->orca": {
      "opportunities": 234,
      "success_rate": 0.78,
      "avg_profit_bps": 14.2,
      "total_volume_sol": 456.7
    }
  },
  "time_range": {
    "from": "2025-09-11T00:00:00Z",
    "to": "2025-09-12T21:47:00Z"
  }
}
```

### **GET /arbitrage/live**
Real-time arbitrage statistics.

**Response Format**: JSON
```json
{
  "live_stats": {
    "opportunities_last_hour": 42,
    "avg_detection_latency_us": 567,
    "active_venues": ["raydium", "orca", "meteora", "jupiter", "phoenix"],
    "current_detection_rate": 38.5
  }
}
```

### **GET /arbitrage/summary**
High-level arbitrage summary statistics.

**Response Format**: JSON
```json
{
  "summary": {
    "total_opportunities": 13980,
    "total_volume_sol": 12456.78,
    "avg_profit_bps": 11.4,
    "top_venue_pair": "raydium->orca",
    "system_uptime_hours": 156.7
  }
}
```

### **GET /health**
API health check endpoint.

**Response Format**: JSON
```json
{
  "status": "healthy",
  "timestamp": "2025-09-12T21:47:00Z",
  "services": {
    "database": "connected",
    "geyser_stream": "active",
    "processing_pipeline": "operational"
  }
}
```

---

## 📈 External Monitoring Service API

**Base URL**: `http://localhost:8091` (Separate monitoring service)

### **GET /health**
Monitoring service health check.

**Response Format**: JSON
```json
{
  "status": "healthy",
  "monitoring_active": true,
  "services_monitored": 5,
  "alerts_active": 0
}
```

---

## 🔧 Response Formats & Status Codes

### **Standard HTTP Status Codes**
- `200 OK`: Successful request
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Authentication required
- `404 Not Found`: Endpoint not found
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error
- `503 Service Unavailable`: Service temporarily unavailable

### **Standard Error Response**
```json
{
  "error": {
    "code": "INVALID_PARAMETER",
    "message": "Parameter 'min_profit_bps' must be between 1 and 1000",
    "details": {
      "parameter": "min_profit_bps",
      "provided_value": -5,
      "valid_range": "1-1000"
    }
  },
  "timestamp": "2025-09-12T21:47:00Z"
}
```

---

## 🔐 Authentication & Security

### **Current Status**
**⚠️ Development Mode**: APIs currently run without authentication for development/testing.

### **Production Requirements** 
For production deployment, implement:
- **API Key Authentication**: Bearer token for external access
- **Rate Limiting**: 100 requests/minute per client
- **IP Whitelisting**: Restrict access to known IPs
- **SSL/TLS**: HTTPS only for all endpoints

---

## 📞 Support & Documentation

### **Testing Endpoints**
```bash
# Test HotPath engine
curl http://localhost:8088/performance

# Test smart filter config
curl http://localhost:8088/api/v1/smart-filter/config

# Test historical data
curl "http://localhost:8088/api/arbitrage/opportunities?limit=5"

# Test AI status
curl http://localhost:8088/ai/config
```

### **Related Documentation**
- **[System Overview](./system-overview.md)** - Architecture context
- **[Production Deployment](../deployment/production-deployment.md)** - Setup instructions
- **[Monitoring Guide](../operations/monitoring-alerting.md)** - Using these APIs for monitoring

---

*This API reference reflects the current verified implementation as of September 12, 2025. All endpoints have been validated against the actual codebase.*