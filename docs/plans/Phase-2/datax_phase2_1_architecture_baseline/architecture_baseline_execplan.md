# Phase 2.1 Architecture Baseline

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document follows `PLANS.md` from the repository root. It is self-contained so a contributor can restart Phase 2.1 from this file alone.

## Purpose / Big Picture

Phase 2.1 establishes the architectural baseline for Datax before adapter implementation begins. After this phase, a contributor can point to one checked-in plan that says which code is Datax public app-server surface, which code still violates the Phase 1 migration intent, where `AgentAdapter` should fit, and where a downstream Codex app-server integration must be isolated later. This phase intentionally does not add runtime behavior; the observable outcome is a reviewable architecture inventory and validation record that makes Phase 2.2 through Phase 2.8 safer to implement.

In this document, Datax app-server means the server in this repository that speaks the public Datax protocol to clients such as CLI, TUI, desktop, or web. Agentic work means work delegated to an AI coding/runtime engine. `AgentAdapter` means the future Datax-owned interface the app-server will call when agentic work is needed. Downstream Codex app-server means an external or separately managed Codex runtime service hidden behind `AgentAdapter` and a later `codex-runtime` boundary.

## Baseline

The starting branch for this phase is `datax/phase2-1-architecture-baseline`, created from `main`. The starting commit observed for this phase is `7754dfc1c9`. The expected prior phase is Phase 1, the fork-first migration baseline. The user reported that Phase 1 tests are still running and will provide results when complete, so this phase records the validation state as pending rather than claiming a pass.

The active Phase 2 roadmap is `docs/plans/Phase-2/Provisional-Datax-Migration-Plan-Phase2.md`. The product direction is `docs/plans/Phase-2/platform.md`, and the per-phase execution rules are `docs/plans/Phase-2/Recommended-Datax-Phase2-Execution-Model.md`. Phase 2.1's required artifact is this file at `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md`.

Important correction: Phase 1 was expected to migrate the app-server-facing runtime model to Datax primitive `Chat`, `Interaction`, and `Message` concepts. Therefore, direct inherited runtime machinery such as `datax-rs/core`, `datax-rs/core-api`, `ThreadManager`, `CodexThread`, `ThreadId`, `TurnItem`, and `RolloutItem` in Datax app-server paths is not an acceptable steady-state Phase 2 baseline. It is a Phase 1 migration gap that must be worked out before Phase 2 proceeds to downstream boundary inventory, adapter contracts, or runtime skeleton implementation.

## Progress

- [x] (2026-07-09 00:00Z) Read `PLANS.md` and the active Phase 2 planning files under `docs/plans/Phase-2/`.
- [x] (2026-07-09 00:00Z) Created branch `datax/phase2-1-architecture-baseline` from `main` at `7754dfc1c9`.
- [x] (2026-07-09 00:00Z) Inspected app-server, app-server protocol, runtime, persistence, CLI/TUI, adapter-candidate, and downstream Codex-boundary candidate files.
- [x] (2026-07-09 00:00Z) Recorded a Phase 2.1 file inventory with `Modified` values resolved for the planning-only scope.
- [x] (2026-07-09 00:00Z) Classified retained Codex references touched by this phase in the Codex Boundary Register.
- [x] (2026-07-09 00:00Z) Ran lightweight validation commands: `git status --short --branch`, `git diff --check`, and focused `rg` checks for product spelling and retained boundary terms.
- [x] (2026-07-09 00:00Z) Created GitHub issue `https://github.com/mbellary/datax/issues/15` for Phase 2.1.
- [x] (2026-07-09 00:00Z) Created draft pull request `https://github.com/mbellary/datax/pull/16` for Phase 2.1.
- [x] (2026-07-09 00:00Z) Recorded user correction that direct inherited runtime machinery in the Datax app-server path is a Phase 1 migration gap and a blocker before later Phase 2 work.
- [x] (2026-07-09 00:00Z) Added `runtime_machinery_remediation_execplan.md` to define the remediation gate and implementation slices before Phase 2.2.
- [ ] Record the user's Phase 1 validation results after the still-running tests complete.
- [ ] Record user-provided results for deferred format, lint, and test commands if they are run outside this phase.

