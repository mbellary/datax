# Provisional Datax Migration Plan: Phase 2 Adapter-First Product Architecture

## Summary

Phase 2 starts from the Phase 1 fork-first migration baseline. Datax remains a
forked and migrated codebase, not a greenfield rewrite. The goal of Phase 2 is
to introduce the architecture that lets Datax become a data-engineering product
while still using a downstream Codex app-server for agentic work.

Phase 1.8 mechanically migrated Datax-facing protocol and runtime vocabulary
according to the mapping `Codex -> Datax`, `Thread -> Chat`, `Turn ->
Interaction`, and `Item -> Message`. Phase 2 therefore starts from Datax-native
contracts such as `ChatManager`, `DataxChat`, `ChatId`,
`InteractionMessage`, `RolloutMessage`, `ChatState`, and `ChatStore`. The
downstream Codex `Thread` / `Turn` / `Item` vocabulary belongs only in future
adapter/runtime bridge code.

Terminology in this document is intentionally explicit:

- `Datax app-server` means the app-server in this repository that serves Datax
  clients and owns the Datax public protocol.
- `downstream Codex app-server` means the external or separately managed Codex
  runtime app-server that Datax may call for agentic work.
- `AgentAdapter` means the Datax-owned boundary between the Datax app-server
  and any downstream agent runtime.
- `codex-runtime` means the implementation boundary that can translate
  `AgentAdapter` calls into downstream Codex app-server calls.

The verified starting workflow after Phase 1 is:

- Before Phase 2: Datax TUI/CLI talks to the inherited Datax app-server. The
  Datax app-server public protocol and TUI request construction already use
  `Chat`, `Interaction`, and `Message` concepts, including `chat/*`,
  `interaction/*`, and `message/*` request and notification methods. After
  Phase 1.8, native Datax-facing runtime, message/history, app-server state,
  and store contracts also use Datax names.
- After Phase 2: Datax TUI/CLI talks to the Datax app-server, and the Datax
  app-server talks to `AgentAdapter` when agentic work is required.
  `AgentAdapter` delegates to `codex-runtime` when the selected runtime is the
  downstream Codex app-server.

The target request flow for agent-backed work is:

1. Datax TUI sends a Datax public request such as `chat/start` to the Datax
   app-server.
2. Datax app-server validates the Datax public request and owns Datax-side
   state and projections.
3. Datax app-server calls `AgentAdapter` using Datax-owned types. It does this
   only when the selected Datax behavior requires downstream agent runtime work.
4. `AgentAdapter` selects or resolves the runtime implementation. For downstream
   Codex work, it selects `codex-runtime`.
5. `codex-runtime` owns downstream Codex app-server lifecycle and maps Datax
   `chat/start` semantics to downstream Codex `thread/start` semantics.
6. `codex-runtime` initializes or uses the downstream Codex app-server as
   needed.
7. The downstream Codex app-server processes the request and returns responses
   or emits events using its own runtime protocol.
8. `codex-runtime` maps downstream Codex `thread`, `turn`, and `item`
   responses or events back into Datax `chat`, `interaction`, and `message`
   responses or events.
9. Responses and events flow back through `AgentAdapter` to Datax app-server.
10. Datax app-server persists or projects Datax state as needed and returns
    responses or sends notifications to Datax TUI.

The path is:

    Datax TUI -> Datax app-server -> AgentAdapter -> codex-runtime -> downstream Codex app-server -> codex-runtime -> AgentAdapter -> Datax app-server -> Datax TUI

The product direction is captured in `docs/plans/Phase-2/platform.md`. The
execution model for this phase is captured in
`docs/plans/Phase-2/Recommended-Datax-Phase2-Execution-Model.md`.

## Phase 2 Strategy

Phase 2 should be adapter-first and product-boundary-first. Do not start by
building data-engineering features directly into downstream Codex protocol
names. If any retained `thread`, `turn`, or `item` terms are encountered, first
classify them as compatibility, provenance, downstream runtime bridge,
protected sandbox identifiers, external dependencies, unrelated English, or a
separately owned migration leftover. Then create the boundaries that keep Datax
product concepts separate from downstream Codex runtime concepts.

