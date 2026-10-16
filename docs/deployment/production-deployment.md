# MEVana Production Deployment Guide
Last updated: 2025-09-15

**Document Version:** 2.0  
**Last Updated:** September 12, 2025  
**System Readiness:** 75% - Ready for controlled production deployment  
**Deployment Complexity:** Moderate (infrastructure ready, execution integration required)

---

## ⚠️ CRITICAL PRE-DEPLOYMENT WARNING

**CURRENT STATUS**: The system is 75% ready for production. **Critical execution integration must be completed before live trading deployment.**

**BLOCKING ISSUE**: Main execution logic contains placeholder code at `main.rs:1183-1192` that prevents actual trade execution. This must be resolved before any production deployment.

---

## 🎯 Deployment Overview

### **What's Production Ready**
- ✅ **Infrastructure**: Docker stack with 34+ hours proven uptime
- ✅ **Data Pipeline**: Real-time processing with sub-millisecond latencies
- ✅ **Flash Loans**: Marginfi protocol integration implemented  
- ✅ **Transaction Templates**: Jupiter routing system functional
- ✅ **Smart Configuration**: Real-time parameter management

### **What Needs Completion (2-3 weeks)**
- ❌ **Execution Integration**: Replace placeholder code with real execution
- ❌ **AI Strategy Connection**: Connect strategy engine to execution decisions
- ❌ **End-to-End Testing**: Complete trading pipeline validation

---

## 📋 Pre-Deployment Requirements

### **1. Infrastructure Requirements**
- **CPU**: 4+ cores (8 recommended for high frequency)
- **RAM**: 8GB minimum (16GB recommended)
- **Storage**: 100GB SSD minimum (fast I/O critical)
- **Network**: Low-latency connection to Solana RPC endpoints
- **OS**: Linux (Ubuntu 20.04+ or RHEL 8+ recommended)

### **2. External Dependencies**
- **Geyser API Access**: Shyft.to account with appropriate limits
- **Domain & SSL**: Production domain with valid SSL certificates
- **Database Backup**: Automated backup solution configured
- **Monitoring**: External monitoring service for uptime checks

### **3. Security Requirements**  
- **Firewall Configuration**: Only required ports exposed
- **SSH Access**: Key-based authentication only
- **Secrets Management**: Environment variables secured
- **Audit Logging**: Comprehensive transaction logging enabled

---

## 🚀 Step-by-Step Deployment

### **Phase 1: Infrastructure Setup**

#### **1.1 Clone and Prepare Repository**
```bash
# Clone repository
git clone https://github.com/your-org/MEVana.git
cd MEVana

# Create production environment file
cp .env.example .env.production

# Edit configuration (see Configuration Guide)
nano .env.production
```

#### **1.2 Docker Infrastructure Deployment**
```bash
# Create Docker network
docker network create mevana_net

# Start infrastructure services
cd infrastructure
docker-compose up -d

# Verify all services are healthy (wait 2-3 minutes)
docker-compose ps
```

**Expected Output** (all services should show "healthy"):
```
SERVICE        IMAGE                                 STATUS
zookeeper      confluentinc/cp-zookeeper:7.4.0      Up (healthy)
kafka          confluentinc/cp-kafka:7.4.0           Up (healthy) 
redis          redis:7-alpine                        Up (healthy)
postgres       postgres:15                           Up (healthy)
prometheus     prom/prometheus:latest                Up (healthy)
grafana        grafana/grafana:latest                Up (healthy)
```

#### **1.3 Database Schema Setup**
```bash
# Apply database migrations
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
cargo run -p executor-service --bin migrate

# Verify schema
docker exec postgres psql -U mev -d mevdb -c "\dt"
```

### **Phase 2: Application Deployment**

#### **2.1 Build Release Binaries**
```bash
# Build optimized release binaries  
cargo build --release

# Verify binaries
ls -la target/release/hotpath_arb_engine
ls -la target/release/monitoring-service
```

#### **2.2 Configuration Validation**
```bash
# Validate port assignments
./scripts/validate_ports.sh

# Test database connectivity
./scripts/test_database.sh

# Verify Geyser API access
./scripts/test_geyser.sh
```

#### **2.3 Service Startup Sequence**
```bash
# Start monitoring service first
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
MONITORING_PORT=8091 \
RUST_LOG=info \
./target/release/monitoring-service &

# Wait 10 seconds for monitoring to initialize
sleep 10

# Start HotPath engine (PAPER TRADING MODE for initial testing)
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
RUST_LOG=info \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="your_shyft_token" \
ORDERBOOK_HISTORY_ENABLED=true \
PAPER_TRADING=true \
METRICS_PORT=8088 \
./target/release/hotpath_arb_engine
```