## Surprises & Discoveries

- Observation: The public app-server protocol already uses Datax-shaped `chat/*`, `interaction/*`, and `message/*` methods, but the Rust type internals still retain inherited names such as `ThreadId`, `TurnItem`, `CodexThread`, and `ThreadManager`.
  Evidence: `datax-rs/app-server-protocol/src/protocol/common.rs` registers `chat/start`, `chat/resume`, `interaction/start`, `interaction/steer`, and `message/*` notifications, while `datax-rs/app-server/src/request_processors/chat_processor.rs` and `datax-rs/app-server/src/request_processors/interaction_processor.rs` still call `datax_core::ThreadManager` and hold `Arc<CodexThread>`.
- Observation: The app-server crate still has a direct dependency on `datax-core`, and the facade crate still preserves inherited thread-management identity.
  Evidence: `datax-rs/app-server/Cargo.toml` lists `datax-core = { workspace = true }`; `datax-rs/core-api/src/lib.rs` says it is a public facade for thread management APIs built on `codex-core` and re-exports `CodexThread`, `ThreadManager`, `NewThread`, `StartThreadOptions`, `ThreadShutdownReport`, and `ThreadId`.
- Observation: The v2 protocol is implemented as a directory, not a single `v2.rs` file.
  Evidence: `datax-rs/app-server-protocol/src/protocol/v2/mod.rs` re-exports files such as `chat.rs`, `interaction.rs`, `message.rs`, and `thread_data.rs`.
- Observation: A dedicated `codex-runtime` crate does not exist at the Phase 2.1 baseline.
  Evidence: `find datax-rs -maxdepth 2 -type f -name Cargo.toml` lists many crates, including `datax-rs/core`, `datax-rs/app-server`, and `datax-rs/thread-store`, but no `datax-rs/codex-runtime/Cargo.toml`.

## Decision Log

- Decision: Treat this phase as documentation and inventory only.
  Rationale: The Phase 2 roadmap explicitly says Phase 2.1 should not implement runtime behavior. Adding code now would blur the boundary before the inventory is complete.
  Date/Author: 2026-07-09 / Codex
- Decision: Record current app-server protocol files as Datax-owned public surface even when some Rust type aliases and comments still use inherited thread or turn terminology.
  Rationale: Public JSON-RPC methods already use `chat`, `interaction`, and `message` names; remaining internal names are migration debt to classify before changing.
  Date/Author: 2026-07-09 / Codex
- Decision: Treat direct inherited runtime machinery in Datax app-server-facing Chat, Interaction, and Message paths as a blocker rather than ordinary Phase 2 debt.
  Rationale: The user clarified that Phase 1 was intended to migrate everything to Datax primitives. Carrying `datax-rs/core` and thread/turn/item runtime machinery forward as the accepted app-server substrate would undermine the product-boundary architecture before adapter work begins.
  Date/Author: 2026-07-09 / Codex
- Decision: Defer creation of `AgentAdapter` and `codex-runtime` code until the Phase 2.1 runtime-machinery blocker has a remediation plan and acceptance gate.
  Rationale: Adding an adapter on top of unremediated inherited runtime machinery would preserve the wrong dependency direction and make the downstream boundary appear to be the center of Datax.
  Date/Author: 2026-07-09 / Codex

## Outcomes & Retrospective

At this checkpoint, Phase 2.1 has established the architecture inventory and baseline decisions needed to stop later phases from building on the wrong substrate. The most important result is corrective: direct use of inherited `datax-rs/core` runtime machinery in the Datax app-server-facing Chat, Interaction, and Message paths is now recorded as a blocker. The remaining work is to add the user's Phase 1 validation results, record validation command results, and complete a remediation plan before Phase 2.2 starts. No implementation code has been changed.

## Context and Orientation

