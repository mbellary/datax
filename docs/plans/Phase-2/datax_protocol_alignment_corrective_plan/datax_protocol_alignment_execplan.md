# Correct Datax Protocol Alignment Before Phase 2 Runtime Adapter Work

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This repository contains `PLANS.md` at the repository root. Maintain this document in accordance with `PLANS.md`: keep it self-contained, update it as discoveries occur, and make each milestone independently verifiable.

## Purpose / Big Picture

Datax is intended to be a lightweight terminal coding agent for data engineering. It was created by migrating the Codex codebase into a Datax codebase while changing the product protocol vocabulary from Codex `Thread`, `Turn`, and `Item` to Datax `Chat`, `Interaction`, and `Message`.

The intended architecture is not a greenfield rewrite and not merely a wrapper around Codex. Datax should own its public app-server protocol and its product model in Datax terms. Downstream Codex integration should happen later through one explicit translation boundary: Datax `Chat`/`Interaction`/`Message` maps to downstream Codex `Thread`/`Turn`/`Item` only inside `AgentAdapter` and `codex-runtime`.

After this corrective plan is implemented, a developer can inspect the Datax app-server, app-server protocol, persistence boundary, and runtime boundary and see one clean rule: Datax-facing code speaks Datax terms, while Codex terms appear only in explicit downstream runtime compatibility code or historical/provenance locations. This makes Phase 2 adapter work simpler because it becomes one clear translation problem rather than scattered mixed terminology across the product.

## Progress

- [x] (2026-07-09 00:00Z) Captured the corrected product intent: Datax owns `Chat`/`Interaction`/`Message`; downstream Codex owns `Thread`/`Turn`/`Item`.
- [x] (2026-07-09 00:00Z) Identified current architectural mismatch: the public app-server protocol has moved toward Datax terms, but runtime and persistence still use Codex-named contracts such as `ThreadManager`, `CodexThread`, `ThreadId`, `RolloutItem`, `TurnItem`, and `EventMsg::Turn*` in Datax-facing code.
- [x] (2026-07-09 00:00Z) Added this corrective ExecPlan as the gate before further Phase 2 runtime adapter implementation.
- [x] (2026-07-09 00:00Z) Clarified that Datax still needs equivalent runtime machinery, such as a chat manager and durable history. The correction is to migrate Datax-facing contracts from Codex vocabulary to Datax vocabulary, not to remove the machinery.
- [ ] Inventory every Datax-facing app-server, protocol, persistence, CLI, and TUI file that still exposes inherited Codex runtime machinery.
- [ ] Define Datax primitive types and boundaries: `ChatId`, `InteractionId`, `Message`, `ChatRuntime`, `ChatStore`, and Datax event/history records.
- [ ] Quarantine or migrate inherited runtime concepts so they no longer appear as app-server-facing product concepts.
- [ ] Establish the future `AgentAdapter` and `codex-runtime` seam as the only allowed Datax-to-Codex translation location.
- [ ] Validate with focused `rg` checks, targeted Rust tests, schema generation checks if API shapes change, and app-server protocol tests.

## Surprises & Discoveries

- Observation: The current Datax public protocol is partially migrated, but Rust implementation names still reveal inherited terminology.
  Evidence: `datax-rs/app-server-protocol/src/protocol/v2/chat.rs` serializes `ChatStartResponse` with a `chat` field while the Rust field is still named `thread`; `datax-rs/app-server-protocol/src/protocol/v2/interaction.rs` serializes `InteractionStartResponse` with an `interaction` field while the Rust field is still named `turn`.

- Observation: Datax app-server currently starts and manages live work through inherited core runtime objects.
  Evidence: `datax-rs/app-server/src/request_processors/chat_processor.rs` stores `Arc<ThreadManager>` and calls `start_thread_with_options`; `datax-rs/app-server/src/request_processors/interaction_processor.rs` loads `Arc<CodexThread>` and submits core `Op` values to it.

- Observation: Persistence still uses Thread/Turn/Rollout vocabulary as a first-class store model.
  Evidence: `datax-rs/thread-store/src/types.rs` defines `CreateThreadParams`, `StoredThread`, `StoredTurn`, and `StoredThreadHistory`; `datax-rs/protocol/src/protocol.rs` defines `RolloutItem`, `SessionMeta`, and event fields with `turn_id`.

