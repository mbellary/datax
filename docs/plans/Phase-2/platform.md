# Platform

## Purpose

datax is a Datax app-server that allows developers to build rich UI clients to
Plan, Build, Deploy, Schedule, Execute, and Monitor data-engineering tasks. It
has a first-class integration with a downstream Codex app-server.


## Actors

| Actor | Goal | Notes |
|---|---|---|
| Product user | TBD | Primary human user of Datax |
| Platform Owner | TBD | Owns System Design and steering the development work. |
| Codex | TBD | Builds and maintains the platform |
| External system | TBD | Integrates with downstream Codex app-server |

## Platform Context

```mermaid
flowchart LR
    User["Product User"] --> ClientInterface["Desktop/Web/CLI"]
    ClientInterface --> Datax["Datax app-server"]
    Datax --> Adapter["AgentAdapter"]
    Adapter --> Codex["downstream Codex app-server"]
```

## Platform Boundary

datax owns:

- Product Features required for managing the Data Engineering landscape.
    - Planning, Building, Deployment and Monitoring Data Engineering requirements.
    - Plugins support for Data Engineering Tools.
    - Worktrees to keep parallel task changes isolated with built-in Git worktree support.
    - Git worktree Support.
    - Automations for scheduling recurring tasks , or wakeup the same thread for ongoing checks.
    - Data Engineering Skills for reusing instructions and workflows in the application.
    - Data Engineering Workflows.
    - Context usage and Compaction.
    - Task Scheduler and Runner support for scheduling and executing tasks.
    - Adapter support for log collection from deployed cloud services.
    - Unified Monitoring support of the DE pipeline.
- Datax app-server protocol for clients.
- First-class Datax integration with downstream Codex app-server through a
  Datax-owned adapter boundary.
    - `AgentAdapter` interface.
    - `codex-runtime` implementation boundary for downstream Codex app-server
      process/client/protocol integration.
- Persist Datax domain state separately.
- Codex events as inputs to Datax projections, not as Datax's entire model.
- Owns the data-engineering product; Codex powers the agentic work inside it.
- Owns the .codex workspace. CLI/Web/Desktop application owns the .datax workspace.




datax integrates with:

- Upstream : Integrates with the client Interface.
- Downstream : Integrates with the downstream Codex app-server through
  `AgentAdapter` and `codex-runtime`.

datax does not own:

- Client Interface development and deployment.

datax traps:
- Do not expose Codex Thread/Turn/Item directly in Datax APIs.
    
    That will re-create the coupling you are trying to escape.
- Do not make Codex history the source of truth for Datax.
    
    Codex history is agent interaction history. Datax also needs durable product state: plans, workflows, schedules, deployments, run records, monitors, approvals, artifacts.
- Do not overfit the first Datax model to Codex UI assumptions.
    
    Codex is optimized for software-agent sessions. Datax is a data-engineering control plane with agent support.
- Do not fork too much Codex code too early.
    
    Reuse concepts and crates where practical, but keep a clean boundary so upstream Codex changes do not constantly break Datax.
- "reuse Codex" means Datax is mostly a thin semantic wrapper around Codex Thread/Turn/Item.



## Open Questions

- Which users and systems are in scope for the first release?
- Which integrations are mandatory versus optional?
- Which service owns each data contract?
