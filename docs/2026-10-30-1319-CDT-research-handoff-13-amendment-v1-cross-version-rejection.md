# 2026-10-30 13:19 CDT — Handoff 13 amendment: v1 cross-version rejection

## Purpose

This amendment resolves the blocker in `docs/2026-10-30-1249-CDT-research-handoff-13-v1-result-linkage-blocker.md`. It supersedes one clause of `docs/2026-10-30-1118-CDT-research-handoff-13-experiment-request-contract-v2.md`. Everything else in that handoff still applies.

## Decision

**v1 stays frozen. A version-2 result checked against a v1 request keeps its existing rejection, `requestCanonicalizationVersion/unsupported_canonicalization_version`.**

Handoff 13, section 5, said a v1 fingerprint must never validate against a v2 request, or the reverse (`request_mismatch`). The clause is amended as follows:

| Result version | Request version | Required outcome |
| --- | --- | --- |
| 1 | v2 | Rejected with `request/request_mismatch` by the new v2 overload |
| 2 | v1 | Rejected with `requestCanonicalizationVersion/unsupported_canonicalization_version` by the unchanged v1 path |

## Why

- The safety requirement is that neither direction validates. Both directions already reject, so only the code differs.
- The code named in the handoff was a drafting error. It missed that v1 result syntax runs first and returns early.
- The freeze is the stronger requirement. The handoff says "no behavior change to v1 in any way", and its exit rule puts that freeze ahead of any conflicting requirement.
- The existing behavior is already contract. `ExperimentResultContractTests.cs:114` pins version 2 to `unsupported_canonicalization_version`, and the `research-experiment-result` spec lists that code.
- The new code would add nothing. `unsupported_canonicalization_version` is the more precise description of a result from another contract version.

## What this means for the branch

Verified against commit `fd508284`:

- `ExperimentResultValidator.cs` needs no change to the v1 paths.
- The test at `ExperimentRequestV2Tests.cs:176` already asserts the amended behavior. Keep it.
- The optional `bool versionTwo = true` parameter on both v2 overloads stays. Existing tests pass a literal `null` to `Validate` and `FromRequest`, so a plain overload would make those calls ambiguous and break v1 source compatibility. Add a one-line comment saying so.

## Required edits

1. In `openspec/changes/experiment-request-v2/specs/research-experiment-request/spec.md`, change the scenario at line 14 to name the exact outcome from the table above. Remove the "request_mismatch or" alternative.
2. Check that `openspec/changes/experiment-request-v2/specs/research-experiment-request-v2/spec.md` covers the first table row, and that `design.md` records this decision.
3. Record this amendment in the closeout.

## Remaining handoff 13 work

- Replace the `PENDING` golden hashes for R1–R3 and add the golden fixture files.
- Finish the tests, including the mirrored v1 tests.
- Confirm whether `ResearchContractJson` parses v1 requests. Section 6 applies only if it does.
- Complete all OpenSpec tasks, which are all still unchecked.
- Run the full validation list, write the closeout, archive the OpenSpec change, and push clean and synchronized.

Constraints are unchanged: .NET and OpenSpec only, no Rust, no network, no credentials, and no `.env`.
