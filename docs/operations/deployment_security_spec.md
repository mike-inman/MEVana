# MEVana Deployment & Security Specification (Draft)
Last updated: 2025-09-15

## 1. Objectives
Provide a hardened, low‑latency, auditable production environment for the MEVana executor and supporting services (streamers, normalizers, provenance DB) in a NY colocation. Primary goals: integrity, availability, confidentiality of keys & trade logic; minimize latency to Solana RPC + Jito relays; enable fast rollback.

## 2. Threat Model (High-Level)
Actors:
- External attacker (network) seeking RCE, data exfiltration, key theft.
- Malicious insider / supply chain (poisoned dependency, CI artifact tampering).
- Opportunistic co‑tenants (shared colo facility risks, side channels).
- Rogue RPC provider (data manipulation, front‑running / censorship).
- Compromised build agent (inject backdoor binary).

Assets:
- Wallet signing keys (hot) + backup keys.
- Execution engine binary & config (deterministic behavior).
- Trade opportunity & execution history (alpha leakage if exposed early).
- API credentials (Shyft, Kafka, Postgres, admin endpoints).

Key Risks:
- Key exfiltration via memory scrape / disk theft.
- Config tampering (disable risk limits).
- Malicious bundle submission (arbitrary transactions).
- Output manipulation (false PnL / risk metrics).
- Registry poisoning (decimals or oracle spoof leading to incorrect sizing).

## 3. Security Controls Overview
1. Host Hardening
   - Minimal OS: Ubuntu 24.04 LTS minimal (no desktop, remove compilers on prod).
   - CIS Benchmark baseline (automated via Ansible hardening role).
   - Kernel: enable lockdown (integrity mode), disable unneeded modules (USB storage if not required), enable BPF LSM restrictions.
   - Mandatory patches: unattended security updates + weekly manual kernel review.
2. Networking
   - Dedicated VLAN / private subnet for internal services (Postgres, Kafka) – no public exposure.
   - Ingress only via WireGuard bastion or mTLS gateway for admin APIs.
   - eBPF firewall (Cilium or nftables) restrict outbound to approved RPC endpoints + Jito relays.
3. Secrets & Keys
   - Hot wallet key in tmpfs + fs-verity verified binary region separate.
   - Use age + hardware-backed sealed secret (TPM 2.0) for at-rest key encryption on boot – operator unlock workflow.
   - Environment secrets loaded from encrypted file (sops + age) -> decrypted in memory, never written plain to disk.
4. Binary & Supply Chain Integrity
   - Reproducible builds (cargo build --locked) inside ephemeral container.
   - Attestations (in-toto / cosign) stored alongside artifact; on startup verify signature + attestation hash vs provenance table.
   - Deny unknown outbound to crates.io at runtime (pre-fetch vendored crates for offline build cache).
5. Observability / Audit
   - Append-only structured logs shipped via TLS to log aggregator (Loki / Elastic) with hash-chain (daily root anchor in provenance DB).
   - Metrics served internally; Prometheus scrapes over mTLS.
   - Execution & opportunity rows contain config_hash + token_registry_hash to allow forensic reconstruction.
6. Access Control
   - Admin HTTP endpoints require x-admin-token or (phase 2) mTLS mutual auth client cert.
   - Principle of least privilege DB roles: executor_write (limited INSERT/SELECT on specific tables), ddl_admin (migrations only).
   - Kafka: SASL/SCRAM with per-service credentials.
7. Runtime Protections
   - Seccomp profile (allow basic syscalls; block ptrace, perf_event_open, bpf unless required).
   - Memory protections: disable core dumps, set /proc restrictions (hidepid=2), ASLR enabled.
   - Resource quotas (systemd slice): CPU set pinning, memory limit with soft+hard, IO priority.
8. Data Protection
   - At-rest encryption: LUKS for block device, Postgres cluster inside encrypted volume.
   - Optional column encryption (pgcrypto) for sensitive API tokens / future user data.
   - Regular encrypted offsite snapshot (daily) – verify restore quarterly.
