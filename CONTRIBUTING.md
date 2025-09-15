Contributing to MEVana

Thank you for contributing. To keep the repo consistent and reliable, please follow these conventions.

Core rules
- One place per artifact type; no duplicates or new top-level dirs without discussion.
- Dashboards: put all Grafana dashboards in `infrastructure/grafana/dashboards/`.
- Datasources: only in `infrastructure/grafana/datasources/`.
- Infrastructure: Docker Compose, Prometheus, Grafana provisioning under `infrastructure/`.
- Scripts: operational scripts in `scripts/` with subfolders `deployment/`, `maintenance/`, `monitoring/`, `testing/`.
- Docs: use `docs/` only. Do not add `docs-new/`.
- Config examples: put env/config samples under `config/` (e.g., `config/environments/*.env`).

Process
- Keep PRs scoped and descriptive. Use the PR template.
- If adding files, state why the chosen directory is canonical.
- Don’t commit archives, OS metadata, or secrets.

Dashboards
- Add only to `infrastructure/grafana/dashboards/`.
- Verify JSON parses and imports in Grafana locally when possible.
- See `docs/CONTRIBUTING-DASHBOARDS.md`.

Pre-commit
- Optional but recommended: install pre-commit hooks to catch common issues.
- Install: `pipx install pre-commit` or `pip install pre-commit`, then `pre-commit install`.

AI Assistants
- See `AGENTS.md` for strict placement rules and safe defaults.