- Observation: A facade named `core-api` still presents inherited thread-management APIs as public API.
  Evidence: `datax-rs/core-api/src/lib.rs` describes itself as a public facade for thread management APIs and re-exports `CodexThread`, `ThreadManager`, and `ThreadId`.

## Decision Log

- Decision: Treat this work as a corrective Phase 1 closure gate before Phase 2 adapter/runtime implementation.
  Rationale: Phase 2 assumes Datax has a clean Datax-owned product protocol. If Datax-facing app-server code still owns Codex-era runtime machinery, `AgentAdapter` and `codex-runtime` will not be a single clean translation boundary.
  Date/Author: 2026-07-09 / Codex

- Decision: Do not build data-engineering product features or downstream Codex runtime process management until the Datax product boundary is clean or explicitly quarantined.
  Rationale: Product concepts such as workflows, schedules, runs, artifacts, approvals, lineage, and deployments should attach to Datax `Chat`/`Interaction`/`Message` and future Datax entities, not to inherited thread machinery.
  Date/Author: 2026-07-09 / Codex

- Decision: Preserve inherited capabilities while migrating Datax-facing names and contracts to Datax concepts.
  Rationale: Datax still needs managers, live chat handles, durable ids, events, and history records. The problem is not the existence of this machinery; the problem is exposing Codex-named contracts as Datax's native product model.
  Date/Author: 2026-07-09 / Codex

- Decision: Allow temporary compatibility code only when it is named and documented as compatibility, not as a new `codex-core` foundation.
  Rationale: Compatibility may be needed to keep the fork working while migration proceeds, but it must not become the architectural center of Datax.
  Date/Author: 2026-07-09 / Codex

## Outcomes & Retrospective

This plan starts as an architecture correction, not an implementation result. The main outcome so far is alignment on the desired direction: Datax should be a migrated product that speaks Datax protocol internally and externally, while downstream Codex protocol translation belongs behind a future adapter boundary.

The largest remaining gap is mechanical and architectural: many app-server, protocol, persistence, and runtime files still expose inherited Codex concepts. The plan below decomposes that gap into reviewable milestones so the migration can proceed without breaking all agent capabilities at once.

## Context and Orientation

This repository is a fork/migration of the Codex codebase. Codex is a lightweight coding agent that uses protocol concepts named `Thread`, `Turn`, and `Item`. Datax is intended to be a lightweight terminal coding agent for data engineering that uses product protocol concepts named `Chat`, `Interaction`, and `Message`.

The desired final model is:

    Datax CLI/TUI/Web
      speaks Chat / Interaction / Message
            |
    Datax app-server
      owns Datax protocol, state, projections, and product behavior
            |
    AgentAdapter
      Datax-owned trait or interface for requesting agent work
            |
    codex-runtime
      translates Datax Chat/Interaction/Message to downstream Codex Thread/Turn/Item
            |
    downstream Codex app-server
      external runtime that speaks Codex protocol

Terms used in this plan:

`Datax-facing code` means code that defines or implements Datax product behavior, Datax public protocol, Datax app-server request handling, Datax persistence, Datax CLI/TUI behavior, or Datax product state. This code should use `Chat`, `Interaction`, and `Message`.

`Downstream Codex runtime` means an external or separately managed Codex app-server that may still speak `Thread`, `Turn`, and `Item`. This runtime is not the Datax product model.

`AgentAdapter` means the Datax-owned boundary used by Datax app-server to request agentic work without exposing downstream Codex types.

`codex-runtime` means the implementation behind `AgentAdapter` that knows how to launch, connect to, and translate requests/events for a downstream Codex app-server.

`Compatibility code` means temporary code that still understands old Thread/Turn/Item records or APIs so the repository can continue working during migration. Compatibility code must be isolated, named as compatibility, and should shrink over time.

Datax still requires the same kinds of machinery Codex had. A `ThreadManager`-like component is still needed, but the Datax-facing contract should be `ChatManager` or another Datax-named equivalent. A `CodexThread`-like live handle is still needed, but the Datax-facing contract should be `DataxChat`, `ChatSession`, or another Datax-named equivalent. A `ThreadId`-like durable identifier is still needed, but the Datax-facing contract should be `ChatId`. A `RolloutItem`-like persisted history record is still needed, but the Datax-facing contract should be a Datax history/message record.

Current source evidence for the mismatch:

