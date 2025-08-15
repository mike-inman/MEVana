# MEVana

One-liners you might want handy
View topics:
kafka-topics --bootstrap-server $KAFKA_BROKER --list

Peek at latest messages:
kafka-console-consumer --bootstrap-server $KAFKA_BROKER --topic prices.latest --from-beginning --max-messages 5

Stop everything (if you backgrounded it):
pkill -f "cargo run -p" || true



# one-time bootstrap
docker compose -f infra/docker/docker-compose.yml --env-file infra/docker/.env.docker up -d
docker compose -f infra/docker/docker-compose.yml --env-file infra/docker/.env.docker up kafka-init


normalizer-orca
Reads raw geyser txs (orca-swaps) → writes normalized swaps (orca-swaps.parsed) and DLQ.

bash
Copy
Edit
# terminal A
export KAFKA_BROKER=localhost:29092
export IN_TOPIC=orca-swaps
export OUT_TOPIC=orca-swaps.parsed
export DLQ_TOPIC=dlq
export GROUP_ID=normalizer-orca
RUST_LOG=info cargo run -p normalizer-orca
Expected logs:

csharp
Copy
Edit
normalizer-orca started: in='orca-swaps' out='orca-swaps.parsed'
... normalized swap -> orca-swaps.parsed key=<txsig>
price-aggregator
Reads normalized swaps (orca-swaps.parsed) → emits rolling prices (prices.latest).

bash
Copy
Edit
# terminal B
export KAFKA_BROKER=localhost:29092
export IN_TOPIC=orca-swaps.parsed
export OUT_TOPIC=prices.latest
export GROUP_ID=price-aggregator
export EMIT_SECS=5
RUST_LOG=info cargo run -p price-aggregator
Expected logs:

csharp
Copy
Edit
price-aggregator started: in='orca-swaps.parsed' out='prices.latest'
... emitted N price snapshots


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