Datax currently exposes app-server functionality through `datax-rs/app-server` and `datax-rs/app-server-protocol`, but the implementation path still reaches into inherited runtime machinery in `datax-rs/core`. The Datax app-server accepts public JSON-RPC requests from clients, dispatches those requests through request processors, and uses `datax_core::ThreadManager` plus `datax_core::CodexThread` to run the inherited agent session implementation. That direct dependency is contrary to the corrected Phase 1 migration expectation and must be remediated before later Phase 2 work.

The public protocol lives mostly in `datax-rs/app-server-protocol/src/protocol/common.rs` and the v2 protocol directory `datax-rs/app-server-protocol/src/protocol/v2/`. Public methods already include Datax-facing names such as `chat/start`, `chat/resume`, `chat/interactions/list`, `interaction/start`, `interaction/steer`, `interaction/interrupt`, and `message/*` notifications. The app-server processing path for chat creation and resumption is in `datax-rs/app-server/src/request_processors/chat_processor.rs`. Interaction start, steer, interrupt, and message injection are in `datax-rs/app-server/src/request_processors/interaction_processor.rs`. Per-chat runtime state is tracked by `datax-rs/app-server/src/thread_state.rs` and status projection by `datax-rs/app-server/src/thread_status.rs`.

The inherited runtime boundary is currently not isolated behind a Datax-owned adapter. Instead, `datax-rs/app-server/src/message_processor.rs` constructs a `ThreadManager`, app-server request processors accept that manager directly, and `datax-rs/core-api/src/lib.rs` re-exports inherited runtime types. Persistence is represented today by `datax-rs/thread-store`, `datax-rs/rollout`, and state database access in `datax-rs/state` and `datax-rs/core`. These references are no longer treated as acceptable baseline debt. They are remediation targets that must either move behind a Datax primitive boundary or be removed from the app-server-facing Chat, Interaction, and Message paths before later Phase 2 implementation.