Datax owns:

- Datax app-server protocol for Datax clients.
- Datax product state.
- Data-engineering concepts such as plans, workflows, deployments, schedules,
  runs, monitors, approvals, and artifacts.
- The `AgentAdapter` contract used to request agentic work.
- Runtime-link records that connect Datax entities to downstream Codex runtime
  identifiers when needed.

Downstream Codex owns:

- Agentic coding/runtime behavior.
- Tool execution, model interaction, command execution, and file changes inside
  the downstream runtime.
- Its own downstream `Thread`, `Turn`, and `Item` model.

Datax public APIs must not expose Codex `Thread`, `Turn`, or `Item` concepts.
When the adapter must translate between models, the translation belongs in the
adapter boundary, not in the public Datax protocol.

## Naming and Boundary Policy

Use `Datax` as the product spelling in Phase 2 artifacts.

Allowed retained Codex references in Phase 2 are limited to:

- Downstream Codex runtime integration.
- Compatibility shims explicitly marked as compatibility work.
- Upstream provenance and license history.
- External package, repository, or URL names that really are Codex-owned.
- Protected sandbox identifiers.

Preferred future boundary names:

- `AgentAdapter` for the Datax-owned abstraction that requests agentic work.
- `codex-runtime` for downstream Codex app-server process/client/protocol
  integration.
- `codex-compat` for legacy compatibility shims if compatibility work becomes
  necessary.

Do not create a new `codex-core` boundary. That name conflicts with the
inherited crate meaning and makes Codex appear to be the center of Datax.

Do not restructure existing Datax-owned folders merely for cleanup in Phase 2.
Only create or move code when needed to isolate `AgentAdapter`,
`codex-runtime`, compatibility shims, or Datax product state.

## Implementation Phases

### Phase 2.1: Product Architecture Baseline

Purpose: record the target Phase 2 architecture in a PLANS.md-compliant
ExecPlan and establish the first Phase 2 branch, issue, and draft pull request.

Scope:

- Confirm Phase 1 test results and current baseline commit.
- Confirm the Phase 1.8 mechanical migration baseline and do not treat
  pre-Phase-1.8 names such as `ThreadManager`, `CodexThread`, `ThreadId`,
  `TurnItem`, or `RolloutItem` as the current Datax app-server substrate.
- Record the Datax app-server as the public boundary in both directions:
  upstream-facing for Datax clients such as TUI/CLI, and downstream-facing only
  to the Datax-owned `AgentAdapter` contract.
- Record the downstream Codex app-server as an implementation detail behind
  `AgentAdapter` and `codex-runtime`.
- Define which current modules are likely Datax-owned versus downstream
  Codex-boundary candidates.
- Do not implement runtime behavior in this phase.

Required artifact:

- `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md`

Exit criteria:

- Architecture ExecPlan exists and follows PLANS.md.
- File inventory identifies Datax app-server, Datax app-server protocol,
  runtime, persistence, CLI/TUI, `AgentAdapter`, and downstream Codex app-server
  candidate files.
- No implementation code is changed unless needed to keep planning references
  accurate.

### Phase 2.2: Downstream Codex Boundary Inventory

Purpose: identify all files and concepts that currently represent downstream
Codex runtime, compatibility, upstream provenance, or accidental retained
identity.

Scope:

- Search remaining Codex references using focused `rg` commands.
- Classify Codex references into downstream runtime, compatibility,
  provenance, protected sandbox, external dependency, unrelated, or rename
  candidate.
- Pay special attention to Datax app-server process launch points, JSON-RPC
  transport, Datax app-server protocol client/server code, schema generation
  outputs, SDK/package surfaces, release artifact URLs, and runtime
  process-management scripts.
- Do not rename everything mechanically.

Required artifact:

- `docs/plans/Phase-2/datax_phase2_2_codex_boundary_inventory/codex_boundary_inventory_execplan.md`

Exit criteria:

- A Codex Boundary Register exists.
- Each retained Codex reference touched by Phase 2 planning has a classification.
- Candidate `codex-runtime` and `codex-compat` files are identified before any
  implementation move or adapter work.

### Phase 2.3: Agent Adapter Contract

