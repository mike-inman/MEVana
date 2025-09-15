AI Onboarding — Read Me First

Last updated: 2025-09-15

Purpose
- Give AI coding agents a 2-minute orientation to MEVana: rules, structure, and safe defaults.

Top 5 Things To Do
- Read AGENTS.md for placement rules and do/don'ts.
- Skim docs/DOCUMENTATION_INDEX.md for critical runbooks and quick links.
- Use scripts in scripts/ (deployment, lifecycle, monitoring, maintenance, testing).
- Keep dashboards only in infrastructure/grafana/dashboards/.
- Run repo hygiene locally before PRs: pre-commit or bash scripts/ci/repo_hygiene.sh

Canonical Structure
- Code: crates/** (Rust workspace)
- Infra: infrastructure/** (Docker compose, Prometheus, Grafana)
- Ops scripts: scripts/** (deployment, lifecycle, maintenance, monitoring, testing)
- Docs: docs/** (operations, deployment, architecture, strategy)
- Dashboards: infrastructure/grafana/dashboards/**

CI Guardrails (fast feedback)
- Repo Hygiene workflow warns/fails for:
  - Dashboards outside infrastructure/grafana/dashboards/
  - docs-new/ folder usage
  - Archives/OS files committed
  - Missing doc timestamps (warn)
  - Scripts outside standard locations (warn)

Safe Defaults For Agents
- Prefer path-agnostic commands (tar -C, git rev-parse --show-toplevel).
- Update existing docs instead of creating new ones; add timestamps.
- Use CODEOWNERS paths; request reviews for infra/docs changes.
- For deploys, use GHCR workflow and compose overlays; do not manually push images from local.

Quick Links
- Rules for assistants: ../../AGENTS.md
- Contribution guide: ../../CONTRIBUTING.md
- Docs index: ../DOCUMENTATION_INDEX.md
- Deploy runbook: ../operations/DEPLOYMENT_RUNBOOK.md
- Critical blockers: ../operations/CRITICAL_PRODUCTION_BLOCKERS.md