## File Inventory

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md` | `Completed` | Required Phase 2.1 artifact; records baseline, inventory, boundaries, and validation state. |
| `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/runtime_machinery_remediation_execplan.md` | `Completed` | Companion remediation plan; blocks Phase 2.2 until direct inherited runtime machinery has a Datax primitive boundary. |
| `docs/plans/Phase-2/Provisional-Datax-Migration-Plan-Phase2.md` | `Not Required` | Inspected as active Phase 2 roadmap; no edit required for Phase 2.1. |
| `docs/plans/Phase-2/Recommended-Datax-Phase2-Execution-Model.md` | `Not Required` | Inspected for required tracking sections and file inventory format; no edit required. |
| `docs/plans/Phase-2/platform.md` | `Not Required` | Inspected for product boundary and actor context; no edit required. |
| `PLANS.md` | `Not Required` | Inspected for ExecPlan format and living document requirements; no edit required. |
| `datax-rs/app-server/src/lib.rs` | `Not Required` | Datax app-server entry module and transport orchestration owner; no implementation edit in Phase 2.1. |
| `datax-rs/app-server/Cargo.toml` | `Not Required` | Directly depends on `datax-core`; remediation target before Phase 2.2. |
| `datax-rs/app-server/src/message_processor.rs` | `Not Required` | Constructs and dispatches through `ThreadManager`; blocker evidence for inherited runtime machinery in the app-server path. |
| `datax-rs/app-server/src/request_processors/chat_processor.rs` | `Not Required` | Main `chat/start`, `chat/resume`, chat listing, and chat lifecycle processor; blocker evidence because the Datax path still uses inherited runtime calls. |
| `datax-rs/app-server/src/request_processors/interaction_processor.rs` | `Not Required` | Main `interaction/start`, `interaction/steer`, and `interaction/interrupt` processor; blocker evidence because Datax interactions still route through inherited runtime machinery. |
| `datax-rs/app-server/src/request_processors/thread_lifecycle.rs` | `Not Required` | Current inherited thread lifecycle helper; remediation target before Phase 2.2. |
| `datax-rs/app-server/src/request_processors/thread_summary.rs` | `Not Required` | Builds Datax-shaped chat summaries and notifications from inherited runtime state. |
| `datax-rs/app-server/src/thread_state.rs` | `Not Required` | Holds active `CodexThread` listener and turn state; likely candidate for future Datax runtime-link separation. |
| `datax-rs/app-server/src/thread_status.rs` | `Not Required` | Projects runtime/chat status for clients; Datax-owned status surface with inherited naming debt. |
| `datax-rs/app-server/src/dynamic_tools.rs` | `Not Required` | Uses `CodexThread` directly for dynamic tool output; downstream runtime candidate to revisit after adapter contract exists. |
| `datax-rs/app-server/src/config/external_agent_config.rs` | `Not Required` | Existing external-agent configuration area; inspect in later phases before deciding whether it belongs to `AgentAdapter` or compatibility work. |
| `datax-rs/app-server/src/request_processors/external_agent_config_processor.rs` | `Not Required` | Existing external-agent config request processor; future adapter configuration candidate. |
| `datax-rs/app-server/src/request_processors/external_agent_session_import.rs` | `Not Required` | Existing external-agent session import path; classify further in Phase 2.2. |
| `datax-rs/app-server-protocol/src/protocol/common.rs` | `Not Required` | Registers public JSON-RPC methods and notification names; Datax public protocol owner. |
| `datax-rs/app-server-protocol/src/protocol/v2/mod.rs` | `Not Required` | Re-exports v2 protocol modules; confirms v2 is a directory, not a single file. |
| `datax-rs/app-server-protocol/src/protocol/v2/chat.rs` | `Not Required` | Defines `ChatStartParams`, `ChatStartResponse`, resume/list/read payloads, and chat notifications. |
| `datax-rs/app-server-protocol/src/protocol/v2/interaction.rs` | `Not Required` | Defines `InteractionStartParams`, `InteractionStartResponse`, steer, interrupt, and interaction notifications. |
| `datax-rs/app-server-protocol/src/protocol/v2/message.rs` | `Not Required` | Defines message-shaped public protocol payloads that map inherited runtime items to Datax messages. |
| `datax-rs/app-server-protocol/src/protocol/v2/thread_data.rs` | `Not Required` | Current v2 module with inherited thread-data naming; candidate for Phase 2.2 classification before any rename. |
| `datax-rs/app-server-protocol/src/protocol/thread_history.rs` | `Not Required` | Converts persisted rollout/runtime items into `Interaction` and `Message` values; likely adapter mapping reference. |
| `datax-rs/app-server-protocol/src/protocol/event_mapping.rs` | `Not Required` | Maps runtime events to Datax public notifications; future adapter event mapping reference. |
| `datax-rs/app-server-client/src/lib.rs` | `Not Required` | Datax app-server client crate; upstream-facing client boundary. |
| `datax-rs/app-server-daemon/src/lib.rs` | `Not Required` | Datax app-server daemon support; launch and lifecycle owner for Datax app-server, not downstream Codex app-server. |
| `datax-rs/cli/src/main.rs` | `Not Required` | CLI entry point and Datax client surface; no Phase 2.1 implementation edit. |
| `datax-rs/tui/src/app.rs` | `Not Required` | TUI orchestration surface; future public behavior consumer but not changed in Phase 2.1. |
| `datax-rs/core-api/src/lib.rs` | `Not Required` | Re-exports inherited runtime types such as `CodexThread`, `ThreadManager`, and `ThreadId`; blocker evidence if used by Datax app-server-facing paths. |
| `datax-rs/core/src/lib.rs` | `Not Required` | Main inherited runtime crate; must not be the accepted Datax app-server runtime substrate. |
| `datax-rs/thread-store/src/lib.rs` | `Not Required` | Existing thread persistence interface; future runtime-link and product-state separation candidate. |
| `datax-rs/thread-manager-sample/src/main.rs` | `Not Required` | Sample that runs one inherited turn through `ThreadManager`; downstream runtime/provenance candidate. |
| `datax-rs/external-agent-sessions/Cargo.toml` | `Not Required` | Existing external-agent session crate; inspect in Phase 2.2 for compatibility or adapter relevance. |
| `datax-rs/external-agent-migration/Cargo.toml` | `Not Required` | Existing migration crate for external-agent data; inspect in Phase 2.2 before reuse or rename. |
| `datax-rs/codex-runtime/Cargo.toml` | `Not Required` | Does not exist at Phase 2.1 baseline; possible Phase 2.4 crate location. |
| `datax-rs/agent-adapter/Cargo.toml` | `Not Required` | Does not exist at Phase 2.1 baseline; possible Phase 2.3 crate location if app-server-local module is insufficient. |

## Architecture Boundary Baseline

The Datax app-server is the public boundary in both directions. Upstream-facing means it receives requests from Datax clients such as CLI, TUI, desktop, or web over JSON-RPC transports. Downstream-facing means it should eventually call only a Datax-owned `AgentAdapter` contract when agentic runtime work is required.

The current app-server still calls inherited runtime types directly. This is not acceptable as the Phase 2 substrate. Phase 2.1 records this as a blocker because the Phase 1 migration intent was to make Datax primitive `Chat`, `Interaction`, and `Message` concepts the app-server-facing runtime model. Later phases must not proceed until direct `ThreadManager`, `CodexThread`, `ThreadId`, `TurnItem`, and `RolloutItem` dependencies in app-server-facing paths are either removed or explicitly contained behind a Datax primitive compatibility boundary.

The downstream Codex app-server must be treated as an implementation detail behind `AgentAdapter` and `codex-runtime`. Public Datax APIs should continue to use `Chat`, `Interaction`, and `Message` concepts. Downstream runtime concepts such as Codex `Thread`, `Turn`, and `Item` may exist inside compatibility, persistence, or runtime mapping code until later phases classify and isolate them, but they must not be introduced as new public Datax API concepts.

## Stop-The-Line Remediation Gate

Before Phase 2.2 starts, Phase 2.1 must produce a concrete remediation path for the inherited runtime machinery still present in app-server-facing paths. The remediation path may be staged, but it must make the dependency direction explicit and must not normalize `datax-rs/core` as the Datax runtime center.

Minimum remediation acceptance criteria:

- `datax-rs/app-server` no longer treats `datax_core::ThreadManager` or `datax_core::CodexThread` as the app-server-facing Chat, Interaction, and Message model.
- Any temporary compatibility layer is named and documented as Datax compatibility, not as a new `codex-core` or inherited runtime boundary.
- App-server request processors consume Datax primitive concepts at their boundary: chat id, interaction id, message payloads, status, and runtime link metadata.
- Runtime event mapping from inherited `TurnItem`, `RolloutItem`, or `EventMsg::Turn*` into Datax messages is isolated to a named translation module or crate.
- Persistence distinguishes Datax product identifiers from downstream runtime identifiers, even if the first implementation keeps a transitional mapping.
- Later `AgentAdapter` and `codex-runtime` work depends on the Datax primitive boundary, not directly on `datax-rs/core`.

## Current Module Ownership Classification

Datax-owned public boundary candidates are `datax-rs/app-server`, `datax-rs/app-server-protocol`, `datax-rs/app-server-client`, `datax-rs/app-server-daemon`, `datax-rs/cli`, and `datax-rs/tui`. These are the places where Datax clients and operators observe behavior.

Current remediation targets are `datax-rs/core`, `datax-rs/core-api`, `datax-rs/protocol`, `datax-rs/rollout`, `datax-rs/thread-store`, and runtime-facing portions of `datax-rs/app-server/src/request_processors`. These names are targets because they currently model sessions as threads, turns, rollout items, and `CodexThread` handles inside the Datax app-server path.

Future `AgentAdapter` candidate locations are a small app-server-local module, a new `datax-rs/agent-adapter` crate, or a narrow protocol-adjacent module if Phase 2.3 proves the types must be shared across crates. The default choice for Phase 2.3 should be the smallest location that prevents Datax public API from importing downstream Codex runtime types.

Future `codex-runtime` candidate location is `datax-rs/codex-runtime`. It should own downstream Codex app-server lifecycle, status, and protocol mapping if Phase 2.4 creates the crate. Datax app-server code should not directly start, stop, restart, or monitor the downstream Codex app-server once this boundary exists.

Persistence candidates are `datax-rs/thread-store`, `datax-rs/rollout`, `datax-rs/state`, and any state database calls in `datax-rs/app-server`. Phase 2.6 should introduce Datax product records and runtime-link records so downstream runtime identifiers do not become Datax primary identifiers.

## Codex Boundary Register

| Reference | Classification | Notes |
| --- | --- | --- |
| `datax-rs/app-server` dependency on `datax-core` | `Phase 1 migration gap` | Direct dependency in `datax-rs/app-server/Cargo.toml`; must be removed or contained before later Phase 2 work. |
| `datax_core::CodexThread` in app-server request processors and state | `Phase 1 migration gap` | Current inherited in-process runtime handle; must not remain the app-server-facing Datax runtime model. |
| `datax_core::ThreadManager` in app-server processors and `message_processor.rs` | `Phase 1 migration gap` | Current runtime/session manager; must be replaced by or contained behind a Datax primitive boundary before adapter work. |
| `datax_protocol::ThreadId` in app-server and protocol files | `Phase 1 migration gap / compatibility candidate` | Currently used as durable id type behind public `chat_id` fields; remediation must preserve persisted identity safely. |
| `TurnItem`, `RolloutItem`, and `EventMsg::Turn*` in protocol mapping files | `Phase 1 migration gap / compatibility candidate` | Used to map inherited runtime events into Datax `Interaction` and `Message` values; should move behind an explicit translation boundary. |
| `datax-rs/core-api` re-exports of `CodexThread`, `ThreadManager`, and `ThreadId` | `Phase 1 migration gap` | The facade still advertises inherited thread-management APIs and cannot be treated as the Datax primitive API. |
| `Codex` in `codex-runtime` planning term | `downstream runtime` | Intentional name for the future boundary that integrates with the downstream Codex app-server. |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`, `CODEX_SANDBOX_ENV_VAR`, `CODEX_SANDBOX_NETWORK_DISABLED`, and `CODEX_SANDBOX` | `protected sandbox exception` | Must not be modified or renamed during Phase 2. |
| Generated schema file names containing `codex_app_server_protocol` | `compatibility shim` | Existing generated artifact naming; do not mechanically rename in Phase 2.1. |
| Crate names prefixed with `codex-` in Cargo packages | `external dependency` | Repository convention says crate package names retain the `codex-` prefix even in Datax folder names. |

