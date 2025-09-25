# MEVana Port Mapping - Production Ready Configuration
Last updated: 2025-09-15

**Last Updated:** 2025-09-09  
**Status:** 🚀 PRODUCTION READY - Organized & Documented

## Infrastructure Services (Docker)
| Service | Internal Port | External Port | Purpose | Status |
|---------|--------------|---------------|---------|---------|
| Zookeeper | 2181 | 2181 | Kafka coordination | ✅ Running |
| Kafka | 9092/29092 | 9092/29092 | Message broker | ✅ Running |
| **Kafka-UI** | 8080 | **8080** | Kafka management | ✅ Running |
| Redis | 6379 | 6379 | Caching | ✅ Running |
| Redis-UI | 8081 | 8083 | Redis management | ✅ Running |
| PostgreSQL | 5432 | 5433 | Database | ✅ Running |
| Adminer | 8080 | 8082 | Database management | ✅ Running |
| Prometheus | 9090 | 9090 | Metrics collection | ✅ Running |
| Grafana | 3000 | 3000 | Metrics visualization | ✅ Running |

## Core Trading Services (Native)
| Service | Port | Purpose | Status | Health Check |
|---------|------|---------|---------|--------------|
| **arb-engine** | **8085** | Main arbitrage engine (WebSocket + API) | ✅ Running | `curl -s localhost:8085/metrics` |
| **executor-service** | **9904** | Trade execution orchestration | ✅ Running | `curl -s localhost:9904/health` |
| **normalizer-service** | **9901** | Transaction parsing (Kafka) | ✅ Running | Process check only |
| **edge-assembler** | **9902** | Real-time edge processing | ✅ Running | Process check only |
| **wallet-vault** | **8090** | Wallet encryption/management | ✅ Running | `curl -s localhost:8090/health` |

## HotPath & Supporting Services  
| Service | Port | Purpose | Status | Health Check |
|---------|------|---------|---------|--------------|
| **hotpath_arb_engine** | **8088** | Zero-latency MEV detection | ✅ Running | `curl -s localhost:8088/performance` |
| **monitoring-service** | **8091** | Performance monitoring APIs | ✅ Running | `curl -s localhost:8091/health` |
| **token-registry** | **9898** | Token metadata (via executor-service) | ✅ Running | `curl -s localhost:9898/token/{mint}` |

## API Services (Running)
| Service | Port | Purpose | Status | Health Check |
|---------|------|---------|---------|--------------|
| **api-service** | **8092** | Public REST API | ✅ Running | `curl -s localhost:8092/health` |
| **gateway (api-service)** | **8095** | Consolidated UI gateway (WebSocket + REST) | Optional | `curl -s localhost:8095/gateway/health` |

## Future Services (Planned)
| Service | Proposed Port | Purpose | Notes |
|---------|--------------|---------|--------|
| **auth-gateway** | 3001 | Clerk/JWT authentication | Node.js auth boundary |
| **Web UI (Next.js)** | 3200 | Trading dashboard | React frontend |
| **system-orchestrator** | 9999 | Service orchestration | Reserved for startup/health |

## Reserved / Port Allocation Strategy
| Range | Allocation | Notes |
|-------|-----------|-------|
| 2181 | Zookeeper | Infrastructure |
| 29092 | Kafka external | Client access (internal 9092) |
| 3000 | Grafana | Metrics dashboard |
| 3001 | Auth Gateway | JWT / session boundary |
| 3100-3199 | Future dashboards | Reserved (avoid clashes) |
| 3200 | Primary Web UI | Proposed selection (changeable) |
| 4200 | (Optional alt) Frontend dev | If Angular/Vite alt needed |
| 5173 | (Optional alt) Vite dev | Use if switching toolchain |
| 5433 | Postgres | Host-mapped (container 5432) |
| 6379 | Redis | Cache / session |
| 8080 | Kafka-UI | Existing |
| 8082 | Adminer | DB management |
| 8083 | Redis-UI | Existing |
| 8084 | dex-stream-universal | Streaming health / metrics |
| 8085 | Arbitrage Engine | API/metrics (doc) |
| 8088 | Hotpath Arb Engine | Performance metrics |
| 8090 | wallet-vault | Secure wallet API boundary |
| 8091 | api-service | Public API aggregation |
| 8095 | Gateway listener | UI gateway (enable with `MEVANA_GATEWAY_ENABLE=true`) |
| 8092-8099 | Future API services | Keep contiguous block |
| 9000-9099 | (Available) | Leave gap for future high-load services |
| 9090 | Prometheus | Metrics collector |
| 9500-9599 | (Future) Internal tools | Reserve for ops/debug |
| 9800-9899 | (Future) Experimental | Ephemeral services |
| 9900-9999 | Internal Rust metrics | Avoid external exposure |

