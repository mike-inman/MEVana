# 2026-10-30 22:20 CDT — Handoff 16: MEVana-UI research page

## Purpose

Handoff 15 produced a validated, read-only Research API, but nothing displays it yet. This slice adds a signed-in research page to MEVana-UI, so the Stage 0 evidence can be read in a browser.

The main risk is the same as in handoff 15: presenting weak evidence as stronger than it is. The page must keep these visible:
- the evidence kind;
- which quote asset each profit is in;
- how much of the candidate set could actually be evaluated;
- that "not measured" is different from zero.

## Scope

**In scope:** MEVana-UI only.

**Out of scope:**
- any change to the MEVana repo, the API, the seed or its pins;
- new `/api` routes, browser fetches or CORS;
- deployment or changing `middleware.ts` and `src/proxy.ts`;
- new npm dependencies;
- charts, filters, sorting, rankings and verdicts;
- mock data or fallback values of any kind.

Do not read, copy or print `.env*` files.

## Execution model

Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/research-page-20261030`, on branch `feat/research-page-20261030`, created from MEVana-UI `main` at `2b66455`. Run `npm ci` there. MEVana-UI has no OpenSpec, so this handoff is the specification.

Baseline on 2026-10-30: `tsc --noEmit`, `eslint src` and `vitest run` (5/5) all pass.

Follow `CODING_STANDARDS.md`, including the 300-line file limit, CSS variables only, no `any`, and a loading state and error boundary for each route.

## Data access

- Add a `research` entry to `ServiceEndpoints` in `src/lib/config.ts`. Use `process.env.BACKEND_RESEARCH_URL || 'http://127.0.0.1:8098'` in both development and production.
- Do not derive it from `DEFAULT_HOST` or `NEXT_PUBLIC_BACKEND_HOST`. Add a `config.test.ts` case for this, including when `BACKEND_HOST` is set.
- Add `src/services/research.ts` with `import 'server-only'`, called only from Server Components. If `server-only` cannot resolve under Vitest, add a test-only alias in `vitest.config.ts`.
- Fetch with `cache: 'no-store'` and a 5-second `AbortSignal.timeout`, using `encodeURIComponent` on the id.
- Check response shapes at runtime with small hand-written guards. A malformed response is treated as an API failure.

The API's error behavior maps to the page like this:

| API result | Page behavior |
| --- | --- |
| Connection failure, timeout, malformed body or unexpected status | "Research API unavailable" state. No numbers are shown. |
| Detail returns 404 | `notFound()` |

The production UI host cannot reach a loopback API on another machine, so there the page will show the unavailable state until a later handoff deploys the API. That is correct behavior. Do not work around it.

## Pages

Add these routes, each with `loading.tsx` and `error.tsx`. Each page repeats the `/mev` guard: `auth()`, then `redirect('/sign-in')` if there is no user.

Add `{ name: "Research", href: "/research" }` as a top-level item in `src/components/Navigation.tsx`.

**`/research`:** lists experiments in the order the API returns them. Do not re-sort. Each entry shows:
- pair or universe scope, with route shapes and pool count;
- `evidenceKind` and `status`;
- quote asset;
- evaluable share for each policy;
- window;
- a link to the detail page.

Also show the dataset's `suitability` value. Use one `GET /research/v1/datasets/{datasetId}` call per distinct dataset.

**`/research/[resultId]`:** shows the list fields and adds:
- a policy table with every `policies` field, including rejection reasons;
- per-policy coverage;
- for v2, `routeCounts`, per-shape coverage and policy results from `shapes`;
- for v2, `poolDirections` rows under a visible "Diagnostic probe: raw input amounts, not chained route amounts" label, with `probeInputAtoms`;
- `limitations`, `sourceCloseout`, version fields and `requestSha256`;
- `request` and `result` in a collapsed `<details>` block.

Use a native `<details>` element, so that the page needs no client component.

## Display rules

These are requirements, not styling preferences:

1. **Evidence kind:** render the API's value on every experiment, in a banner on detail and on each list entry. Never replace it with hard-coded text.
2. **Null:** `missedCandidates` and realized profit render as "Not measured". A null must never render as 0, a dash or an empty cell.
3. **Zero:** a real 0 renders as 0, and is never hidden.
4. **Profit:** convert atoms using `quoteTokenDecimals` with `BigInt` string arithmetic, not floats, and always show the quote asset. For example, `-5191` at 6 decimals is `-0.005191 USDC`. Keep the sign.
5. **Token names:** the API returns no symbols. Map only these three mints, and show any other mint in full:

   | Mint | Symbol |
   | --- | --- |
   | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | USDC |
   | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` | USDT |
   | `So11111111111111111111111111111111111111112` | SOL |