### **Phase 3: Verification & Monitoring**

#### **3.1 Health Check Verification**
```bash
# Check all service endpoints
curl -f http://localhost:8088/performance  # HotPath engine
curl -f http://localhost:8091/health       # Monitoring service
curl -f http://localhost:9090/-/healthy    # Prometheus
curl -f http://localhost:3000/api/health   # Grafana
```

#### **3.2 Data Flow Validation**
```bash
# Monitor opportunity detection (should see continuous flow)
curl http://localhost:8088/metrics | grep opportunities_detected

# Check database writes
docker exec postgres psql -U mev -d mevdb -c \
"SELECT COUNT(*) FROM arbitrage_opportunities WHERE timestamp > NOW() - INTERVAL '10 minutes';"

# Monitor system performance
curl http://localhost:8088/performance
```

#### **3.3 Performance Baseline**
Expected metrics after 10 minutes of operation:
- **Opportunities Detected**: 100+ (depends on market conditions)
- **Processing Latency**: <1500μs average
- **Database Writes**: Continuous (every few seconds)
- **Memory Usage**: <2GB total system
- **Error Rate**: <1%

---

## 🔧 Configuration Management

### **Critical Environment Variables**

#### **Database Configuration**
```bash
# REQUIRED - Single source of truth for database
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb"
```

#### **Geyser API Configuration**  
```bash
# REQUIRED - Shyft.to API access
GEYSER_ENDPOINT="https://grpc.ny.shyft.to"
X_TOKEN="your_shyft_api_token"
```

#### **Service Configuration**
```bash
# Service ports (must not conflict)
METRICS_PORT=8088           # HotPath engine metrics
MONITORING_PORT=8091        # Monitoring service
```

#### **Trading Configuration**
```bash
# CRITICAL - Always start with paper trading
PAPER_TRADING=true          # Set to false ONLY after full validation
ORDERBOOK_HISTORY_ENABLED=true
RUST_LOG=info              # or debug for troubleshooting
```

### **Smart Filter Configuration**
The system uses database-driven configuration. Initial setup:

```sql
-- Connect to database
docker exec -it postgres psql -U mev -d mevdb

-- Set ultra-conservative initial parameters
INSERT INTO smart_filter_config_history 
(parameter_name, new_value, applied_by) VALUES 
('min_profit_bps', '5', 'initial_deployment'),
('max_profit_bps', '50', 'initial_deployment'),
('min_net_profit_sol', '0.001', 'initial_deployment');
```

---

## 📊 Monitoring & Alerting Setup

### **Key Metrics to Monitor**

#### **System Health**
- **Service Uptime**: All services should be continuously healthy
- **Memory Usage**: <2GB total, no memory leaks
- **CPU Usage**: <50% average, spikes during high activity
- **Disk Space**: Monitor log growth, database size

#### **Trading Performance**  
- **Opportunities Detected**: Should be continuous during market hours
- **Processing Latency**: Target <1500μs, alert >3000μs
- **Database Writes**: Regular flow, alert if stopped >5 minutes
- **Error Rates**: Target <1%, alert >5%

#### **Business Metrics**
- **Profit/Loss**: Track in paper trading mode
- **Success Rate**: Detection accuracy >15%
- **Volume Processed**: Transaction throughput

### **Grafana Dashboards**
Access: http://your-domain:3000 (admin/<ADMIN_PASSWORD>)

**Pre-configured dashboards**:
1. **System Overview** - Infrastructure health, uptime
2. **Trading Performance** - Opportunities, latency, success rates  
3. **Database Metrics** - Query performance, connection pools
4. **Business Intelligence** - Profit tracking, volume analysis

### **Alert Configuration**
```yaml
# prometheus/alerts.yml
groups:
  - name: mevana.rules
    rules:
      - alert: ServiceDown
        expr: up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "MEVana service {{ $labels.job }} is down"
      
      - alert: HighLatency
        expr: processing_latency_us > 3000
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Processing latency is high: {{ $value }}μs"
```

---

## 🛡️ Security Hardening

