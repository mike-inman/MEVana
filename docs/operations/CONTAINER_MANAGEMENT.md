# Container Management Guide

## Critical Issue: Container Naming Conflicts

### Problem
Multiple docker-compose deployments can create orphaned containers with different naming schemes:
- `mevana-*` containers (from old docker-compose.yml)
- `infrastructure-*` containers (from docker-compose.core.yml)

This causes:
- Port conflicts
- Duplicate services running
- Resource exhaustion
- Data inconsistencies (e.g., Kafka cluster ID mismatches)

### Solution: Standardized Deployment Process

#### 1. Clean Shutdown Before Deployment
Always stop ALL containers before deploying:

```bash
# On production server
cd /opt/MEVana/infrastructure

# Stop all MEVana-related containers
docker ps -a --filter 'name=mevana-' --format '{{.Names}}' | xargs -r docker stop
docker ps -a --filter 'name=infrastructure-' --format '{{.Names}}' | xargs -r docker stop

# Remove stopped containers
docker ps -a --filter 'name=mevana-' --format '{{.Names}}' | xargs -r docker rm
docker ps -a --filter 'name=infrastructure-' --format '{{.Names}}' | xargs -r docker rm
```

#### 2. Use Correct Docker Compose File
Production deployments MUST use `docker-compose.core.yml`:

```bash
# Correct deployment command
docker compose -f docker-compose.core.yml up -d

# Incorrect (creates wrong naming)
docker compose up -d  # Uses docker-compose.yml instead
```

#### 3. Verify No Orphaned Containers
After deployment:

```bash
# Check for any old mevana-* containers
docker ps -a --filter 'name=mevana-'

# Should return empty if deployment was clean
```

## Container Naming Standards

### Infrastructure Services (Docker)
Prefix: `infrastructure-*`

Services:
- `infrastructure-zookeeper-1`
- `infrastructure-kafka-1`
- `infrastructure-kafka-ui-1`
- `infrastructure-redis-1`
- `infrastructure-redis-ui-1`
- `infrastructure-postgres-1`
- `infrastructure-adminer-1`
- `infrastructure-prometheus-1`
- `infrastructure-grafana-1`

### Application Services (Systemd)
No containers - these run as native binaries under systemd:
- `hotpath-arb-engine.service`
- `executor-service.service`
- `normalizer-service.service`
- `discovery-service.service`
- `wallet-metrics-exporter.service`

## Port Assignments (Production)

### Infrastructure Services
- **2181**: Zookeeper (internal only)
- **29092**: Kafka (external)
- **8084**: Kafka-UI
- **6380**: Redis
- **8083**: Redis-UI
- **5433**: PostgreSQL
- **8087**: Adminer
- **9091**: Prometheus
- **3001**: Grafana

### Application Services
- **8088**: HotPath Metrics
- **9901**: Normalizer Metrics
- **9102**: Discovery Metrics
- **9904**: Executor Metrics
- **9905**: Wallet Metrics

## Volume Management

### Infrastructure Volumes
Created by `docker-compose.core.yml`:
- `infrastructure_kafka_data` - Kafka message persistence
- `infrastructure_redis_data` - Redis data persistence
- `infrastructure_postgres_data` - PostgreSQL database
- `infrastructure_prometheus_data` - Metrics history
- `infrastructure_grafana_data` - Dashboards and settings

### Old Volumes (May Need Cleanup)
Legacy volumes from previous deployments:
- `mevana_kafka_data`
- `mevana_redis_data`
- etc.

**Note**: Old volumes are NOT automatically deleted. Use `docker volume prune` with caution.

## Health Checks

### Quick Health Check Script
```bash
#!/bin/bash
# File: /opt/MEVana/scripts/health_check.sh

echo "=== Infrastructure Services ==="
docker ps --filter 'name=infrastructure-' --format 'table {{.Names}}\t{{.Status}}'

echo -e "\n=== Application Services ==="
systemctl status hotpath-arb-engine executor-service normalizer-service discovery-service wallet-metrics-exporter --no-pager | grep -E '(Active:|●)'

echo -e "\n=== Port Usage ==="
lsof -i :29092 -i :5433 -i :6380 -i :9091 -i :3001 -i :8088 -i :9904 -i :9905 2>/dev/null | grep LISTEN
```