6. **Coverage:** percentages are `evaluableCandidates / candidatesEvaluated`, to two decimals, always alongside both counts. Show every policy. Do not collapse them, even when totals match. If `candidatesEvaluated` is 0, show "Not measured".
7. **Per-shape coverage:** count `state_gap`, `insufficient_state` and `tick_array_absent` as unevaluable. `rejectedCandidates` is an object in `shapes` but an array in `policies`, so handle each form.
8. **No aggregation:** never sum, average, compare or rank across experiments. They use different quote assets and they overlap.
9. **No judgment:** no profitable, successful, winner, best or recommended wording, and no green or red profit coloring.
10. **Scope of 76.11%:** that figure covers the whole universe. Handoff 14's 69.81% is the triangle share only. Label each with its scope.

## Tests

Start the handoff 15 API from `main`, capture the real responses for the list, the dataset, experiment A and the v2 experiment, and commit them as test fixtures. Stop the API afterward. Tests must use these fixtures and must not need a running API.

Run the API with `dotnet MEVana.Research.Api.dll` from its build output. The native launcher does not work on this machine.

Render components with `react-dom/server` `renderToStaticMarkup`, which needs no new dependency. Cover:

- **Expected coverage**, using the `FixedUniverse` policy:

  | Experiment | Quote | Evaluable / evaluated | Share |
  | --- | --- | ---: | ---: |
  | A SOL/USDC | USDC | 66,870 / 67,050 | 99.73% |
  | B SOL/USDT | USDT | 20,061 / 44,700 | 44.88% |
  | C USDC/USDT | USDT | 4,458 / 4,470 | 99.73% |
  | 152-route universe | USDC | 258,564 / 339,720 | 76.11% |
  | Triangle shape | USDC | From `shapes` | 69.81% |

  If a value differs, stop and report it. Do not change the expected value to fit.
- **Rules:** null versus 0, profit formatting including negatives and values under one whole unit, the unknown-mint fallback, and both `rejectedCandidates` forms.
- **Failure states:** connection failure, timeout, malformed body and unexpected status all give the unavailable state with no numbers. A detail 404 calls `notFound()`.
- **Config:** the research URL stays on loopback.
- **Forbidden content:** rendered list and detail output contain no judgment wording from rule 9.

## Validation

Run all of these and report the results:

```bash
npx tsc --noEmit
npx eslint src
npx vitest run
npm run build
git diff --check origin/main...HEAD
```

Also run the page against the real API with `curl` or a server-side check if Clerk allows it. If sign-in blocks this, say so. Claude will do the signed-in browser check.

Confirm that:
- no file over 300 lines was added;
- the build output contains no reference to `8098`, `BACKEND_RESEARCH_URL` or the research service in client chunks (`.next/static`);
- the credential scan passes and no `.env*` file was read or committed;
- nothing is listening on port 8098 or left running.

## Done means

- `/research` and `/research/[resultId]` show the real API data under all ten display rules.
- All tests and checks pass, with no new dependency.
- A dated closeout is in the UI repo's `docs/`, including the coverage table as served.
- The work is committed and pushed to `feat/research-page-20261030`, with a clean worktree.
- Nothing is merged. Claude verifies the branch, checks it in a signed-in browser, and fast-forwards MEVana-UI `main`.

## Later slices

- Deploying the Research API with real authentication, so the production UI can show this page.
- A longer capture on a server host, with better SOL/USDT tick-array coverage.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-10-30-2220-CDT-research-handoff-16-mevana-ui-research-page.md.
Work only in MEVana-UI, in a new worktree at
/Users/michaelinman/Repos/MEVana-worktrees/research-page-20261030 on branch
feat/research-page-20261030 from main at 2b66455. Do not change the MEVana repo, add dependencies,
add /api routes, or read .env files. If a served coverage value differs from the handoff, stop and
report it. Commit and push the branch, but do not merge. Report the validation output, the coverage
values as served, and the final commit hash.
```