Purpose: define a Datax-owned trait or interface for requesting agentic work
without exposing downstream Codex types to Datax public APIs.

Scope:

- Add or identify the smallest appropriate crate/module for the adapter
  contract.
- Define Datax request and event types that use Datax names such as chat,
  interaction, message, artifact, approval, and runtime status.
- Define error and timeout semantics.
- Do not launch a downstream Codex process in this phase.
- Do not add data-engineering product workflow behavior yet.

Potential implementation locations for `AgentAdapter`, subject to the phase file
inventory:

- `datax-rs/app-server`
- `datax-rs/app-server-protocol`
- a new focused crate such as `datax-rs/agent-adapter`

Required artifact:

- `docs/plans/Phase-2/datax_phase2_3_agent_adapter_contract/agent_adapter_contract_execplan.md`

Exit criteria:

- Datax-owned adapter contract exists.
- No public adapter type exposes Codex `Thread`, `Turn`, or `Item`.
- Targeted tests or compile checks are documented for user execution.

### Phase 2.4: Codex Runtime Adapter Skeleton

Purpose: create the `codex-runtime` boundary that owns downstream Codex
app-server lifecycle and protocol integration, without wiring broad runtime
behavior into the Datax app-server.

Scope:

- Create or identify a dedicated boundary for downstream Codex runtime code.
- Make `codex-runtime` the owner of downstream Codex app-server lifecycle:
  locating or launching the process, performing initialization, tracking
  status, stopping or restarting when requested, reporting degraded or
  unavailable states, and ensuring Datax app-server code never manages the
  downstream Codex app-server process directly.
- Implement a minimal adapter skeleton that can represent configured,
  unavailable, starting, ready, degraded, and stopped states.
- Define configuration for locating the downstream Codex app-server, but avoid
  making the downstream runtime mandatory for basic Datax startup.
- Keep direct Codex protocol names inside the boundary.

Potential implementation location:

- `datax-rs/codex-runtime`

Required artifact:

- `docs/plans/Phase-2/datax_phase2_4_codex_runtime_skeleton/codex_runtime_skeleton_execplan.md`

Exit criteria:

- Downstream Codex runtime boundary exists or is explicitly deferred with a
  documented replacement location.
- Downstream Codex app-server lifecycle ownership is documented and isolated in
  `codex-runtime`.
- The skeleton can be constructed and queried for status.
- No direct downstream Codex process dependency is required by default Datax
  startup.
- Datax app-server code does not directly start, stop, restart, or monitor the
  downstream Codex app-server process.

### Phase 2.5: Datax App-Server Mediation

Purpose: route one narrow Datax app-server path through `AgentAdapter` so the
Datax app-server becomes the owner of mediation without directly depending on
the downstream Codex app-server.

Scope:

- Pick one minimal Datax app-server flow, such as runtime status read or a
  dry-run `AgentAdapter` handshake.
- Wire Datax app-server to the `AgentAdapter` contract.
- Return Datax-shaped responses and notifications.
- Let Datax app-server request runtime status or work through `AgentAdapter`;
  lifecycle actions must remain delegated to `codex-runtime`.
- Do not expose downstream Codex protocol details to clients.

Required artifact:

- `docs/plans/Phase-2/datax_phase2_5_app_server_mediation/app_server_mediation_execplan.md`

Exit criteria:

- Datax app-server owns the mediation point.
- Datax app-server uses `AgentAdapter` and does not directly manage downstream
  Codex app-server lifecycle.
- The selected flow is observable through a Datax command, test client, or
  Datax app-server test.
- Runtime unavailable behavior is explicit and non-crashing.

### Phase 2.6: Datax Persistence and Runtime-Link Model

Purpose: introduce the minimal persistence model that separates Datax product
state from downstream Codex runtime identifiers.

Scope:

- Define records for Datax-owned entities and runtime links.
- Store downstream Codex identifiers only as runtime-link fields, not as the
  primary Datax model.
- Include migration strategy and schema generation commands if the existing
  persistence system requires them.
- Keep the model narrow enough to support the first adapter-mediated smoke
  flow.

Potential concepts:

- `Chat`
- `Interaction`
- `Message`
- `RuntimeLink`
- `RuntimeStatus`

