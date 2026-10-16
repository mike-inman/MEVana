# 2026-10-15 23:10 CDT — Main synchronization conflict inventory

Purpose: assign every merge conflict to exactly one handoff while preserving both branch histories.

- Remote baseline: `7bd165edc61c2657738940cba71bb115f37b896a`
- Local baseline: `7b57c61021f8eed643544c74b58a8b93289eab33`
- Preview command: `git merge-tree --write-tree --name-only <remote baseline> <local baseline>`.
- Total: 128 conflicts: 127 add/add, one content conflict (`.gitignore`).
- This inventory covers conflicting paths only. The final integration handoff must inspect nonconflicting changes too.

## Slice 02 Runtime and code (16 paths)

| Type | Path |
| --- | --- |
| add/add | `crates/_archive/api-service-v2-original/src/main.rs` |
| add/add | `crates/api-service/src/api.rs` |
| add/add | `crates/dex-registry/src/config.rs` |
| add/add | `crates/executor-service/src/bin/replay_export.rs` |
| add/add | `crates/executor-service/src/execution_engine.rs` |
| add/add | `crates/executor-service/src/main.rs` |
| add/add | `crates/executor-service/tests/mtls_reload_binary.rs` |
| add/add | `crates/hotpath_arb_engine/src/main.rs` |
| add/add | `crates/hotpath_arb_engine/src/system_health_monitor.rs` |
| add/add | `crates/kafka_db_writer/src/main.rs` |
| add/add | `crates/monitoring-service/src/main.rs` |
| add/add | `crates/shared-core/src/database_config.rs` |
| add/add | `crates/token-registry-service/src/main.rs` |
| add/add | `crates/wallet-vault/src/main.rs` |
| add/add | `test_hotpath.sh` |
| add/add | `tools/quick_api_server.py` |

## Slice 03 Deployment and operations (47 paths)

| Type | Path |
| --- | --- |
| add/add | `.env.example` |
| add/add | `.github/workflows/self-hosted-deploy.yml` |
| content | `.gitignore` |
| add/add | `config/deployment-architecture.md` |
| add/add | `config/environments/common.env.sample` |
| add/add | `config/multi-stream-docker.yml` |
| add/add | `docker-compose.local.yml` |
| add/add | `infrastructure/docker-compose.core.yml` |
| add/add | `infrastructure/docker-compose.monitoring.yml` |
| add/add | `infrastructure/docker-compose.yml` |
| add/add | `infrastructure/docker-compose.yml.backup` |
| add/add | `infrastructure/docker-compose.yml.old-with-custom-services` |
| add/add | `infrastructure/env.example` |
| add/add | `infrastructure/grafana/datasources/postgres.yml` |
| add/add | `infrastructure/scripts/test-live-arbitrage.sh` |
| add/add | `infrastructure/systemd/wallet-metrics.env.template` |
| add/add | `scripts/auto_setup_dashboard.sh` |
| add/add | `scripts/ci/post-deployment-diagnostics.sh` |
| add/add | `scripts/ci/quick-health-check.sh` |
| add/add | `scripts/ci/run-all-diagnostics.sh` |
| add/add | `scripts/ci/validate-grafana-dashboards.sh` |
| add/add | `scripts/dashboard/deploy-dashboard.sh` |
| add/add | `scripts/dashboard/export-dashboard.sh` |
| add/add | `scripts/deployment/bootstrap_database.sh` |
| add/add | `scripts/deployment/build-and-ship.sh` |
| add/add | `scripts/deployment/deploy-server.sh` |
| add/add | `scripts/deployment/deploy-vultr.sh` |
| add/add | `scripts/deployment/deploy_fast.sh` |
| add/add | `scripts/deployment/deploy_monitoring.sh` |
| add/add | `scripts/deployment/direct_deploy.sh` |
| add/add | `scripts/deployment/release-services.sh` |
| add/add | `scripts/geyser_recovery.sh` |
| add/add | `scripts/lifecycle/start_clean.sh` |
| add/add | `scripts/lifecycle/start_local_stack.sh` |
| add/add | `scripts/lifecycle/start_system.sh` |
| add/add | `scripts/maintenance/backup_postgres.sh` |
| add/add | `scripts/maintenance/sync_remote_db.sh` |
| add/add | `scripts/monitoring/preflight_diagnostics.sh` |
| add/add | `scripts/monitoring/start_local_observability.sh` |
| add/add | `scripts/reset_grafana_clean.sh` |
| add/add | `scripts/setup_grafana_dashboards.sh` |
| add/add | `scripts/setup_grafana_production.sh` |
| add/add | `scripts/testing/run-end-to-end-tests.sh` |
| add/add | `scripts/testing/test-observability-pipeline.sh` |
| add/add | `scripts/testing/test-shyft-apis.sh` |
| add/add | `scripts/testing/test-tickarray-fallback.sh` |
| add/add | `scripts/testing/test-validation-pipeline.sh` |

## Slice 04 Documentation and archive (65 paths)