## Public Surface Checklist

CLI is inspected but not changed. Config is inspected only for existing external-agent and inherited runtime references; it is not changed. Datax app-server protocol is inspected but not changed. Generated schemas and TypeScript bindings are not changed. Persisted state is inspected at the crate boundary level but not changed. UI text is not changed. Packaging and release metadata are not changed. App-server runtime behavior is not changed.

The public surface rule for this phase is: no new Datax public API may expose Codex `Thread`, `Turn`, or `Item`, and no later Phase 2 implementation may use direct inherited runtime machinery as the Datax app-server-facing substrate. Since Phase 2.1 adds only this planning file, it introduces no new public runtime surface, but it does block later phases until the remediation gate is satisfied.

## Dependency Order

The safe order for Phase 2.1 is to first record the architecture baseline, then link operational tracking artifacts, then record validation results, then resolve this blocker before Phase 2.2. No source edits, generated artifacts, schema updates, or runtime tests are required before the plan exists. Later phases must use this corrected order: remediate inherited runtime machinery first, inventory downstream Codex boundary second, adapter contract third, runtime skeleton fourth, app-server mediation fifth, persistence split sixth, product-domain skeleton seventh, and end-to-end smoke last.

If Phase 2.1 needs an update after user validation arrives, update the `Baseline`, `Progress`, `Validation Matrix`, `Validation and Acceptance`, and `Outcomes & Retrospective` sections together so the plan remains self-contained.

