# 2026-10-30 21:43 CDT — Handoff 15a: Pin the Research API seed

## Purpose

Handoff 15 is not ready to merge. Its fail-closed startup checks leave parts of the seed unbound, so altered evidence can load and be served. This follow-up closes that gap on the same branch, `feat/research-api-20261030`, on top of `7689271c`.

This slice is .NET plus OpenSpec only. It changes no Rust, seed bytes, HTTP response shapes or `MEVana.Research.Core`, and it makes no provider calls.

## What was found

The Codex review reported two gaps. Claude reproduced both on a scratch copy of `7689271c`, then found further cases of the same problem. Before the fix, each altered seed below started without error:

| Altered seed | Why it passes today |
| --- | --- |
| `poolDirections` set to null | Served as raw text with no check |
| `probeInputAtoms` changed | Served as raw text with no check |
| Policy counts moved between `triangle` and `two_leg`, totals preserved | Only aggregate totals are reconciled |
| One entry removed from `manifest.json` | Only duplicates are checked, not the required set |
| Simulated profit changed in a v1 result | Nothing binds result bytes; canonical and contract checks still pass |
| Token decimals changed in `catalog.json` | Nothing binds catalog bytes |

Rows 1–4 cover the reported findings. Rows 2, 5 and 6 are additional. Rows 5 and 6 matter most, because they alter the profit and the units the API serves.

## Decision

Pin the SHA-256 of all 11 seed files in compiled code, and verify the pins as the final step of `ResearchArtifactStore.Load`.

One pin check closes every case above. Validating `poolDirections` row by row would not be enough, because moved shape counts that keep their totals are structurally valid. Only a byte pin catches them.

The pins are compiled in rather than stored in `manifest.json`, because an altered manifest must not be able to vouch for itself. This protects against a corrupted, stale or wrongly regenerated seed. It does not protect against someone who can also change the source, which is out of scope for a loopback-only local API.

## Implementation

In `dotnet/src/MEVana.Research.Persistence/ResearchArtifactStore.cs`:

1. Add a private static read-only map of file name to lowercase SHA-256, using `StringComparer.Ordinal` and the values below.
2. In `Load`, after the `duplicate_experiment` check and before returning, read each mapped file through the existing `Read` helper and compare its hash.
3. On any difference, throw `ArtifactIntegrityException("seed_hash_mismatch", file)`.

The pin check must run last. That way, every existing tamper test keeps its current specific failure code.

```text
63788e9af74985533851fbeb6f2d24481d3f6c87e3acadcecb7387ef2b713953  2026-10-30-multi-route-v2-request.json
a4223ade793105e3c060a6b1d711d02993f94161312e8ef44491f5d94bf4dd09  2026-10-30-multi-route-v2-result.json
8edff656b82130ee7006d90687b3a303a85052b0fd20f52482b30b3e32c5f599  2026-10-30-multi-route-v2-summary.json
474dec245a21450366f9339b20d58ffc3962c04f5c99e049e7d33bd2c9a8c373  2026-10-30-snapshot-replay-a-request.json
00d36a8a44df3ed53b1c475b045fc0cf13470e888cefa472fd73265581191a5f  2026-10-30-snapshot-replay-a-result.json
7c20450f5289fa9c1f19804f49e161b4297a900f8ac3ba47b1c5f129ce91117d  2026-10-30-snapshot-replay-b-request.json
bd1bfa458a60934e8b5aa4fdf0eed5ddc89c414ef4d1f176177128582cb0fd47  2026-10-30-snapshot-replay-b-result.json
868c9996c6d491d53c0ddc5adb957b19c796ded330df23d1d000723b977ac414  2026-10-30-snapshot-replay-c-request.json
b211afe876d4769d15b242d6a6ef52e96b86129ea2a6512c2cc3c5a925c07d85  2026-10-30-snapshot-replay-c-result.json
b3c46a902b33eb4f6ff86dd7ddf38c091cb2504a692d0dff45b2158aebc7df02  catalog.json
920844455dbfd6df1064fce50a26005e4158ba482642a8b8425535e904cbf3bc  manifest.json
```

Recompute these from the committed files before using them. If any value differs, stop and report it. Do not change a seed file or adopt a new hash to make it match.

## Tests

Add to `ResearchArtifactStoreTests`, one test per row of the table above. Each must assert `seed_hash_mismatch` through the existing `Fails` helper, and each must change the file's bytes:

- `poolDirections` set to null;
- `probeInputAtoms` changed to `[1,2,3]`;
- `268200` to `268199` plus `71520` to `71521` in the summary's `candidatesEvaluated` values;
- the first `manifest.json` entry removed;
- a `simulatedNetProfitQuoteAtoms` value changed in `2026-10-30-snapshot-replay-b-result.json`;
- a `decimals` value changed in `catalog.json`.

All 294 existing tests must pass without being edited, and the new total must be 300.

Then delete the pin loop temporarily and confirm that exactly the six new tests fail. Restore it and report the result.

## OpenSpec and closeout

- Create and archive a change named `pin-research-seed`.
- Add byte-pinned seed files to the check list in the `research-api` startup requirement, and add a scenario for an altered file that passes every other check.
- Update the handoff 15 closeout: add `seed_hash_mismatch` to the fail-closed table, and state that handoff 15a was required before merge.

## Validation

Run all of these and report the results:

```bash
cd dotnet && dotnet build MEVana.slnx -warnaserror && dotnet test MEVana.slnx
openspec validate --all --strict
./scripts/ci/repo_hygiene.sh
git diff --check origin/main...HEAD
```

Also confirm that:
- nothing under `crates/`, `MEVana.Research.Core` or `dotnet/data/research-artifacts/` changed;
- the existing fixtures are unchanged;
- the non-printing credential scan passes;
- nothing is listening on port 8098.

## Done means

- All six altered seeds fail at startup with `seed_hash_mismatch`.
- 300 of 300 tests pass, with no existing test edited.
- The work is committed and pushed to `feat/research-api-20261030`, with a clean worktree.
- Nothing is merged. Claude verifies the branch and fast-forwards `main`.

## Prompt for Codex

```text
Implement docs/2026-10-30-2143-CDT-research-handoff-15a-pin-research-seed.md in
/Users/michaelinman/Repos/MEVana-worktrees/research-api-20261030 on branch
feat/research-api-20261030. Use OpenSpec. Recompute the 11 hashes from the committed seed and stop
if any differs from the handoff. Run the pin check last in Load, and do not edit existing tests or
seed files. Commit and push the branch, but do not merge. Report the validation output, the result
of the temporary pin-removal check, and the final commit hash.
```
