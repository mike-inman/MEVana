# 2026-10-29 23:54 CDT — Research handoff 10 closeout and validation gate

## Purpose and state

Exact integer Whirlpool quoting and offline golden evaluation are implemented on `feat/exact-whirlpool-quoting-20261030`. This is a research-only branch; no streams, RPC, provider calls, wallets, transactions, or trading were used. The OpenSpec change remains active because the unmodified locked Rust validation is blocked by `ethnum 1.5.2` on the installed compiler. Do not merge this branch until that compatibility decision and validation gate are resolved.

## License and lockfile

`orca_whirlpools_core =1.0.4` and `orca_whirlpools_macros =1.0.4` are direct exact pins with `default-features = false`. Both 1.0.4 crates report Apache-2.0. Core's default `floats` and `wasm` features are off; the quote path uses integer types. The Cargo.lock guard requires both names and refuses any other `orca_*` version. The lockfile adds exactly these two package entries; their other dependencies (`ethnum`, `proc-macro2`, `quote`, `syn`) were already locked, and no already-locked package version changed. No Orca source or documentation code from 2025-02-27 or later was read.

## Decoder and PDA verification

The largest captured DynamicTickArray is 820 bytes raw (six initialized ticks), 247 bytes zstd compressed. It decodes through the new common fixed/dynamic facade; malformed lengths and tag/bitmap mismatches are rejected. The handoff requested a few-KB real dynamic fixture, but none that large was present in the six captures; this is the largest real one. An offline address check found 108 captured tick-array addresses matching decimal-string start-index PDA seeds and zero matching the legacy helper's little-endian seed. The new exact module derives the capture-verified address; the legacy helper and its callers remain untouched.

## Golden evaluation

The six finalized handoff 09 captures produced 1,775 successful single-allowlisted-standard-pool post-states. Of these, 1,660 were evaluable, 1,657 matched both observed vault amounts exactly in at least one of exact-in or exact-out mode, and 3 remained unexplained (0.18% of evaluable cases). Exact matches include 858 A-to-B, 799 B-to-A and 66 initialized-tick crossings. Orca core 1.0.4 returns swap amounts and fee but does not expose next sqrt price, tick or liquidity, so those post-state values could not be compared.

| Classification | Cases |
| --- | ---: |
| Exact match | 1,657 |
| Missing pre-state | 12 |
| Missing intermediate write | 56 |
| Insufficient tick arrays | 6 |
| Multiple swaps through same pool | 1 |
| Non-swap state change | 40 |
| Unexplained | 3 |

The unexplained examples are recorded with full public signatures and slots in `~/MEVanaData/golden/whirlpool-20261029/report-v1.json` (slots 463133730, 463133740 and 463133754). The full versioned JSONL corpus remains outside Git at the same path. No math parameters were tuned to fit these cases.

| Pool prefix | Exact | Crossings | A-to-B | B-to-A |
| --- | ---: | ---: | ---: | ---: |
| `21gTfxAn` | 12 | 0 | 1 | 11 |
| `4fuUiYxT` | 24 | 2 | 2 | 22 |
| `7qbRF6Ys` | 44 | 0 | 2 | 42 |
| `83v8iPyZ` | 823 | 43 | 470 | 353 |
| `AhhoxZDm` | 7 | 0 | 3 | 4 |
| `B6LL9aCW` | 17 | 0 | 3 | 14 |
| `BSddxwYW` | 15 | 0 | 10 | 5 |
| `BmXVhzBH` | 2 | 0 | 0 | 2 |
| `Czfq3xZZ` | 373 | 20 | 226 | 147 |
| `ELqXddRq` | 0 | 0 | 0 | 0 |
| `FpCMFDFG` | 199 | 0 | 90 | 109 |
| `FwewVm8u` | 131 | 1 | 51 | 80 |
| `HcoJqG32` | 10 | 0 | 0 | 10 |

The committed compressed subset is 13 KB with 20 real cases across 8 standard-fee pools and 7 tick spacings. It covers both directions and five initialized-tick crossings. Its test requires exact observed input and output on every case. These are quote-correctness cases, not evidence of opportunities or profitability.

## Step 8 validation

| Check | Result |
| --- | --- |
| `cargo fmt -p whirlpool-core -p geyser-capture -- --check` | Pass |
| `cargo clippy -p whirlpool-core -p geyser-capture --all-targets -- -D warnings` | With the temporary `ethnum` override, fails on two pre-existing `manual_checked_ops` warnings in untouched `quoter.rs`; a scoped run allowing that lint passes. The unmodified locked dependency also blocks compilation. |
| `cargo test -p whirlpool-core -p geyser-capture` | Unmodified locked build fails in existing `ethnum 1.5.2` (`E0512`, `TryFromIntError` size); all 23 focused/offline tests pass with a temporary copy of `ethnum` patched outside the repository |
| Dependent `cargo build -p edge-assembler -p arb-engine -p executor-service` | Passes with that same temporary outside-repository compatibility override; unmodified locked build remains unverified |
| `orca_*` lockfile license test | Passes against restored final lockfile |
| `./scripts/ci/repo_hygiene.sh` | Passes with pre-existing documentation and script-location warnings |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` | Passes, 194 tests |
| `openspec validate --all --strict` | Passes |
| `git diff --check` | Passes |
| Non-printing secret scan of changed files | Passes, zero hits |

## Open risks and next decision

Adaptive-fee pools return `AdaptiveFeeUnsupported`; a later consent or clean-room slice with Oracle capture is needed. Three golden cases remain unexplained, within the handoff's one-percent threshold. The temporary compiler workaround was used only for validation and is not in the branch or final lockfile. The operator must decide a separate remedy for the already-locked `ethnum` compiler incompatibility and the legacy Clippy warnings before exact unmodified validation and OpenSpec archiving. Merging to `main` remains the operator's decision.

## Addendum — 2026-10-30 00:58 CDT: validation gate resolved on the pinned toolchain

The operator verified the cause of the two reported validation failures. This Mac has two Rust installations:
- Homebrew Rust 1.98.1 at `/usr/local/bin/cargo`;
- rustup at `~/.cargo/bin/cargo`.

When `/usr/local/bin` comes first on `PATH`, Homebrew's cargo runs and ignores `rust-toolchain.toml`. The `ethnum 1.5.2` E0512 error and the `manual_checked_ops` lints in the legacy `quoter.rs` occur only on that newer compiler. Neither is a defect of this branch.

The operator re-ran the step 8 Rust commands **unmodified**, with `--locked` and no workaround, on the pinned `nightly-2025-09-15` toolchain (rustc 1.92.0-nightly 2025-09-14) at `1b8e8288`:

| Command | Result |
| --- | --- |
| `cargo test --locked -p whirlpool-core -p geyser-capture` | Pass, 23/23 |
| `cargo clippy --locked -p whirlpool-core -p geyser-capture --all-targets -- -D warnings` | Pass |
| `cargo fmt -p whirlpool-core -p geyser-capture -- --check` | Pass |
| `cargo build --locked -p edge-assembler -p arb-engine -p executor-service` | Pass |

The validation gate is therefore met, with no exceptions and no dependency or legacy-file changes.
- **OpenSpec:** `add-exact-whirlpool-quoting` was archived as `2026-10-30-add-exact-whirlpool-quoting`, and `openspec/specs/whirlpool-quoting/spec.md` was created.
- **Branch:** approved for a fast-forward merge to `main`.
- **Prevention:** `AGENTS.md` and `CLAUDE.md` on `main` now require the rustup toolchain.
