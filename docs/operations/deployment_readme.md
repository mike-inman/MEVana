## Deployment Quick Guide (Executor Service)

### 1. Build & Install
```
cargo build --release -p executor-service
sudo mkdir -p /opt/mevana/bin
sudo cp target/release/executor-service /opt/mevana/bin/
```

### 2. Create User & Directories
```
sudo useradd --system --home /opt/mevana --shell /usr/sbin/nologin mevana
sudo chown -R mevana:mevana /opt/mevana
sudo mkdir -p /etc/mevana
sudo chown root:mevana /etc/mevana
sudo chmod 750 /etc/mevana
```

### 3. Secrets
Place decrypted runtime secrets into `/etc/mevana/secrets.env` (mode 640). Example:
```
SHYFT_API_KEY=...
ADMIN_API_TOKEN=...
DATABASE_URL=postgres://mev:...@db/mevdb
KAFKA_BROKER=localhost:9092
```

### 4. systemd Unit
Copy `infrastructure/systemd/executor-service.service` to `/etc/systemd/system/` then:
```
sudo systemctl daemon-reload
sudo systemctl enable --now executor-service
```

### 5. mTLS Admin API (Optional)
Set environment variables in secrets.env:
```
ADMIN_MTLS_ENABLE=1
ADMIN_TLS_CERT=/etc/mevana/tls/admin.crt
ADMIN_TLS_KEY=/etc/mevana/tls/admin.key
ADMIN_TLS_CA=/etc/mevana/tls/ca.crt
```

### 6. Hash Chain Anchoring
Daily anchor rows stored in table `run_provenance_anchor`. Intermediate checkpoints logged every 60s.

### 7. Secrets Reload
Rotate sensitive values then: `sudo systemctl kill -s HUP executor-service`.

### 8. Hardening Additions
- Consider applying seccomp profile via systemd `SystemCallFilter` (provided JSON is reference for container usage).
- Add AppArmor/SELinux profile for stricter confinement.

### 9. Observability
Prometheus metrics: `http://host:METRICS_PORT/metrics`.
Admin endpoints (token or mTLS):
- GET `/token/{mint}`
- POST `/admin/registry/refresh`

### 10. Backup & Restore
- Snapshot Postgres daily.
- Copy token registry snapshot file if using snapshots (`TOKEN_REGISTRY_SNAPSHOT_PATH`).

---
Generated: 2025-09-01