| Type | Path |
| --- | --- |
| add/add | `CLAUDE.md` |
| add/add | `_archive/scripts/deployment/deploy-multi-stream-FUTURE.sh` |
| add/add | `_archive/scripts/deployment/deploy-multi-stream.sh` |
| add/add | `_archive/scripts/deployment/deploy-single-stream.sh` |
| add/add | `_archive/scripts/deployment/deploy-to-server.sh` |
| add/add | `_archive/scripts/deployment/deploy.sh` |
| add/add | `_archive/scripts/deployment/deploy_working_config.sh` |
| add/add | `_archive/scripts/deployment/docker-deploy.sh` |
| add/add | `_archive/scripts/deployment/rapid_deploy.sh` |
| add/add | `_archive/scripts/deployment/simple-deploy.sh` |
| add/add | `_archive/tests/test-config.toml` |
| add/add | `docs/CHANGES_2025-01-08.md` |
| add/add | `docs/CONTRIBUTING-DASHBOARDS.md` |
| add/add | `docs/DOCUMENTATION_INDEX.md` |
| add/add | `docs/GEYSER_MONITORING_ALERTS.md` |
| add/add | `docs/LIVE_TEST_ANALYSIS_2025-01-08.md` |
| add/add | `docs/LIVE_TEST_SESSION_2025-01-08.md` |
| add/add | `docs/PROMETHEUS_MONITORING_SETUP.md` |
| add/add | `docs/Shyft/congestion_problem.md` |
| add/add | `docs/_archive/MEVANA_CONSOLIDATED_README.md` |
| add/add | `docs/_archive/NEXT_HELPER_NOTES.md` |
| add/add | `docs/_archive/REALISTIC_PROFIT_EXPECTATIONS.md` |
| add/add | `docs/_archive/STARTUP_TROUBLESHOOTING.md` |
| add/add | `docs/_archive/SYSTEM_STARTUP_SUCCESS.md` |
| add/add | `docs/ai/AI_CONTEXT.md` |
| add/add | `docs/ai/CLAUDE.md` |
| add/add | `docs/archive/2025-01-06/SLACK_ALERTING_SETUP.md` |
| add/add | `docs/archive/2025-01-06/chat_history/cursor_help_to_avoid_losing_changes.md` |
| add/add | `docs/archive/2025-01-06/chat_history/cursor_request_for_chat_history.md` |
| add/add | `docs/archive/2025-01-06/chat_history/cursor_request_for_chat_history_08192025.md` |
| add/add | `docs/archive/START_HERE.md` |
| add/add | `docs/claude_notes_datafloww.txt` |
| add/add | `docs/core/README_CONSOLIDATED.md` |
| add/add | `docs/deployment/DEPLOYMENT_PREPARATION.md` |
| add/add | `docs/deployment/DEPLOYMENT_QUICKSTART.md` |
| add/add | `docs/deployment/LOCAL_MONITORING_RUNBOOK.md` |
| add/add | `docs/deployment/LOCAL_TO_PRODUCTION.md` |
| add/add | `docs/deployment/PRODUCTION_DEPLOYMENT.md` |
| add/add | `docs/deployment/PRODUCTION_DEPLOYMENT_RUNBOOK.md` |
| add/add | `docs/deployment/VULTR_DEPLOYMENT_PIPELINE.md` |
| add/add | `docs/deployment/production-deployment.md` |
| add/add | `docs/development/README_AI_AGENTS_SHYFT_APIS_STRATEGIC_FOUNDATION.md` |
| add/add | `docs/notes.txt` |
| add/add | `docs/observability/observability-pipeline-status.md` |
| add/add | `docs/operations/CONFIGURATION_CONSOLIDATION_REPORT.md` |
| add/add | `docs/operations/CURRENT_DEPLOYMENT_CONFIGURATION.md` |
| add/add | `docs/operations/CURRENT_SYSTEM_STATUS.md` |
| add/add | `docs/operations/DEPLOYMENT_RUNBOOK_legacy.md` |
| add/add | `docs/operations/DOCKER_DEPLOYMENT_STRATEGY.md` |
| add/add | `docs/operations/ISP_WORKAROUND.md` |
| add/add | `docs/operations/ISP_WORKAROUND_STATUS_2025-09-13.md` |
| add/add | `docs/operations/LASERSTREAM_PRODUCTION_DEPLOYMENT.md` |
| add/add | `docs/operations/LOCAL_STACK_OVERVIEW.md` |
| add/add | `docs/operations/PRODUCTION_CHECKLIST.md` |
| add/add | `docs/operations/PRODUCTION_DEPLOYMENT_STRATEGY.md` |
| add/add | `docs/operations/PRODUCTION_IMPLEMENTATION_OUTLINE.md` |
| add/add | `docs/operations/PRODUCTION_LESSONS_LEARNED.md` |
| add/add | `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md` |
| add/add | `docs/operations/SERVER_DEPLOYMENT_PLAN.md` |
| add/add | `docs/operations/SHUTDOWN_PROCEDURES.md` |
| add/add | `docs/operations/TOMORROW_MORNING_COMMANDS.md` |
| add/add | `docs/operations/operator-manual.md` |
| add/add | `docs/strategy/orderbook-history-plan.md` |
| add/add | `docs/testing/TESTING_VALIDATION_PROCEDURES.md` |
| add/add | `docs/testing/end-to-end-testing-framework.md` |