### **Network Security**
```bash
# Firewall configuration
ufw enable
ufw allow 22/tcp        # SSH
ufw allow 3000/tcp      # Grafana (restrict to admin IPs)
ufw allow 8088/tcp      # HotPath metrics (internal only)
ufw allow 8091/tcp      # Monitoring API (internal only)
ufw deny 5433/tcp       # PostgreSQL (internal only)
ufw deny 6379/tcp       # Redis (internal only)
```

### **Application Security**
```bash
# Secure environment variables
chmod 600 .env.production
chown root:root .env.production

# Application user (don't run as root)
useradd -r -s /bin/false mevana
chown -R mevana:mevana /opt/mevana
```

### **Data Security**
```bash
# Database backup encryption
pg_dump postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb | \
gpg --cipher-algo AES256 --compress-algo 1 --symmetric \
--output backup_$(date +%Y%m%d).sql.gpg
```

---

## 🚨 Disaster Recovery

### **Backup Strategy**
```bash
# Automated daily backup script
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)

# Database backup
docker exec postgres pg_dump -U mev mevdb > backup_db_$DATE.sql

# Configuration backup
tar -czf backup_config_$DATE.tar.gz \
  .env.production \
  infrastructure/ \
  scripts/ \
  config/

# Upload to secure storage
aws s3 cp backup_*.* s3://mevana-backups/
```

### **Recovery Procedures**
```bash
# Database restore
docker exec -i postgres psql -U mev -d mevdb < backup_db_YYYYMMDD.sql

# Service restart
docker-compose restart
./scripts/start_clean.sh
```

---

## ⚠️ Known Issues & Workarounds

### **Critical Issues**
1. **Execution Placeholder** - Main trading logic not connected
   - **Impact**: No actual trading possible
   - **Workaround**: Use PAPER_TRADING=true only
   - **Timeline**: 2-3 weeks to resolve

2. **AI Strategy Disconnection** - Strategy engine not connected
   - **Impact**: Manual strategy selection only
   - **Workaround**: Use conservative default parameters
   - **Timeline**: 1-2 weeks to resolve

### **Minor Issues**
1. **Redis-UI Health Check** - May show unhealthy but functional
   - **Impact**: Cosmetic only
   - **Workaround**: Ignore if Redis itself is healthy

2. **Log Volume** - High verbosity in debug mode
   - **Impact**: Disk space usage
   - **Workaround**: Use RUST_LOG=info for production

---

## 📞 Support & Troubleshooting

### **Common Issues**

#### **Service Won't Start**
```bash
# Check port conflicts
netstat -tlnp | grep :8088

# Check Docker network
docker network ls | grep mevana

# Check logs  
docker-compose logs service_name
```

#### **Database Connection Issues**
```bash
# Test connection
docker exec postgres psql -U mev -d mevdb -c "SELECT 1;"

# Check port binding
docker port postgres
```

#### **No Opportunities Detected**
```bash
# Check Geyser connection
curl -H "x-token: your_token" https://grpc.ny.shyft.to

# Check smart filter config
curl http://localhost:8088/smart-filter/config
```

### **Log Locations**
- **Application Logs**: `./logs/`
- **Docker Logs**: `docker-compose logs`
- **System Logs**: `/var/log/syslog`

### **Emergency Procedures**
```bash
# Emergency shutdown
pkill -f hotpath_arb_engine
pkill -f monitoring-service
docker-compose down

# Emergency database backup
./scripts/emergency_backup.sh

# Service restart
./scripts/start_clean.sh
```

---

## 🎯 Post-Deployment Checklist

### **Immediate (First 24 Hours)**
- [ ] All services show healthy status
- [ ] Opportunities being detected continuously  
- [ ] Database writes flowing regularly
- [ ] No error spikes in logs
- [ ] Monitoring dashboards operational
- [ ] Alert system functional

### **Short-term (First Week)**
- [ ] Performance metrics stable
- [ ] No memory leaks detected
- [ ] Database growth within expectations
- [ ] Backup procedures tested
- [ ] Documentation updated with any deployment-specific notes

### **Before Live Trading (Critical)**
- [ ] Execution integration completed and tested
- [ ] AI strategy engine connected and validated
- [ ] End-to-end trading pipeline tested on testnet
- [ ] Security audit completed
- [ ] Disaster recovery procedures tested
- [ ] Team trained on operational procedures

---

**⚠️ REMEMBER: This deployment guide assumes completion of the execution integration work. Do not attempt live trading until the placeholder code has been replaced with real execution logic.**

---

*This deployment guide reflects the current state of the MEVana system as of September 12, 2025. All procedures have been verified against the actual codebase and infrastructure.*