`datax-rs/app-server/src/request_processors/chat_processor.rs` stores `Arc<ThreadManager>` in `ChatRequestProcessor`, loads `Arc<CodexThread>`, and starts live work through `start_thread_with_options`. This means Datax `chat/start` is still implemented directly by inherited thread runtime machinery.

`datax-rs/app-server/src/request_processors/interaction_processor.rs` stores `Arc<ThreadManager>`, loads `Arc<CodexThread>`, and submits core `Op` values. This means Datax `interaction/start` and `interaction/steer` are not yet routed through a Datax-owned interaction runtime boundary.

`datax-rs/app-server/src/thread_state.rs` tracks listener state with `Weak<CodexThread>`, `RolloutItem`, and `EventMsg::Turn*`. This means running Datax chat state still depends on inherited live thread objects and turn events.

`datax-rs/app-server-protocol/src/protocol/thread_history.rs` converts persisted `RolloutItem` and `EventMsg::Turn*` values into `Interaction` values. This is useful compatibility behavior, but it should eventually live behind a Datax history projection boundary or downstream runtime translator.

`datax-rs/thread-store/src/types.rs` defines `CreateThreadParams`, `StoredThread`, `StoredTurn`, `StoredThreadHistory`, and `AppendThreadItemsParams` using `ThreadId` and `RolloutItem`. This is persistence debt that prevents Datax from having a clean Chat/Interaction/Message store model.

`datax-rs/protocol/src/thread_id.rs` defines `ThreadId` as the durable identifier. Datax should expose and own a Datax-named equivalent such as `ChatId`; downstream Codex `ThreadId` should appear only when translating to or from downstream Codex protocol.

`datax-rs/core-api/src/lib.rs` re-exports `CodexThread`, `ThreadManager`, and `ThreadId`, and describes itself as a facade for thread management. This should not be the center of Datax app-server architecture.

## Plan of Work

Milestone 1 establishes a complete boundary inventory. Search the repository for inherited Codex runtime concepts in Datax-facing paths, and classify each occurrence. Use categories: `Datax product surface`, `Datax implementation debt`, `temporary compatibility`, `downstream Codex runtime`, `upstream provenance`, `external dependency`, and `protected sandbox identifier`. The result should be a register in this plan or a companion document that names files, symbols, and the intended action. This milestone changes documentation only.

Milestone 2 defines the Datax primitive model. Add or identify Datax-owned primitive types such as `ChatId`, `InteractionId`, `Message`, `ChatEvent`, and `InteractionEvent`. If the existing `datax-rs/protocol` crate remains the right home, add these as Datax primitives without deleting old compatibility types in the same change. If a smaller crate is better, create a focused crate and keep its API small. The acceptance point is that new Datax-facing code can use Datax-named primitives for the same roles currently served by `ThreadId`, `TurnItem`, and `RolloutItem`.

Milestone 3 creates an app-server runtime boundary. Introduce or rename the inherited runtime manager into a Datax-owned contract, tentatively `ChatManager`, that gives app-server request processors a Datax interface for starting a chat, starting or steering an interaction, interrupting an interaction, subscribing to chat events, reading live history, and shutting down a chat. The Datax-facing contract should use Datax names such as `ChatId`, `InteractionId`, and Datax history/message records. Its first implementation may delegate to current migrated machinery, but any Codex-named compatibility surface must be isolated and named as compatibility.

Milestone 4 migrates app-server request processors to depend on the Datax runtime boundary. Change `ChatRequestProcessor`, `InteractionRequestProcessor`, listener lifecycle code, dynamic tool response handling, status tracking, and goal processing so their public dependencies are Datax-named contracts such as `ChatManager`, `ChatSession`, `ChatId`, and Datax event/history records. Any old Codex-named interaction must happen inside compatibility implementation code. This milestone preserves current behavior while moving the boundary.

Milestone 5 creates a Datax persistence boundary. Introduce `ChatStore`-style types or rename/migrate the existing `ThreadStore` API so Datax-facing code reads and writes `Chat`, `Interaction`, and `Message` records. If existing rollout files must remain readable, implement a compatibility importer/projector that maps old `RolloutItem` records into Datax history. New Datax product features should write Datax-named history records unless explicitly operating in compatibility mode.

