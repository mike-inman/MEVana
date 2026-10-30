# 2026-10-29 21:51 CDT — Background Codex factory worker stop audit

Created: 2026-10-29

## Request and scope

Michael requested that none of his open projects run background Codex factory jobs, specifically the background factory instances. This action did not stop interactive Codex sessions, the desktop app, MCP servers, ordinary development servers or tests.

## Inspection and action

Inspected the local process table, open-project inventory, user/system launch-agent definitions, current crontab and local Codex automation definitions for factory launchers. No active `codex exec` worker or factory lane/runner/watchdog process was found. Existing bare Codex CLI processes were traced to interactive VS Code terminal shells and left running.

The following user launch agents were disabled persistently with `launchctl disable`. Loaded agents were unloaded with `launchctl bootout`:

| Agent | Previously loaded | Final state |
| --- | --- | --- |
| `com.agentengineering.factory.lanes` | No | Disabled and unloaded |
| `com.mediaraq.factory.lanes` | Yes | Disabled and unloaded |
| `com.jagertrailcams.jagervision.factory.lanes` | Yes | Disabled and unloaded |
| `com.jagertrailcams.jagervision.factory-watchdog` | No | Disabled and unloaded |

Post-action verification found zero `codex exec` processes and zero factory lane/runner/watchdog processes. `launchctl print-disabled` confirmed all four overrides.

Factory dashboard servers were left running. Factory nightly test schedules and the MediaRaq automatic deployment service were left unchanged: inspection showed test/backlog and deployment functions rather than Codex worker launchers. The AgentEngineering nightly cron entry also remains. Disabled worker launchers prevent the inspected automatic lane services from consuming backlog jobs. This does not prevent a person or future session from explicitly starting a new worker or reinstalling/enabling a launcher.

No matching local Codex factory automation configuration was found. No local factory launcher was found for MEVana or AI-Influencer-Factory in the inspected launch-agent definitions. This is a local-machine audit; external machines and hosted CI schedules were not audited.

## Recovery and preservation

Before changing service state, copied the four launch-agent plists and saved prior disabled-state output under:

`/Users/michaelinman/.codex/backups/2026-10-29-215149-CDT-factory-worker-launchers`

The backup also contains `actions.json` recording prior loaded state and verification results. Original launch-agent files, worktrees, job records, project changes and credentials were preserved. No factory job was deleted.

Re-enabling requires a new user instruction. For an agent that was previously loaded, restoration would involve `launchctl enable gui/<uid>/<label>` followed by `launchctl bootstrap gui/<uid> <original-plist-path>`. Consult the recorded previous disabled state before restoring agents that were already unloaded; do not automatically start all four.
