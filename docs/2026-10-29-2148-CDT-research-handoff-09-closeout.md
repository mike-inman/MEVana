# 2026-10-29 21:48 CDT — Handoff 09 partial closeout: capture stopped at stream audit

## Outcome and boundary

The rebased feature branch contains the OpenSpec Revision 3 update, source-derived account layouts, a bounded raw capture crate, inspector, fixed TickArray layout and 14 passing offline capture tests. The 60-second Sodae filter-support check succeeded. **The required after-run socket check failed:** after the recorder exited, `lsof` still showed an established `45.154.33.73:443` socket owned by `Codex (Service)` PID 3646, FD 40. This socket was not owned by the recorder PID 87084, which had exited. Its cause and whether it is still streaming are unknown. No further provider stream was opened. Discovery, allowlist selection, the ten-minute run and real TickArray fixture are pending. This is a partial blocker closeout, not successful handoff completion. The earlier Shyft blocker closeout remains historical.

## Preflight already completed before this session

Sodae trial (primary): 20 seconds, owner-filtered Processed subscription, Yellowstone 4.2.1; 683 ms connect, 208 ms to first account, 1,403 account updates, no errors, last slot 463,112,617. Tatum free plan (secondary): 20 seconds, 423 ms connect, 410 ms to first account, 2,212 updates, no errors, last slot 463,113,490. Shyft returned UNAUTHENTICATED with both historical token aliases and was not used. Neither passing preflight was repeated. Tatum quotas remain unknown; no Tatum bytes were received in this session.

## Filter check and audit

Start command (no secrets): `python3 /tmp/mevana_capture_runner.py filter-check 60 0 --filter-check`, in this worktree. The supervisor ran `target/debug/geyser-capture --provider sodae --duration 60 --output /Users/michaelinman/MEVanaData/captures/handoff09-20261029 --filter-check` in a tracked foreground session; PID 87084 stopped by its hard duration. There was no force kill. Source host was `ams.tls.sodae.io`, tier `dev-test`, Processed commitment. This subscribed to pool and fixed/dynamic TickArray filters with datasize/memcmp alternatives plus the optional transaction filter. The filter request was accepted; no rejected-filter error occurred.

Capture session: `/Users/michaelinman/MEVanaData/captures/handoff09-20261029/capture-1793328155307143000`. One finalized `.cap` file and matching manifest; no `.part`. The manifest reports **929,396 raw protobuf payload bytes** (970,632 framed file bytes), 1,045 pool updates, 813 slot updates, 8 pings and 8 pongs, no gaps or stalls. The check lasted 60.274 seconds inside the process (63.55 seconds supervisor wall time). Peak sampled RSS was 22,503,424 bytes. Inspector verified hash, framing, sequence and manifests; 1,045/1,045 pool decodes succeeded, zero unexpected account families in this deliberately pool-focused run, 213 distinct pools. This 60-second rate (~0.93 MB/min payload, ~0.056 GB/hour or ~1.34 GB/day) is a filter-check sample, **not** the required five/ten-minute projection and not production evidence.

After-run DNS resolved Sodae to `45.154.33.73`; Tatum to `34.160.43.88` and `2600:1901:0:3a76::`. `pgrep -fl geyser` contained build commands whose worktree path includes `geyser`, so exact process-name inspection was used: no `geyser-capture` or `geyser_probe` executable remained; PID 87084 was absent. `lsof -nP -iTCP -sTCP:ESTABLISHED` did show the Codex-owned Sodae socket above, and a repeat check still showed it. The after-run requirement is therefore **not met**. The audit JSON and inspector output remain with the capture under `~/MEVanaData/captures/handoff09-20261029/` and are outside Git. No process launched by this work remains running.

Running raw-payload byte totals for this slice: **Sodae 929,396; Tatum 0; combined 929,396**. TLS/HTTP2/TCP overhead and provider billing counters are unknown, so quota use can exceed payload bytes. A 450 MB combined payload guard leaves headroom below the 0.5 GB slice ceiling. No provider reported quota exhaustion. No discovery or allowlist volumes, rates, decode results or retention projection can be reported honestly yet.

## Account and decoder findings

The local IDL and [current Orca source](https://github.com/orca-so/whirlpools/tree/cc683ec92acb5e7606caa3d8e514d9383e9c7c11/programs/whirlpool/src/state) derive: Whirlpool 653 bytes; fixed TickArray 9,988 bytes with pool offset 9,956; DynamicTickArray `148+112·n` bytes with pool offset 12; Position 216 bytes; Oracle 254 bytes. The detailed calculations and discriminator values are in `docs/2026-10-29-2148-CDT-geyser-account-layouts.md`. Fixed and dynamic array filters are distinct. Dynamic decoding is explicitly unsupported and accounted for. The filter run captured pools only, so it cannot yet verify TickArray decode or compare all five observed preflight size families by actual discriminator.

The decoded pool inventory contained the SOL and USDC mints named in the handoff. It also contained the **actual case-sensitive USDT mint** `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`, rather than the handoff's `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCE8BenwNYb`. Code now uses the captured mint. That triple is strong evidence of mainnet-beta inventory; capture manifests retain `observed_cluster: unknown` because the binary that produced this run used the handoff typo. The five-minute discovery run must confirm the cluster in its own manifest and select every observed SOL/USDC, SOL/USDT and USDC/USDT pool. No allowlist has yet been committed.

`whirlpool-core` now matches the IDL's `Tick` initialized byte and trailing TickArray pool field. It rejects wrong-length fixed arrays. The offline serializer/decoder test passes. Real pool and TickArray fixtures remain pending until capture work resumes. The existing `whirlpool-core/examples/seed_and_log.rs` caller benefits from the fixed layout and stricter rejection; no caller was changed. No math/quoter/tick-window changes were made.

## Validation and remaining work

- `cargo fmt --all`: ran; formatter also touched pre-existing out-of-scope files, which were restored to their original clean state. New code was formatted directly.
- `cargo test -p geyser-capture -p whirlpool-core`: pass, 14 offline capture tests; rerun after the USDT correction passed.
- `cargo clippy -p geyser-capture -p whirlpool-core --all-targets -- -D warnings`: pass after installing the Clippy component for the pinned nightly and fixing new-crate lint findings.
- `cargo build --workspace --locked`: failed in the pre-existing `dex-registry` link, with unresolved `_BIO_eof` and `_OPENSSL_sk_set_thunks` from `rdkafka-sys`. This needs a separate build/toolchain remedy outside handoff scope. The targeted capture binary built and ran.
- `./scripts/ci/repo_hygiene.sh`: completed with warnings about pre-existing dated docs and archived scripts.
- `dotnet test dotnet/MEVana.slnx --verbosity quiet`: pass, 194 tests.
- `openspec validate --all --strict`: pass, six items.
- `git diff --check`: pass. Staged token full/prefix scan: zero tracked matches for Sodae and Tatum, without printing values.

OpenSpec remains active with discovery, fixtures, allowlist capture and final checks unchecked. It cannot be archived truthfully yet. Recommended provisional rotation is 150 seconds for the bounded validation run and a 64 MiB size ceiling; 7/30-day disk planning must use the actual five/ten-minute sample rates, and no retention deletion is authorized here. Open risks include the unexplained Codex-owned Sodae socket, unknown Tatum quotas, dev/test feed completeness, provider filter limits, dynamic decode support, local network gaps and the unrelated workspace OpenSSL link failure. Existing wrong Jupiter ID, port 9898 collision and Redis 6379/6380 mismatch remain follow-ups, outside scope. Captures are public market-data input for later replay; they do not establish profitable opportunities.