Milestone 6 cleans the app-server protocol internals. Keep wire compatibility where needed, but align Rust field/type names with Datax terms. For example, `ChatStartResponse` should own a field named `chat`, not a Rust field named `thread` that is serialized as `chat`; `InteractionStartResponse` should own `interaction`, not a Rust field named `turn` serialized as `interaction`. When changing public protocol shapes or generated TypeScript schemas, regenerate app-server schema fixtures and run protocol tests.

Milestone 7 establishes the future adapter seam. Define `AgentAdapter` using Datax-owned request, response, event, and error types. Do not implement downstream Codex process management yet. The purpose is to prove that Datax app-server can ask for agentic work without importing downstream Codex protocol concepts.

Milestone 8 introduces `codex-runtime` as the only downstream Codex translation implementation. This milestone belongs after the cleanup above. It should connect `AgentAdapter` to a downstream Codex app-server and own all mapping between Datax `Chat`/`Interaction`/`Message` and downstream Codex `Thread`/`Turn`/`Item`. No other Datax-facing code should perform that mapping.

## Concrete Steps

Start every work session from the repository root:

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch

For Milestone 1, run focused inventory searches. These commands are read-only and can be repeated safely:

    rg -n "CodexThread|ThreadManager|ThreadId|TurnItem|RolloutItem|EventMsg::Turn|EventMsg::RawResponseItem|Op::UserTurn|Op::DynamicToolResponse|thread_id|turn_id" datax-rs/app-server datax-rs/app-server-protocol datax-rs/thread-store datax-rs/protocol datax-rs/core-api -g '*.rs' -g 'Cargo.toml'

    rg -n "thread/start|turn/start|rawResponseItem|numTurns|conversation_id|parent_thread_id|StoredThread|StoredTurn|CreateThreadParams|ResumeThreadParams" datax-rs/app-server datax-rs/app-server-protocol datax-rs/thread-store datax-rs/protocol -g '*.rs' -g '*.ts' -g '*.json'

Expected result for the current baseline: these searches should find many matches. That is not a test failure during Milestone 1; it is evidence for the boundary register.

After each implementation milestone that changes Rust code, run formatting from `datax-rs`:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt

Run targeted tests for changed crates. For app-server protocol changes:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p datax-app-server-protocol

For app-server implementation changes:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p datax-app-server

If app-server protocol schema shapes change, regenerate schema fixtures:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just write-app-server-schema
    just test -p datax-app-server-protocol

Before finalizing a large app-server Rust change, run the scoped fixer:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fix -p datax-app-server

Do not run `cargo test` directly in `datax-rs`. Use `just test` according to repository instructions. If changes touch common, core, or protocol crates broadly, ask before running the complete workspace test suite with `just test`.

## Validation and Acceptance

The corrective migration is accepted when the following observable checks are true.

First, Datax public app-server protocol still exposes Chat/Interaction/Message methods and payloads. Run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p datax-app-server-protocol

Expect protocol tests to pass. If schema fixtures were regenerated, review the generated changes and confirm new public v2 names remain Datax-oriented.

Second, Datax-facing app-server request processors no longer directly own inherited runtime machinery. Run:

    cd /home/mbellary/wsl/projects/datax
    rg -n "ThreadManager|CodexThread|TurnItem|RolloutItem|EventMsg::Turn|Op::UserTurn" datax-rs/app-server/src/request_processors datax-rs/app-server/src/thread_state.rs datax-rs/app-server/src/dynamic_tools.rs

Expected final result: Datax-facing request processors and state files use Datax-named contracts such as `ChatManager`, `ChatSession`, `ChatId`, `InteractionId`, and Datax history/message records. Matches for Codex-named contracts should exist only in explicitly named compatibility or downstream runtime adapter files.

Third, Datax app-server can still start a chat and interaction through the existing test suite:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p datax-app-server

Expect existing chat and interaction tests to pass. The behavior should remain that a Datax client can call `chat/start`, then `interaction/start`, and receive Datax `Chat`, `Interaction`, and `Message` responses or notifications.

Fourth, the future downstream Codex seam is clean. Run:

    cd /home/mbellary/wsl/projects/datax
    rg -n "Thread|Turn|Item|thread_id|turn_id" datax-rs/app-server/src datax-rs/app-server-protocol/src/protocol/v2 datax-rs/thread-store/src