## Plan of Work

Create this ExecPlan under the required Phase 2.1 directory and keep all code files unchanged. Read the active Phase 2 roadmap and execution model. Inspect the app-server, protocol, runtime, persistence, CLI, and TUI files listed in the file inventory. Classify current modules as Datax public boundary, Phase 1 migration gap, compatibility candidate, persistence candidate, adapter candidate, or downstream Codex-boundary candidate. Record the user's Phase 1 test result when available. Create a GitHub issue and draft pull request for the Phase 2.1 artifact, and link them in `Artifacts and Notes` when their URLs exist.

The next action before Phase 2.2 is a remediation plan that answers which concrete files will stop importing direct inherited runtime machinery and what transitional Datax primitive boundary will replace those imports.

That remediation plan is recorded in `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/runtime_machinery_remediation_execplan.md`.

## Concrete Steps

From the repository root, confirm branch and baseline:

    git status --short --branch
    git rev-parse --short HEAD

Expected evidence at phase start:

    ## datax/phase2-1-architecture-baseline
    7754dfc1c9

Inspect the plan and relevant modules:

    sed -n '1,260p' PLANS.md
    sed -n '1,260p' docs/plans/Phase-2/Recommended-Datax-Phase2-Execution-Model.md
    sed -n '1,260p' docs/plans/Phase-2/Provisional-Datax-Migration-Plan-Phase2.md
    sed -n '1,220p' datax-rs/app-server-protocol/src/protocol/v2/chat.rs
    sed -n '1,240p' datax-rs/app-server-protocol/src/protocol/v2/interaction.rs
    sed -n '1,220p' datax-rs/app-server/src/request_processors/chat_processor.rs
    sed -n '1,220p' datax-rs/app-server/src/request_processors/interaction_processor.rs

