# 🚀 MEVana Deployment Quickstart
Last updated: 2025-09-15

## Tomorrow's Vultr Deployment (30 minutes total)

### 1. Deploy Vultr High Frequency Compute Server
```bash
# Location: New York
# Instance: 8 vCPU, 32GB RAM, NVMe SSD ($192/month)
# OS: Ubuntu 22.04 LTS
```

### 2. SSH into your new server and run:
```bash
# Clone repository
git clone https://github.com/mike-inman/MEVana.git
cd MEVana

# Run automated deployment (builds everything)
sudo ./deploy.sh

# Test network latency
./scripts/testing/latency_test.sh

# Start MEVana services
./start_services.sh
```

### 3. Monitor and verify:
```bash
# Live performance monitor
./monitor.sh

# Health check
./health_check.sh

# View logs
tail -f logs/*.log
```

## Emergency Procedures

### If services crash:
```bash
./stop_services.sh
./start_services.sh
./health_check.sh
```

### If data flow stops:
```bash
# Restart infrastructure
cd infrastructure
docker-compose -f docker-compose.prod.yml restart kafka
sleep 30
cd ..
./start_services.sh
```

## Success Metrics

**Deployment Success:**
- ✅ All services running without errors
- ✅ Latency < 15ms to Solana (target: 8-12ms)
- ✅ EdgeDelta messages flowing (no "kafka_idle")

**Trading Readiness:**
- ✅ First EdgeDelta processed within 5 minutes
- ✅ Arbitrage opportunities detected
- ✅ Test mode execution ready

## Monitoring URLs
- Arbitrage Config: http://YOUR_SERVER_IP:8081/ai/config
- Grafana Dashboard: http://YOUR_SERVER_IP:3000 (admin/admin123)
- Prometheus: http://YOUR_SERVER_IP:9090

## Next Steps After Deployment
1. Add trading wallet with funds (SOL)
2. Test mode execution (4+ hours)
3. Switch to live trading
4. Weekend pipeline development

The complete deployment pipeline is now ready for tomorrow's Vultr deployment! 🎯