Expected final result: remaining matches are either ordinary English unrelated to protocol concepts, explicitly documented compatibility code, or downstream runtime adapter code. There should be no accidental scattered mapping between Datax and Codex protocol concepts.

## Idempotence and Recovery

The inventory and validation commands are safe to run repeatedly. Formatting with `just fmt` is safe after Rust edits. Schema generation should only be run when protocol shapes change; if it produces unexpected changes, inspect the diff before committing.

Do not delete inherited runtime code in a single broad change. Prefer additive boundaries first, then migrate call sites, then remove old exposed paths once tests pass. This keeps the app-server usable throughout the correction.

If a milestone breaks app-server tests, first identify whether the failure is caused by API naming, runtime event mapping, persistence replay, or schema generation. Revert only the local change that caused the failure; do not reset unrelated user changes in the worktree.

## Artifacts and Notes

Current branch when this plan was created:

    datax/phase2-1-architecture-baseline

Current important related files:

    docs/plans/Phase-2/Provisional-Datax-Migration-Plan-Phase2.md
    docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md
    docs/plans/Phase-2/datax_phase2_1_architecture_baseline/runtime_machinery_remediation_execplan.md

This plan supersedes the assumption that Phase 2 can proceed directly to downstream adapter work. The corrected sequence is:

    1. Finish Datax protocol alignment and quarantine inherited machinery.
    2. Define Datax-owned AgentAdapter.
    3. Implement codex-runtime as the only Datax-to-downstream-Codex translator.
    4. Build data-engineering product features on Datax primitives.

## Interfaces and Dependencies

At the end of this corrective migration, Datax-facing code should depend on Datax-owned contracts shaped like the following. Exact module and crate placement should be decided during implementation based on existing crate boundaries and file sizes.

In a Datax-owned protocol or runtime boundary module, define a product identifier:

    pub struct ChatId(...);

`ChatId` is the durable Datax product identifier for a chat. It may serialize as a string UUID, but it should not be named `ThreadId`. A downstream Codex thread id, if needed, belongs in a runtime link record, not in the Datax product id type.

Define an interaction identifier:

    pub struct InteractionId(...);

`InteractionId` identifies one user-agent exchange within a chat. It should replace Datax-facing uses of `turn_id`.

Define a Datax-facing manager or service contract:

    pub struct ChatManager { ... }

    impl ChatManager {
        pub async fn start_chat(&self, request: StartChatRequest) -> Result<StartChatResult, ChatManagerError>;
        pub async fn start_interaction(&self, request: StartInteractionRequest) -> Result<StartInteractionResult, ChatManagerError>;
        pub async fn steer_interaction(&self, request: SteerInteractionRequest) -> Result<SteerInteractionResult, ChatManagerError>;
        pub async fn interrupt_interaction(&self, request: InterruptInteractionRequest) -> Result<(), ChatManagerError>;
    }

This can be a concrete struct, a trait, or another local pattern that fits the codebase. The important constraint is naming and ownership: Datax-facing request/result/error types should use Datax concepts. Codex-named types belong only inside compatibility or downstream runtime adapter code.

Define a runtime link only at the downstream adapter boundary:

    pub struct RuntimeLink {
        pub chat_id: ChatId,
        pub runtime_kind: RuntimeKind,
        pub runtime_chat_id: String,
    }

For the downstream Codex implementation, `runtime_chat_id` can hold the downstream Codex thread id. Datax product code should not interpret that value directly.

Define `AgentAdapter` after the Datax runtime boundary is stable:

    pub trait AgentAdapter {
        fn start_agent_work(&self, request: AgentWorkRequest)
            -> impl std::future::Future<Output = Result<AgentWorkStarted, AgentAdapterError>> + Send;
    }

`AgentAdapter` is the Phase 2 product boundary. `codex-runtime` is only one implementation of it.

## Revision Notes

2026-07-09 / Codex: Created this corrective ExecPlan after clarifying the product vision. The reason for this plan is that Datax should not build Phase 2 adapter and data-engineering product work on mixed Datax/Codex runtime vocabulary. The plan makes Datax protocol alignment the explicit gate before downstream Codex runtime integration.

2026-07-09 / Codex: Tightened the language after clarifying that Datax still needs equivalent runtime machinery. The correction is to migrate Datax-facing contracts to `Chat`/`Interaction`/`Message` names, not to remove managers, live handles, durable ids, or history records.