## 🚀 PRODUCTION DEPLOYMENT STRATEGIES

### Immediate Production Readiness Actions

#### 1. Port Consolidation & Security
```bash
# Production Port Ranges (Firewall Rules)
# PUBLIC (External Access): 3000-3200, 8080-8099
# INTERNAL (Service-to-Service): 9900-9999
# INFRASTRUCTURE (Docker): 2181, 5433, 6379, 9090-9092, 29092

# Firewall Configuration (Production)
ufw allow 3000:3200/tcp   # Web interfaces & API
ufw allow 8080:8099/tcp   # Trading engine APIs
ufw deny 9900:9999/tcp    # Block internal metrics
```

#### 2. Environment-Specific Configuration
```bash
# environments/production.env
HTTP_PORT=8080
METRICS_PORT=9001  # Internal only
DATABASE_URL=postgresql://mev:${DB_PASS}@db:5432/mevdb
KAFKA_BROKER=kafka:9092
REDIS_URL=redis://redis:6379

# environments/staging.env  
HTTP_PORT=8081
METRICS_PORT=9002
# ... offset all ports by 1

# environments/development.env
# Current port assignments (as documented)
```

#### 3. Docker Compose Production Setup
```yaml
version: '3.8'
services:
  arb-engine:
    build: .
    command: cargo run -p arb-engine
    environment:
      - WS_BIND=0.0.0.0:8080
      - METRICS_PORT=9001
    ports:
      - "8085:8080"    # External:Internal
    networks:
      - mevana_internal
    deploy:
      replicas: 2
      restart_policy:
        condition: on-failure
        max_attempts: 3
    
  nginx-lb:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443" 
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/ssl/certs
    depends_on:
      - arb-engine
      - executor-service
```

#### 4. Service Discovery Integration
```rust
// crates/shared-core/src/service_discovery.rs
use consul::{Client as ConsulClient, Config as ConsulConfig};

#[derive(Clone)]
pub struct ServiceRegistry {
    consul: ConsulClient,
    service_name: String,
    health_check_url: String,
}

impl ServiceRegistry {
    pub async fn register(&self) -> Result<()> {
        let registration = ServiceRegistration {
            name: self.service_name.clone(),
            id: format!("{}-{}", self.service_name, uuid::Uuid::new_v4()),
            port: self.port,
            check: HealthCheck {
                http: self.health_check_url.clone(),
                interval: "10s".to_string(),
            },
        };
        self.consul.agent().service().register(&registration).await
    }
}
```

#### 5. Health Check Standardization
```rust
// Standard health endpoint for ALL services
#[get("/health")]
pub async fn health_check(
    app_state: web::Data<AppState>,
) -> impl Responder {
    let health = HealthStatus {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        uptime: app_state.start_time.elapsed(),
        dependencies: check_dependencies(&app_state).await,
        metrics: BasicMetrics {
            memory_usage: get_memory_usage(),
            cpu_usage: get_cpu_usage(),
            active_connections: app_state.connection_count.load(Ordering::Relaxed),
        }
    };
    HttpResponse::Ok().json(health)
}
```

