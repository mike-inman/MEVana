# Successful MEVana Deployment Configuration
**Date**: 2025-09-19
**Status**: ✅ WORKING (Hotpath + Normalizer healthy, Grafana accessible)

## Working Services ✅
- **mevana-hotpath**: 4+ hours, healthy, metrics on 8088
- **mevana-normalizer**: 4+ hours, healthy
- **kafka-service**: healthy, ports 9092/29092
- **grafana-service**: healthy, port 3000
- **zookeeper-service**: healthy
- **kafka-ui-service**: healthy, port 8080
- **adminer-service**: healthy, port 8082

## Failing Services ❌
- **mevana-discovery**: Restarting (1) - needs investigation
- **mevana-executor**: Restarting (101) - needs investigation

## Working Configuration
- **GitHub Actions Workflow**: `self-hosted-deploy.yml`
- **Docker Compose**: `infrastructure/docker-compose.yml` (NOT docker-compose.server.yml)
- **Ports**: Standard ports (3000, 9090) - NOT custom ports (3300, 9091)
- **Network**: `mevana_net` (external)

## Dashboard Issues
- **Problem**: Multiple broken dashboards deployed to wrong location
- **Working Dashboard**: `infrastructure/grafana/dashboards/curated/mevana-system-status.json` (LOCAL ONLY)
- **Current Location**: Dashboards in `infrastructure/grafana/dashboards/` (wrong)
- **Correct Location**: Should be in `infrastructure/grafana/dashboards/curated/`

## Immediate Actions Needed
1. Clean up broken dashboards
2. Deploy only working dashboard
3. Fix discovery/executor service failures
4. Set up isolated dashboard development workflow