After the plan is updated, run the validation commands listed below or record why they are deferred.

## Validation Matrix

| Area | Command | Status | Expected Result |
| --- | --- | --- | --- |
| Git state | `git status --short --branch` | `Completed` | Showed `## datax/phase2-1-architecture-baseline` with untracked `docs/plans/Phase-2/`, which is expected for the planning artifacts in this working tree. |
| Whitespace | `git diff --check` | `Completed` | No output. |
| Product spelling | `rg -n "Data[Xx]" docs/plans/Phase-2` | `Completed` | Matches were reviewed. The command also matches the preferred `Datax` spelling, so the result is informational rather than a failure. |
| Public concept leakage | `rg -n "\\b(Thread|Turn|Item)\\b" docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md` | `Completed` | Matches are intentional references to inherited or downstream concepts. |
| Codex references | `rg -n "\\b(Codex|codex|CODEX)\\b" docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md` | `Completed` | Matches are classified in the Codex Boundary Register or are command examples and revision authorship notes. |
| Runtime machinery blocker | `rg -n "datax_core|datax-core|CodexThread|ThreadManager|ThreadId|TurnItem|RolloutItem" datax-rs/app-server datax-rs/app-server-protocol datax-rs/core-api datax-rs/thread-store` | `Completed` | Matches confirm direct inherited runtime machinery remains in app-server-facing paths and must be remediated before Phase 2.2. |
| Formatting | `cd datax-rs && just fmt` | `Deferred` | Not required for planning-only Markdown change unless a future code edit is added. |
| Targeted tests | `cd datax-rs && just test -p codex-app-server-protocol` | `Deferred` | Not required for planning-only Markdown change; record user Phase 1 test results when available. |
| Fix/lint | `cd datax-rs && just fix -p codex-app-server-protocol` | `Deferred` | Not required for planning-only Markdown change. |

## Validation and Acceptance

