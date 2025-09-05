## Profiles & Registries

This directory contains versioned protocol / token / account registry manifests and related allowlists used by MEVana services.

### Account Registry Files

Versioned account registry files follow the pattern:

```
account_registry.v{N}.json (e.g. account_registry.v1.json, account_registry.v2.json)
```

Guidelines:
1. Increment N by 1 for every promoted (merged) change set.
2. Never rewrite or amend prior versions retroactively—immutability enables reproducible historical diffs.
3. Only keep a small tail of older versions locally if size becomes an issue; CI only needs the latest two for gating.

### Example File

`account_registry.example.json` illustrates structure but contains placeholders. It is **excluded from CI diff / strict validation** to avoid blocking real version promotions. Do not base diffs against it—start from the first real version (`v1`).

### Validation & Diff Workflow

Core validator binary: `account-registry-validate` (in `executor-service`).

Examples:

Validate a single version strictly:
```
cargo run -p executor-service --bin account-registry-validate -- --strict profiles/account_registry.v1.json
```

Diff the two latest versions with JSON Patch output:
```
cargo run -p executor-service --bin account-registry-validate -- \
  --diff --patch-json diff.json profiles/account_registry.v1.json profiles/account_registry.v2.json
```

Exit Codes:
- 0: All provided files valid (and diff acceptable)
- 1: General argument / runtime error
- 2: Validation or policy violation (including unauthorized decimal decreases)

### Decimal Decrease Policy

Reductions in `mint_decimals` are blocked unless allow‑listed.

Allowlist sources merged by the validator:
1. File: `profiles/decimal_decrease_allowlist.txt`
2. Env var: `DECIMAL_DECREASE_ALLOWLIST` (comma separated)

If a decrease is intentional, add the mint to the file (or temporary env var in an emergency) _before_ committing the new version.

### Promotion Steps
1. Copy previous version to new (e.g. `cp account_registry.v1.json account_registry.v2.json`).
2. Apply edits (add/remove pools, accounts, etc.).
3. Run local validation & diff (see above). Inspect `diff.json`.
4. Update allowlist if any intentional decimal decreases.
5. Commit and open PR. CI will:
   - Validate each version
   - Produce a diff (attached artifact & summary)
   - Fail if unauthorized decimal decreases detected

### Future Enhancements (Planned)
- Provenance metadata (who/when) embedded per version
- Optional cryptographic signature / attestations for registry promotion
- Automated PR comment with structured diff summary

---
For questions, open an issue or see the main project README.