9. Incident Response
   - Defined runbooks: key compromise, suspicious outbound traffic, latency anomaly, PnL integrity discrepancy.
   - Automated containment script: revoke API keys, disable bundle submission flag, snapshot memory for forensics.

## 4. Hardware Recommendations
- CPU: AMD EPYC (Milan / Genoa) 16–24 cores; prioritize higher single-thread turbo for path-critical tasks.
- NUMA: Single-socket preferred to avoid cross-NUMA latency; pin critical executor threads to isolated core set (taskset / systemd CPUAffinity).
- RAM: 64GB ECC (executor footprint modest, headroom for Kafka/Postgres & future analytics in-memory caches).
- Storage: NVMe Gen4 SSD (enterprise) mirrored (RAID1) for OS + data; separate device or partition for Postgres WAL with high endurance.
- NIC: Dual 10/25GbE low-latency NIC (Mellanox ConnectX) with firmware updated; enable RSS & tune IRQ affinity.
- Time: PTP (Precision Time Protocol) client sync; fallback NTP.

## 5. Network Topology
- Segments:
  - dmz_admin (bastion only)
  - app_exec (executor, normalizers, streamers)
  - data_core (Postgres, Kafka, Redis/future)
- Firewall policy: default deny egress from app_exec except whitelisted Solana RPC, Jito endpoints, Shyft, monitoring collector.
- WireGuard tunnel for admin operations; disable password SSH (keys only, FIDO2 optional).

## 6. Deployment Pipeline
1. Developer push -> CI (lint, tests, security scan: cargo audit, trivy).
2. Build artifact + SBOM + signature.
3. Staging deploy (ephemeral namespace) – smoke tests: migrations dry run, synthetic opportunity replay.
4. Promotion requires manual approval + config hash validation.
5. Production deploy: systemd unit update (atomic symlink swap), health check gate (warm-up), remove old binary after 24h.

## 7. Configuration & Secrets Layout
- /etc/mevana/config.d/*.toml (read-only root owned, 600)
- /run/mevana/secrets.env (tmpfs) populated by bootstrap decrypt script
- systemd unit sets EnvironmentFile pointing to secrets.env
- Secrets rotation: replace encrypted file, trigger reload hook (SIGHUP) -> re-derive in-memory tokens.

## 8. Key Management Strategy
- Hot key: single limited-balance execution wallet; enforce balance guard already implemented.
- Treasury / cold keys: kept offline; periodic top-ups via manual multi-sig transaction.
- Option: integrate HSM (YubiHSM2 or NitroKMS with remote signing adapter) – latency tradeoff evaluation.

## 9. Hardening Checklist (Operational)
- [ ] OS baseline applied & audited weekly
- [ ] All binaries signed & verified on startup
- [ ] Admin token rotated quarterly
- [ ] Postgres role review monthly
- [ ] Restore test last 90 days pass
- [ ] Log integrity anchor (daily) stored externally
- [ ] Vulnerability scan (cargo audit + trivy) clean
- [ ] PTP sync within <50µs of source

## 10. Future Enhancements
- Full mTLS mutual auth for all internal HTTP (replace token header)
- Hardware-backed remote attestation (TPM quotes stored in provenance)
- Enforced read-only root FS (overlayfs for ephemeral writes)
- Side-channel resistant signing enclave (SGX / SEV-SNP) feasibility study
- Live anomaly detection on execution pattern (sudden variance)

## 11. Runbook Stubs (Outline)
- Key Compromise: disable EXECUTION_ALLOW_LIVE, revoke API keys, rotate admin token, redeploy with new signing key.
- Latency Spike: check NIC stats, CPU steal, upstream RPC health, revert recent config hash.
- Data Drift (registry/oracle mismatch): trigger /admin/registry/refresh, diff new hash vs last good, rollback if decimals changed unexpectedly.

---
Draft version 0.1 (2025-09-01). Update as features mature.
