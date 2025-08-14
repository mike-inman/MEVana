# MEVana



# one-time bootstrap
docker compose -f infra/docker/docker-compose.yml --env-file infra/docker/.env.docker up -d
docker compose -f infra/docker/docker-compose.yml --env-file infra/docker/.env.docker up kafka-init



crates/
  normalizer-runtime/      # traits, helpers, serializers, redis writer
  normalizer-codegen/      # parses IDL+profile, emits Rust
  normalizer-generated/    # (git-ignored) emitted sources
  normalizer-service/      # Kafka/Geyser consumer; uses registry from generated
idls/
  orca_whirlpool_idl.json
  raydium_idl.json
  meteora_idl.json
profiles/
  orca_whirlpool.yaml
  raydium.yaml
  meteora.yaml


Geyser client (Rust) ──► Kafka: geyser.raw
                           │
                           ▼
                 Normalizer (IDL+profile, amounts-from-meta)
                           │
                           ├──► Kafka: dex.swaps.normalized.v1  (system of record)
                           │
                           └──► Redis (TOB + short windows)     (hot path / queries)

Common commands
Build everything: cargo build --workspace

Run a specific crate: cargo run -p arb-feed

Test all: cargo test --workspace



.
├─ Cargo.toml            # the [workspace] file above
├─ rust-toolchain.toml   # pin toolchain (recommended)
├─ idls/
├─ crates/
│  ├─ stream_and_parse_whirlpool_account_updates/
│  ├─ dex-data-parser/
│  ├─ arb-feed/
│  └─ arb-executor/
└─ .vscode/
   ├─ settings.json
   ├─ launch.json
   ├─ tasks.json
   └─ extensions.json