Phase 2.1 is accepted when this file exists, is current, and includes the required file inventory, Codex Boundary Register, public-surface checklist, dependency order, validation matrix, rollback note, and stop-the-line remediation gate. It is also accepted when the phase branch exists and GitHub issue and draft pull request links are recorded, or when a clear blocker is recorded for those operational artifacts. Phase 2.2 is not accepted to start until the inherited runtime machinery blocker has a concrete remediation plan.

Run these commands from the repository root:

    git status --short --branch
    git diff --check
    rg -n "Data[Xx]" docs/plans/Phase-2
    rg -n "\\b(Thread|Turn|Item)\\b" docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md
    rg -n "\\b(Codex|codex|CODEX)\\b" docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md

Successful validation means `git diff --check` prints nothing, product spelling matches are intentional, and all `Thread`, `Turn`, `Item`, and `Codex` references in this artifact are either baseline debt, downstream runtime descriptions, compatibility notes, or protected sandbox exceptions.

Corrected validation also means the inherited runtime machinery matches are not waived. They are recorded evidence for the stop-the-line remediation gate.

## Idempotence and Recovery

This phase is safe to repeat. Re-reading files and re-running `rg` commands does not mutate the repository. If the architecture baseline needs to be backed out, remove only `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md` and leave runtime code unchanged. If a later validation result changes the baseline, update this plan in place rather than creating a competing Phase 2.1 document.

## Rollback or Recovery Note

Because Phase 2.1 is planning-only, rollback is a documentation rollback. Do not revert implementation files to recover from a Phase 2.1 mistake because no implementation file should be changed in this phase. If a GitHub issue or draft pull request is created with the wrong scope, update its title/body to point back to this file and keep the branch name stable unless the branch itself was created from the wrong base commit.

## Open Questions

- What were the final Phase 1 test results from the still-running validation mentioned by the user?
- Which files should make up the first reviewable remediation slice for removing direct `datax-rs/core` machinery from Datax app-server-facing Chat, Interaction, and Message paths?
- Should the temporary Datax primitive boundary live inside `datax-rs/app-server`, a new focused compatibility crate, or an existing non-core Datax crate?
- Should Phase 2.3 place `AgentAdapter` inside `datax-rs/app-server` first, or create a new `datax-rs/agent-adapter` crate immediately, after the remediation gate is satisfied?
- Should Phase 2.4 create a new `datax-rs/codex-runtime` crate, or isolate the first skeleton in an existing crate and move it later, after the remediation gate is satisfied?

## Artifacts and Notes

Branch:

    datax/phase2-1-architecture-baseline

Baseline commit:

    7754dfc1c9

Issue:

    https://github.com/mbellary/datax/issues/15

Draft pull request:

    https://github.com/mbellary/datax/pull/16

Phase 1 validation:

    Pending user update. The user reported tests are still running.

Revision note:

    2026-07-09 / Codex: Created the Phase 2.1 architecture baseline ExecPlan so the adapter-first architecture can be reviewed before runtime behavior changes begin.
    2026-07-09 / Codex: Updated the baseline after user correction. Direct inherited runtime machinery in Datax app-server-facing Chat, Interaction, and Message paths is now recorded as a Phase 1 migration gap and a blocker before Phase 2.2.
    2026-07-09 / Codex: Linked the companion remediation ExecPlan that defines implementation slices for replacing inherited app-server-facing runtime machinery with a Datax primitive boundary.

## Interfaces and Dependencies

No new code interfaces are introduced in Phase 2.1. The intended future interface is `AgentAdapter`, a Datax-owned contract that accepts Datax-shaped requests and emits Datax-shaped responses or events. Its public types should use names such as chat, interaction, message, artifact, approval, and runtime status. It must not expose downstream Codex `Thread`, `Turn`, or `Item` as public Datax API concepts.

The intended future dependency boundary is `codex-runtime`, which should translate `AgentAdapter` calls into downstream Codex app-server lifecycle and protocol calls. Datax app-server code should depend on the adapter contract, not directly on downstream Codex app-server lifecycle or protocol details.