## Deployment Checklist

Before deploying:
- [ ] Stop all old containers (`docker stop` + `docker rm`)
- [ ] Verify no orphaned containers (`docker ps -a`)
- [ ] Check port availability (`lsof -i :PORT`)
- [ ] Backup critical data if needed

During deployment:
- [ ] Use correct docker-compose file (`docker-compose.core.yml`)
- [ ] Verify services start in correct order (dependencies)
- [ ] Check health status of each service

After deployment:
- [ ] Verify all containers running with `infrastructure-*` prefix
- [ ] Check application logs (`journalctl -u SERVICE -n 50`)
- [ ] Test service endpoints (curl health checks)
- [ ] Verify no port conflicts

## Troubleshooting

### "Port already in use" Error
```bash
# Find what's using the port
lsof -i :PORT

# If it's an old container, stop it
docker stop CONTAINER_NAME
docker rm CONTAINER_NAME
```

### "Container name already exists" Error
```bash
# Remove the old container
docker rm -f CONTAINER_NAME
```

### Kafka Cluster ID Mismatch
```bash
# This happens when Kafka data persists but Zookeeper was recreated
# Solution: Stop both, remove Kafka volume, restart

docker compose -f docker-compose.core.yml stop kafka zookeeper
docker volume rm infrastructure_kafka_data
docker compose -f docker-compose.core.yml up -d zookeeper kafka
```

### Database Connection Refused
```bash
# PostgreSQL may take 10-15 seconds to fully start
# Wait for health check:
docker ps --filter name=postgres --format '{{.Status}}'

# Should show: "Up X seconds (healthy)"
```

## Maintenance Tasks

### Weekly
- [ ] Check disk usage: `docker system df`
- [ ] Review container logs for errors
- [ ] Verify all services healthy

### Monthly
- [ ] Cleanup unused images: `docker image prune -a`
- [ ] Review and archive old logs
- [ ] Update service configurations if needed

### Quarterly
- [ ] Full infrastructure restart
- [ ] Review and optimize resource limits
- [ ] Update Docker images to latest stable versions

## Emergency Procedures

### Complete System Restart
```bash
# 1. Stop all services
sudo systemctl stop hotpath-arb-engine executor-service normalizer-service discovery-service wallet-metrics-exporter

# 2. Stop all containers
cd /opt/MEVana/infrastructure
docker compose -f docker-compose.core.yml down

# 3. Clean up (if needed)
docker ps -a --filter 'name=mevana-' --format '{{.Names}}' | xargs -r docker rm -f

# 4. Restart infrastructure
docker compose -f docker-compose.core.yml up -d

# 5. Wait for Kafka to be healthy (30 seconds)
sleep 30

# 6. Restart application services
sudo systemctl start hotpath-arb-engine executor-service normalizer-service discovery-service wallet-metrics-exporter
```

### Data Recovery
If data is lost or corrupted:
1. Stop all services
2. Restore from backup (if available)
3. If no backup, remove corrupted volumes and start fresh
4. Re-initialize database with migrations

## Best Practices

1. **Always use docker-compose.core.yml** for production deployments
2. **Never mix deployment methods** (don't run both docker-compose.yml and docker-compose.core.yml)
3. **Clean up before deploying** (stop and remove old containers)
4. **Verify naming consistency** (all infrastructure-* containers)
5. **Check port conflicts** before starting services
6. **Monitor resource usage** (CPU, memory, disk)
7. **Keep logs** for troubleshooting
8. **Document changes** when modifying configurations

## References

- Docker Compose Reference: `/opt/MEVana/infrastructure/docker-compose.core.yml`
- Systemd Services: `/etc/systemd/system/*.service`
- Service Configurations: `/etc/mevana/*.env`
- Health Check Script: `/opt/MEVana/scripts/health_check.sh`