Required artifact:

- `docs/plans/Phase-2/datax_phase2_6_persistence_runtime_links/persistence_runtime_links_execplan.md`

Exit criteria:

- Datax persistence can represent a Datax entity independently from a downstream
  Codex id.
- Runtime-link records can associate Datax ids with downstream Codex ids when
  present.
- Persistence tests or schema checks are documented.

### Phase 2.7: First Data-Engineering Domain Skeleton

Purpose: add the first thin Datax product-domain skeleton without building full
product workflows.

Scope:

- Introduce a minimal domain shape for data-engineering work such as a plan,
  workflow, run, monitor, artifact, or approval.
- Keep the domain model independent from Codex `Thread`, `Turn`, and `Item`.
- Connect only as far as needed to prove that Datax product state can coexist
  with adapter-mediated agentic work.

Required artifact:

- `docs/plans/Phase-2/datax_phase2_7_domain_skeleton/domain_skeleton_execplan.md`

Exit criteria:

- At least one Datax product-domain concept exists in a narrow, testable form.
- The domain concept is not modeled as a wrapper around Codex history.
- Follow-up Phase 3 product workflows are identified but not implemented here.

### Phase 2.8: End-to-End Adapter Smoke and Freeze Checkpoint

Purpose: prove the Phase 2 architecture works end to end and record remaining
work before Phase 3 product-feature implementation.

Scope:

- Run static boundary checks.
- Run schema, build, formatting, fix, and targeted test commands as recorded in
  the phase ExecPlans.
- Run an end-to-end smoke that starts Datax app-server and exercises the narrow
  adapter-mediated behavior.
- Verify downstream Codex app-server lifecycle behavior through `codex-runtime`
  status or smoke checks, including unavailable or degraded state reporting.
- Record known limitations and Phase 3 candidates.

Required artifact:

- `docs/plans/Phase-2/datax_phase2_8_adapter_smoke_freeze/adapter_smoke_freeze_execplan.md`

Exit criteria:

- Datax app-server can exercise at least one adapter-mediated path.
- Downstream Codex unavailable behavior is verified.
- Downstream Codex app-server lifecycle ownership is verified to remain inside
  `codex-runtime`.
- Remaining Codex references are classified.
- Phase 3 can begin from a documented Datax product architecture baseline.

## Initial Validation Commands

These are planning-stage commands. Each phase ExecPlan must replace or extend
them with exact commands for its specific changed files.

From repository root:

    git status --short --branch
    git diff --check
    rg -n "Data[Xx]" docs/plans/Phase-2
    rg -n "\\b(Thread|Turn|Item)\\b" docs/plans/Phase-2
    rg -n "\\b(Codex|codex|CODEX)\\b" docs/plans/Phase-2

Expected result:

- `git diff --check` has no output.
- The forbidden mixed-case product spelling has no matches.
- `Thread`, `Turn`, and `Item` matches appear only when describing downstream
  Codex concepts or forbidden public API leakage.
- `Codex` matches are intentional downstream runtime, compatibility, or
  provenance references.
- Focused searches for pre-Phase-1.8 native runtime names such as
  `ThreadManager`, `CodexThread`, `ThreadId`, `TurnItem`, and `RolloutItem`
  either have no native app-server-facing matches or return explicitly
  classified compatibility/provenance/downstream-runtime leftovers.

## Phase 2 Risks

The main Phase 2 risk is accidentally rebuilding Datax as a thin rename over
Codex history. The mitigation is to make the adapter boundary and runtime-link
model explicit before adding data-engineering product workflows.

The second risk is hiding too much behavior behind planning language. The
mitigation is to require each implementation phase to produce a narrow,
observable behavior and exact validation commands.

The third risk is over-restructuring inherited code too early. The mitigation is
to keep Datax-owned folder structure as-is unless a move is required to isolate
downstream Codex runtime or compatibility code.

## Phase 2 Completion Standard

Phase 2 is complete when Datax has a documented, testable, and implementation
backed boundary between Datax product state and downstream Codex runtime work.
At that point Phase 3 can focus on data-engineering features rather than
identity cleanup or adapter architecture.
