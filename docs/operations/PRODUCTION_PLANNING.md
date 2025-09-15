Last updated: 2025-09-15

## Production Planning (AI‑friendly)

Date: 2025-08-19

### Objective
Define a pragmatic plan to harden and operate MEVana in production with real data, high availability, cost control, and observability. This is a plan, not an immediate implementation.

### Scope
- Kafka + ZooKeeper operations (retention, capacity, snapshot cleanup)
- Network and configuration hygiene (no hardcoded addresses)
- Security hardening (secrets, images, RBAC, TLS roadmap)
- Observability (metrics, logs, dashboards)
- Data retention policies (per-topic classes)
- Backup & restore (Kafka metadata, dashboards)

### Broker Stack
- Kafka
  - Per-topic defaults by class:
    - debug/dev topics: retention.ms=1h, retention.bytes=512MB
    - raw high-volume topics: retention.ms=24h, retention.bytes=50GB (cluster-level decision)
    - derived/curated topics: retention.ms=168h (7d), retention.bytes as needed
  - Monitoring: disk usage per topic dir, consumer lag, partition skew
  - Alarms: volume 70/85% thresholds, topic growth anomalies
- ZooKeeper
  - Enable autopurge (compose env):
    - ZOOKEEPER_AUTOPURGE_SNAP_RETAIN_COUNT=3
    - ZOOKEEPER_AUTOPURGE_PURGE_INTERVAL=24

### Security Hardening (Roadmap)
- Secrets management
  - Move SHYFT_API_KEY and DB creds to Docker/Compose secrets or Vault-like provider
  - Avoid env exposure in logs/UI
- Image provenance
  - Pin digests for base images
  - Weekly image rebuilds and security scan (Grype/Trivy)
- Network policy
  - Restrict inter-service traffic to `mevana_net`
  - Separate internal/external listener profiles for Kafka
- Kafka auth (phase 2)
  - SASL/PLAIN for internal dev; mTLS in prod clusters
  - ACLs for producers/consumers by topic namespace
- HTTP services
  - Add minimal auth/allowlists to UIs (Grafana, Kafka-UI, Adminer)
  - TLS termination via reverse proxy (future)

### Observability
- Prometheus & Grafana (already integrated)
  - Dashboards for: Kafka broker, topic byte/s, disk, consumer lag; service health; latency histograms
  - Persistent volumes for metrics and dashboards
- Logging
  - Standard JSON logs with rate limits; rotate via Docker logging options

### Capacity & Cost Controls
- Disk management
  - Rolling shrink on large topics; per-topic retention tightened for debug feeds
  - ZK autopurge (enabled)
  - Alerts on volume usage
- CPU/mem
  - Compose limits already specified; tune per service from Prom metrics

### Backups & Restore
- Kafka metadata: export topic configs, ACLs (scripted)
- Grafana dashboards: keep JSON in repo (already provisioned); restore from files
- Postgres: daily dump, store off-host

### Runbook (Ops)
- Check heavy topics:
  - `docker exec kafka bash -lc 'du -sh /var/lib/kafka/data/* | sort -h | tail -n 30'`
- Adjust retention (example):
  - `kafka-configs --bootstrap-server localhost:9092 --alter --entity-type topics --entity-name <topic> --add-config retention.ms=3600000,retention.bytes=536870912`
- Verify configs:
  - `kafka-topics --bootstrap-server localhost:9092 --describe --topic <topic>`
- ZooKeeper health:
  - `docker logs zookeeper | tail -n 100`
  - Autopurge is configured via compose envs (see docker-compose.yml)

### Next Steps
- Decide per-topic policies and commit a `topics-policy.yml` for reproducible config
- Add alert rules and a lightweight on-call doc
- Plan Kafka auth & ACLs (phase 2)


