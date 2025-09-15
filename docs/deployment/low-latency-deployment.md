# Low-Latency Deployment Plan (Prod Readiness)
Last updated: 2025-09-15

## Targets
- Decision latency: <= 10 ms p50 (Geyser RX → bundle submitted), p99 <= 25 ms
- Availability: >= 99.9% (active + warm standby)
- Observability: latency histograms, counters, kill-switch

## Environment Strategy
- Colocation (prod): NYC facility near performant validators/Jito relays; bare metal preferred
- Cloud (staging): same build with relaxed tuning; measure and iterate

## Hardware Sizing (Prod)
- CPU: 16–32 cores (modern Intel/AMD)
- RAM: 64–128 GB
- Storage: 1–2 TB NVMe
- NIC: 10G/25G low-latency NIC
- Redundancy: warm standby with identical specs

## OS & Kernel Tuning (Ubuntu 22.04+)
- CPU governor: performance; pin hot services to dedicated cores
- Sysctl examples:
  - net.core.netdev_max_backlog=100000
  - net.core.rmem_max/wmem_max=268435456
  - net.core.somaxconn=65535
- NIC/IRQ:
  - IRQ affinity to isolate NIC queues
  - Enable RPS/XPS; tune queues
- NTP/PTP strict time; drift alarms

## Software Tuning (Hot Path)
- In-proc Normalizer → Arb Engine; binary (prost) hot stream; JSON only at UI
- Kafka (if used hot):
  - Producer: linger.ms=0, acks=1, compression=lz4|none, idempotence=false
  - Consumer: fetch.wait.max.ms=1–2, fetch.min.bytes=1
- Local caches: mint/pool snapshots; Redis only for background refresh
- Logging: off on hot path; counters only

## Network Placement
- Same VLAN for broker/Redis; private link to Jito relay
- Ensure firewall permits Jito endpoints

## Deployment Pipeline (CI/CD)
- CI (GitHub Actions): build workspace, tests, codegen fixtures, build/push Docker images (tag: branch+sha)
- CD: Docker Compose for small footprints; Kubernetes for HA
- Release: feature branches → PR → CI → merge → tag → staging → smoke test → prod
- Secrets: vault/sealed-secrets; no secrets in repo

## Services & Env Vars
- dex-stream-universal: KAFKA_BROKER, ENDPOINT, X_TOKEN, PROGRAM_IDS, RUST_LOG
- normalizer-service (tap_orca_extract): KAFKA_BROKER, IN_TOPIC, OUT_TOPIC, REDIS_URL, RUST_LOG
- ui-gateway: KAFKA_BROKER, PRICES_TOPIC, SWAPS_TOPIC, SWAPS_V2_TOPIC
- Backfills: REDIS_URL, SHYFT_API_KEY/NETWORK

## Observability & SLOs
- Metrics: decision latency histograms (p50/p90/p99), decode hits, extraction success, arb attempts/accepts/fills, PnL
- Dashboards: latency + PnL, venue breakdown; alerts on SLO breach/negative PnL window/crash loops

## Jito Integration Plan
- Bundle builder & simulator; private relay connection
- Tip model service; canary mode to calibrate acceptance vs tips
- Simulation gate: reject non-positive expected net PnL

## Disaster Recovery
- Config backups to secure bucket; standby node images; runbooks for cutover

## Staging/Devnet
- Same topology with relaxed tuning; mock/devnet Geyser
- Load test: replay captured raw streams at 1–5× to validate p99

## Open Items & Next Steps
- feature/fast-mode-hotpath: binary hot stream + low-latency flags
- feature/extractor-local-caches: local mint/pool caches + UI amounts
- feature/jito-bundle-scaffold: bundle builder, tip model, canary
- feature/metrics-latency-slos: histograms/dashboards/kill-switch