#### 6. Configuration Management
```rust
// Centralized config with env overrides
#[derive(Deserialize, Clone)]
pub struct ServiceConfig {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    
    pub database_url: String,
    pub kafka_broker: String,
    pub redis_url: String,
    
    #[serde(default)]
    pub environment: Environment,
}

impl ServiceConfig {
    pub fn load() -> Result<Self> {
        let mut config = config::Config::new();
        
        // Start with defaults
        config.merge(config::File::with_name("config/default"))?;
        
        // Environment-specific overrides
        if let Ok(env) = std::env::var("ENV") {
            config.merge(config::File::with_name(&format!("config/{}", env)).required(false))?;
        }
        
        // Environment variable overrides
        config.merge(config::Environment::with_prefix("MEVANA").separator("__"))?;
        
        config.try_into().map_err(Into::into)
    }
}
```

#### 7. Monitoring & Observability
```yaml
# docker-compose.monitoring.yml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
  
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      
  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml
```

### Production Deployment Checklist

#### Pre-Deployment
- [ ] All services have standardized health endpoints
- [ ] Environment-specific configuration files created
- [ ] SSL certificates generated and configured
- [ ] Database migrations tested
- [ ] Monitoring dashboards configured
- [ ] Alerting rules defined
- [ ] Load testing completed
- [ ] Security audit performed

#### Infrastructure
- [ ] Kubernetes cluster provisioned (or Docker Swarm)
- [ ] Service mesh configured (Istio/Linkerd)
- [ ] Ingress controller deployed (NGINX/Traefik)
- [ ] External DNS configured
- [ ] Backup strategy implemented
- [ ] Log aggregation setup (ELK/Fluentd)

#### Deployment Process
- [ ] Blue-green deployment pipeline
- [ ] Automated rollback capability
- [ ] Database migration automation
- [ ] Configuration secret management (Vault/K8s Secrets)
- [ ] Zero-downtime deployment testing

## Action Items

## Verification Checklist (Quick)
- [ ] No overlapping host ports in use
- [ ] Metrics ports not colliding with UI/API ports
- [ ] Postgres mapped port matches application configuration
- [ ] Web UI chosen port (3200) not already bound (validate before launch)

## Operational Notes
- Postgres logical backup script: `scripts/backup_postgres.sh` (creates `backups/pg-backup-<db>-<timestamp>.sql.gz`). Run before destructive operations.
- Web UI reserved host port: 3200 (update here first if changed).
- For new services allocate from contiguous blocks (see Reserved / Port Allocation Strategy) and update this file in the same commit.

## Port Conflict Resolution
- **Kafka-UI uses port 8080** (Docker internal)
- **Arbitrage Engine moved to port 8085** (native service)
- This prevents conflicts between Docker and native services

## Access URLs & API Endpoints
- Kafka UI: http://localhost:8080
- **Arbitrage Engine**: http://localhost:8085/metrics, http://localhost:8085/ai/config
- Grafana: http://localhost:3000 (admin/admin123)
- Redis UI: http://localhost:8083 (admin/admin123)
- Prometheus: http://localhost:9090
- Adminer: http://localhost:8082
- HotPath Engine: http://localhost:8088/performance, http://localhost:8088/metrics

### Key Documents & Links
- **This Document**: Master port allocation registry
- **Startup Script**: `scripts/start_system.sh` - References these port assignments
- **Database Schema**: `database/complete_schema.sql` - Master schema with user associations
- **CLAUDE.md**: Development guidance with architecture overview
- **STARTUP_TROUBLESHOOTING.md**: Comprehensive troubleshooting guide with binary specs and port fixes
- **STARTUP_PROGRESS.md**: Detailed startup troubleshooting and discoveries

### Environment Variables by Service
- **arb-engine**: Uses `WS_BIND=0.0.0.0:8085` (NOT ARB_HTTP_PORT)
- **hotpath_arb_engine**: Uses `METRICS_PORT=8088`
- **normalizer-service**: Uses `METRICS_PORT=9901`
- **edge-assembler**: Uses `METRICS_PORT=9902`
- **executor-service**: Uses `METRICS_PORT=9904`
- **wallet-vault**: Uses built-in port 8090

## Network Configuration
- Docker services use `mevana_net` network
- Native services connect to Docker services via localhost ports
- No port conflicts between infrastructure and trading services
