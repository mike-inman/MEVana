# research-grafana Specification

## Purpose
Provides a local, read-only SQL and Grafana presentation of validated research experiments while preserving evidence scope, quote assets, missing values and diagnostic limits.

## Requirements

### Requirement: Isolated local research stack
The system SHALL run research Postgres and Grafana in a separate stack bound only to loopback, retain data volumes on stop, and generate local credentials without overwriting existing ones. Grafana SHALL receive only its reader and admin passwords, not the Postgres superuser password.

#### Scenario: Local startup and stop
- **WHEN** an operator starts then stops the research stack
- **THEN** only research containers run, host ports 5434 and 3010 bind to loopback, and named volumes remain.

#### Scenario: Second checkout with retained volumes
- **WHEN** a checkout without `research.env` starts and loads the stack against retained volumes
- **THEN** loading succeeds without deleting volumes, and the existing Grafana admin password is not changed.

### Requirement: Validated seed export
The system SHALL export only artifacts accepted by the pinned research store into one atomic, idempotent SQL load and SHALL write no output if validation fails.

#### Scenario: Tampered artifact
- **WHEN** a seed artifact differs from its pin
- **THEN** the export fails with no SQL output file.

### Requirement: Exact and scoped evidence storage
The system SHALL store experiments, policy and shape outcomes, rejections and diagnostic probes with integer atoms, SQL NULL for unmeasured values, quote metadata and exact numeric conversion views. Profit views SHALL remove trailing scale without rounding.

#### Scenario: Missing profit
- **WHEN** missed or realized values are absent from a validated result
- **THEN** stored and queried values remain SQL NULL and the dashboard shows Not measured.

#### Scenario: Exact profit display
- **WHEN** a simulated profit is queried
- **THEN** its decimal representation has no padded fractional zeroes, including zero as `0`, and carries its quote asset in the dashboard.

### Requirement: Read-only Grafana access
Grafana SHALL connect with a role limited to SELECT on the research schema. Anonymous Viewer access SHALL be enabled only on loopback; wider exposure SHALL require a later handoff with real authentication.

#### Scenario: Viewer queries
- **WHEN** an anonymous viewer opens the provisioned dashboard
- **THEN** its panels can query research views but the database role cannot modify rows or read non-research user tables.

### Requirement: Evidence-aware dashboard
The dashboard SHALL show evidence kind, status, readable scope, quote assets, all per-policy coverage rows and counts, scoped rejection reasons with distinct neutral colors, exact profit, v2 shape coverage and time-free labeled raw diagnostic probes without cross-experiment aggregation or verdicts.

#### Scenario: Mixed scopes
- **WHEN** the universe experiment is selected
- **THEN** its 76.11% whole-universe share and 69.81% triangle share are labeled separately and profit panels have no All option.

#### Scenario: Default and v1 selection
- **WHEN** the dashboard opens
- **THEN** the universe experiment is selected and all 12 coverage rows with two-decimal shares are visible.
- **WHEN** a v1 experiment is selected
- **THEN** shape and probe panels explain that those measurements are unavailable for that experiment.
