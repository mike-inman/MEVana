# Solana Arbitrage Platform - Infrastructure

## Overview

This directory contains the Docker infrastructure for the Solana Arbitrage Platform, providing a streamlined development environment with optimized builds and consistent naming conventions.

## Quick Start

### 1. Environment Setup

```bash
# Copy environment template
cp env.example .env

# Edit environment variables
nano .env
```

### 2. Build and Start

```bash
# Full build and start (recommended for first time)
./build.sh full

# Or step by step:
./build.sh cache      # Build cache layers
./build.sh build      # Build all services
./build.sh start      # Start services
```

### 3. Monitor Services

```bash
# Check service health
./build.sh health

# View logs
./build.sh logs stream-orca
./build.sh logs swap-ingest
./build.sh logs pathfinder

# View all logs
./build.sh logs

# Test heartbeats
./build.sh heartbeats

# Monitor heartbeats in real-time
./build.sh monitor
```

## Architecture

### Services

| Service | Port | Description | Health Check | Heartbeat |
|---------|------|-------------|--------------|-----------|
| `redis` | 6379 | Message broker | Redis ping | - |
| `stream-orca` | 9092 | Orca transaction streaming | Process check | Every 30s |
| `swap-ingest` | 9094 | Swap data processing | Process check | Every 30s |
| `pathfinder` | 9095 | Arbitrage detection | Process check | Every 30s |
| `redis-viewer` | 5001 | Redis data inspection | - | - |
| `kafka-ui` | 8080 | Kafka management UI | - | - |
| `redis-ui` | 8083 | Redis management UI | - | - |
| `adminer` | 8081 | Database management | - | - |

### Network

All services run on the `arbitrage-network` bridge network for consistent communication.

## Development Workflow

### Fast Development Cycle

```bash
# 1. Make code changes

# 2. Rebuild specific service (fast)
./build.sh build stream-orca

# 3. Restart service
./build.sh restart stream-orca

# 4. Check logs
./build.sh logs stream-orca
```

### Environment Variables

All services use environment variables for configuration. Key variables:

- `REDIS_URL`: Redis connection string
- `SHYFT_GRPC_ENDPOINT`: Shyft gRPC endpoint
- `SHYFT_GRPC_TOKEN`: Shyft API token
- `LOG_LEVEL`: Logging level (trace, debug, info, warn, error)
- `MIN_PROFIT_THRESHOLD`: Minimum profit for arbitrage detection

### Build Optimization

The infrastructure uses several optimizations for faster builds:

1. **Multi-stage Dockerfiles**: Separate builder and runtime stages
2. **Dependency caching**: Build dependencies separately from source code
3. **Targeted COPY commands**: Only copy necessary files
4. **Build cache**: Reuse cached layers between builds
5. **Slim base images**: Use minimal runtime images

## Monitoring & Debugging

### Health Checks

All services include health checks that monitor:
- Process availability
- Service responsiveness
- Resource usage

### Logging

Structured logging with configurable levels:
- `trace`: Detailed execution flow
- `debug`: Debug information
- `info`: General information
- `warn`: Warnings
- `error`: Errors

### Heartbeat Monitoring

All core services (stream-orca, swap-ingest, pathfinder) include heartbeat functionality:

- **Heartbeat Interval**: Every 30 seconds
- **Heartbeat Message**: `💓 [service-name] heartbeat - service is alive and [description]`
- **Monitoring**: Use `./build.sh heartbeats` to test or `./build.sh monitor` for real-time monitoring

Example heartbeat messages:
```
💓 stream-orca heartbeat - service is alive and processing transactions
💓 swap-ingest heartbeat - service is alive and listening for swaps
💓 pathfinder heartbeat - service is alive and scanning for arbitrage
```

### Metrics

All services expose Prometheus metrics on port 9090:
- Request counts
- Response times
- Error rates
- Custom business metrics

## Troubleshooting

### Common Issues

1. **Build Context Too Large**
   ```bash
   # Clean up Docker resources
   ./build.sh clean
   
   # Rebuild with cache
   ./build.sh cache
   ./build.sh build
   ```

2. **Service Not Starting**
   ```bash
   # Check logs
   ./build.sh logs <service-name>
   
   # Check health
   ./build.sh health
   ```

3. **Network Issues**
   ```bash
   # Restart network
   docker network prune
   ./build.sh restart
   ```

### Performance Tuning

1. **Increase Docker Resources**
   - Memory: 8GB+ recommended
   - CPU: 4+ cores recommended
   - Disk: SSD recommended

2. **Optimize Build Cache**
   ```bash
   # Build cache layers first
   ./build.sh cache
   
   # Use cache for subsequent builds
   ./build.sh build
   ```

## File Structure

```
infrastructure/
├── docker-compose.yml      # Service orchestration
├── build.sh               # Build and management script
├── env.example            # Environment variables template
├── README.md              # This file
└── .dockerignore          # Docker build exclusions
```

## Commands Reference

### Build Script Commands

| Command | Description |
|---------|-------------|
| `./build.sh build [SERVICE]` | Build specific service or all services |
| `./build.sh cache` | Build cache layers for faster rebuilds |
| `./build.sh start [SERVICES]` | Start services |
| `./build.sh stop` | Stop all services |
| `./build.sh restart [SERVICES]` | Restart services |
| `./build.sh logs [SERVICE]` | Show logs |
| `./build.sh health` | Check service health |
| `./build.sh clean` | Clean up Docker resources |
| `./build.sh full` | Full build and start |

### Docker Compose Commands

```bash
# View service status
docker compose ps

# View service logs
docker compose logs -f <service>

# Scale services
docker compose up -d --scale stream-orca=2

# Execute commands in containers
docker compose exec stream-orca /bin/bash
```

## Contributing

When adding new services:

1. **Follow naming conventions**:
   - Service names: `kebab-case`
   - Container names: `kebab-case`
   - Image names: `arbitrage-<service>:latest`

2. **Include health checks** for all services

3. **Use environment variables** for configuration

4. **Add to build script** for easy management

5. **Update documentation** with service details

## Support

For issues and questions:
1. Check the troubleshooting section
2. Review service logs
3. Check health status
4. Verify environment configuration 