# MEVANA .NET Solution Arrangement

Date: 2026-10-07 15:43 CT. Purpose: record the approved ownership and directory arrangement before generating C# application code.

AgentEngineering remains a separate repository containing the coding-agent controller, planner, developer/reviewer capabilities when implemented, orchestration, and its learning guide. MEVANA owns the application being developed. Its existing Rust core remains in place; new C# services belong under dotnet/, with dotnet/MEVana.slnx grouping the .NET projects. This arrangement does not couple deployment: Rust may run on external bare metal while C# services later run in Azure.

Created from the MEVANA repository root:

```bash
dotnet new sln --name MEVana --format slnx --output dotnet
dotnet sln dotnet/MEVana.slnx list
```

Result: an empty solution was created successfully. No projects found is expected. No services, containers, database, trading integration, or Azure resources were created. No existing Rust source was modified.

Proposed future layout, not yet created:

```text
dotnet/
  MEVana.slnx
  src/
    MEVana.Research.Api/
    MEVana.Research.Core/
  tests/
```

Next: finalize a dated specification for the experiment-request API, resolving persistence, dataset catalog provenance/availability, parameter bounds, idempotency and concurrent submissions, truthful status, and local access boundaries. This initial feature records requests; it does not execute experiments. Review the specification before preparing a bounded coding task with explicit permitted paths. The planner draft in AgentEngineering is evidence and is not an approved implementation specification.

Future agent executions receive an explicit target checkout and permitted files. Independent executions should use isolated worktrees with distinct file ownership and integration order. No coding agent was launched by creating this solution.

The two repositories have separate local commits. GitHub publication remains future work; review tracked content and history before publishing, preserving local credentials and excluding them from publication.
