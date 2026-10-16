# 2026-10-16 08:35 CDT — Main sync handoff 01 history and credential review

## Decision

**HOLD** publication of local-only `main` history. The review found credential-shaped historical content that needs targeted verification before a `GO` can be supported. This does not establish that any live credential was exposed. Do not merge local-only history into the integration branch or push it while this decision stands.

## References and scope

- Local `main` at review start: `5964bfba626489cfb8bd5e3cb10909630a38ca79` (354 commits absent from GitHub `main`).
- GitHub `main`: `7bd165edc61c2657738940cba71bb115f37b896a` (340 commits absent from local `main`); `git ls-remote --heads origin main` matched this SHA.
- The scan used `git rev-list --objects main ^origin/main`, which includes historical versions no longer present at the tip. It examined 21,062 objects, including 16,912 blobs. Commit messages in `origin/main..main` were also scanned. The original 351/340 count and 128-conflict preview are historical baselines and require refresh before slice 02.
- At the tips, `private.txt` has the same blob on both branches. The local `.env` is ignored and untracked. Neither fact establishes that prior local-only history is safe.

## Sanitized findings

- 16,864 text blobs were scanned for PEM private-key blocks, common provider-token formats, credential-bearing URLs, and assignments to credential-like names. No common GitHub, AWS, Stripe, or Slack token signature was detected. No matching signature was found in commit messages.
- One local-only blob in `crates/executor-service/tests/mtls_reload_binary.rs` contains a PEM private-key block and certificate. It appears in test code, but whether it is exclusively disposable test material has not been established.
- Six local-only blobs contained 12 URL basic-auth pattern matches, including historical dashboard scripts/examples and dependency examples. The dashboard examples use password-like placeholder wording; each relevant source and its publication history still needs review.
- Generic assignment matching produced 440 matches in 93 blobs, including archived source and chat-history documents. This detector has many code and placeholder false positives. Several candidate lines in historical chat archives and source have not been manually classified. Values were not printed or copied into this report.
- 45 binary blobs and three blobs over 8 MB were excluded from text-pattern scanning. The large exclusions are a generated executable and dependency files. Binary and oversized-content coverage remains a limit.
- A scanner such as Gitleaks, TruffleHog, or detect-secrets was not installed locally. This custom pattern scan is triage, not a comprehensive credential or private-data audit.

## Resolution path

1. Review the flagged historical blob/path categories with value-suppressed tooling and confirm whether the PEM material and credential-shaped strings are disposable examples, already public, or live credentials. Review archived chat content for other private material beyond token patterns.
2. Run a maintained history-aware secret scanner, or document equivalent coverage, against the local-only commit range. Record only sanitized findings and limitations.
3. For any confirmed live credential, identify its owner and a gradual rotation sequence. Preserve existing local values until replacements are verified. Do not delete or overwrite local passwords or API keys under this handoff.
4. If local-only history is unsafe to publish, decide on a reviewed publication approach that retains the original history locally. A normal merge of unsafe commits cannot satisfy the publication gate; do not silently rewrite history.
5. Repeat handoff 01 against the exact proposed publishable history and issue a new documented `GO` before starting slices 02–05.

## Worktree state

No application code, local credentials, or integration worktree content was changed during this review. The report belongs to canonical local `MEVana/docs`; its commit will advance local `main` beyond the review-start SHA and must be included in any later fresh baseline.

## 2026-10-16 owner direction and follow-up

Credential rotation is deferred. Preserve the newest verified values locally and retain all distinct environment entries. The real `.env` is Git-ignored, is absent from both branch trees, and is not a merge conflict or a blocker to local conflict work. The tracked `.env.example` contains replacement-marker values for its credential fields; its fuller local template was reconciled with the remote comment on the integration branch. The `HOLD` applies to publishing unreviewed local-only history, not to keeping or locally merging the current `.env`.

## 2026-10-16 09:00 CDT maintained-scanner follow-up

**Handoff 01 decision: HOLD reaffirmed.** At this scan, local `main` was `66ebf29ab8731468e138b8d107707ed20ac1bc47` and GitHub `main` remained `7bd165edc61c2657738940cba71bb115f37b896a`, confirmed by `git ls-remote`. Divergence was 358 local-only and 340 remote-only commits. The integration worktree was clean and one local commit ahead of its remote tracking branch.

Gitleaks 8.30.1 (default rules) scanned `origin/main..main` with full redaction and no saved raw report. It reported 95 matches across 17 commits and 27 commit/path pairs: 66 generic API-key, 15 curl user-auth, seven curl auth-header, five Slack webhook URL, one Stripe test-token prefix, and one PEM private-key finding. These are matches, not 95 distinct credentials. Every flagged commit/path version mapped to a blob in `git rev-list --objects main ^origin/main`, so none of those exact versions is already reachable from GitHub `main`.

The highest-priority unresolved category is the five Slack webhook URL matches in `SLACK_ALERTING_SETUP.md` and `infrastructure/grafana/provisioning/alerting/contact-points.yaml`; the matched lines did not carry an obvious example or replacement marker. Their validity has not been tested. The PEM finding is in an mTLS test path and the Stripe finding uses a test prefix, but neither observation proves all historical content is safe. The generic matches include generated parser code and archived chat material; the latter also needs private-data review beyond token detection. No credential value was printed, saved in a report, or changed.

Potential owners for any later decision are the Slack/Grafana notification owner, the mTLS test maintainer, and the owners of archived chat/source documentation. Credential rotation remains deferred by the owner's instruction. If any flagged value is real, preserve the current local value and original branch; a normal merge would publish its history. A reviewed sanitized publication approach would then be required. Local path-level conflict preparation for handoff 02 may proceed at the owner's request, but no integration merge or push of local-only history is authorized by this `HOLD`.
