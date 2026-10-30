# 2026-10-29 20:32 CDT — Tatum and Sodae Geyser development setup

Created: 2026-10-29

## Purpose and current result

Receive a bounded sample of real Solana Yellowstone slot updates from each provider without starting MEVana, connecting a wallet, or submitting transactions. This is provider connectivity setup, not implementation of the separate read-only market-data capture handoff.

Verified locally on 2026-10-29:

- Tatum's TLS endpoint answered `geyser.Geyser/GetSlot` without authentication.
- Tatum's `Subscribe` stream returned `UNAUTHENTICATED` without a key. No stream updates were received. Public RPC availability does not establish anonymous streaming access.
- Sodae requires an API key and an active Yellowstone entitlement. No authenticated Sodae request has been made.
- Sodae currently advertises a team-approved 5 GB / 7-day Yellowstone trial with no card required. The earlier 10 GB / 30-day description is superseded.

## Tatum account

1. Sign in at <https://dashboard.tatum.io>.
2. Obtain an API key and confirm that Solana Yellowstone gRPC is enabled with free access or trial credits. A free API key alone does not establish that all streaming is free. Do not purchase a plan for this development check.
3. Endpoint: `https://solana-mainnet-grpc.gateway.tatum.io:443`.
4. Authentication metadata: `x-api-key`.
5. If the free account does not include streaming, report that limitation rather than upgrading automatically.

## Sodae account

1. Sign in at <https://sodae.io/auth> using email, Google or Discord. Email signup requires the emailed verification code.
2. Find the first generated key under API Keys.
3. Request the **Yellowstone gRPC** free trial during onboarding or on the product page. Wait for team approval. The RPC trial is a separate product.
4. Endpoint: `https://ams.tls.sodae.io:443`.
5. Authentication metadata: `x-token`.
6. According to the quickstart, trials accept any IP until an allowlist is added. If you add an allowlist, include the public IP of the machine running the sample.

## Get a small sample

Run in a local zsh terminal from the repository root. The hidden prompts keep keys out of shell history. These provider-specific variables do not replace the existing Shyft credentials. They last for the current terminal session; no existing secret files are changed. Keep the keys saved in your password manager for later sessions.

```zsh
cd /Users/michaelinman/Repos/MEVana
read -rs 'TATUM_API_KEY?Tatum API key: '; print
export TATUM_API_KEY
uv run scripts/development/geyser_provider_sample.py tatum

read -rs 'SODAE_TOKEN?Sodae API key: '; print
export SODAE_TOKEN
uv run scripts/development/geyser_provider_sample.py sodae
```

The helper uses pinned Python dependencies through `uv`, which is installed locally. Each invocation requests processed slot notifications, prints ten JSON lines and closes the connection, with a 20-second maximum RPC duration. Each line contains provider, local UTC receipt time, slot, parent slot and numeric Yellowstone status. Status 0 means processed. No tokens or provider error details are printed. This short diagnostic does not implement reconnect or ping handling for a sustained stream.

To save the returned public chain data without overwriting an earlier sample:

```zsh
sample_dir=$(mktemp -d /tmp/mevana-geyser-samples.XXXXXX)
uv run scripts/development/geyser_provider_sample.py tatum > "$sample_dir/tatum-slots.jsonl"
uv run scripts/development/geyser_provider_sample.py sodae > "$sample_dir/sodae-slots.jsonl"
print -r -- "$sample_dir"
```

A zero exit code and ten slot lines establish that the subscription works. An empty output file does not. `UNAUTHENTICATED` means check the key/header; `PERMISSION_DENIED` can mean missing trial approval, entitlement, quota or IP permission. Confirm the reason in the dashboard; the helper deliberately suppresses error details that could echo credentials. Do not repeatedly retry quota or permission failures.

## MEVana integration boundary

The HotPath code currently uses `.x_token(...)` in `crates/hotpath_arb_engine/src/main.rs`, which matches Sodae's header but not Tatum's `x-api-key`. The sample helper supports both independently. Do not assume setting `X_TOKEN` to a Tatum key will work. Adding provider-specific authentication and filtered account/transaction capture is a separate implementation step; these slot samples do not establish Orca decoding, account filter support, cluster identity or trading readiness. Mainnet is the advertised network, not independently confirmed by these checks.

## Local validation

Three decoder tests pass (slot fields, unknown fields, ping exclusion and default processed status). The unauthenticated Tatum streaming check failed explicitly, and Sodae's missing-key guard fails before connecting. `git diff --check` passed. Repository hygiene passed with existing warnings. Workspace Rust formatting checks found pre-existing differences; Clippy could not run because the pinned toolchain lacks its component. No Rust source was changed and no workspace build or test run was needed for this standalone Python diagnostic.

```zsh
uv run --with grpcio==1.76.0 --with protobuf==6.33.0 python -m unittest discover -s scripts/development -p test_geyser_provider_sample.py
```

## Sources checked 2026-10-29

- Tatum public endpoint listing: <https://freerpc.com/solana>
- Tatum's official streaming test client and authentication options: <https://github.com/tatumio/solana-grpc-test>
- Sodae trial: <https://sodae.io/yellowstone-grpc>
- Sodae onboarding: <https://docs.sodae.io/quickstart>
- Sodae endpoint and protocol: <https://docs.sodae.io/yellowstone/overview>
- Yellowstone protobuf schema: <https://github.com/rpcpool/yellowstone-grpc/blob/master/yellowstone-grpc-proto/proto/geyser.proto>
