# Phase 2.1 Runtime Machinery Remediation

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document follows `PLANS.md` from the repository root. It is self-contained so a contributor can restart the Phase 2.1 remediation work from this file alone.

## Purpose / Big Picture

Phase 1 was expected to migrate the app-server-facing runtime model to Datax primitive `Chat`, `Interaction`, and `Message` concepts. The Phase 2.1 architecture baseline discovered that direct inherited runtime machinery still exists in the Datax app-server path, especially `datax-rs/core`, `datax-rs/core-api`, `ThreadManager`, `CodexThread`, `ThreadId`, `TurnItem`, and `RolloutItem`.

This remediation plan works out that gap before Phase 2.2 begins. After this remediation, Datax app-server-facing code should consume Datax primitives and runtime-link metadata, while any inherited thread, turn, rollout, or item concepts are either removed from the path or contained behind an explicit Datax compatibility boundary.

Update after Phase 1.8: the mechanical naming portion of this remediation was completed by the Phase 1.8 milestones. The Datax-facing substrate now uses `ChatManager`, `DataxChat`, `ChatId`, `InteractionMessage`, `RolloutMessage`, `ChatState`, and `ChatStore`. This document remains useful as historical context and as a reminder that future Phase 2 bridge work must isolate downstream Codex `Thread` / `Turn` / `Item` translation behind `AgentAdapter` and `codex-runtime`.

## Baseline

The starting branch is `datax/phase2-1-architecture-baseline`. The starting Phase 2.1 architecture artifact is `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md`. The correction being applied is that inherited runtime machinery is not an acceptable Phase 2 substrate. It is a Phase 1 migration gap and must be remediated before downstream boundary inventory, adapter contracts, or runtime skeleton work continues.

The current app-server public protocol already uses `chat/*`, `interaction/*`, and `message/*` methods. The implementation still imports and passes inherited runtime types. The first implementation goal is not to delete all downstream compatibility in one patch. The goal is to make the app-server-facing boundary Datax-owned, explicit, and testable.

Current status after Phase 1.8: the native app-server-facing names were remediated mechanically. Remaining Phase 2 work should not implement these old slices as written; it should instead inventory any retained Codex references, classify them, and introduce `AgentAdapter` / `codex-runtime` only at the downstream bridge boundary.

## Progress

- [x] (2026-07-09 00:00Z) Recorded the user correction in the Phase 2.1 architecture baseline.
- [x] (2026-07-09 00:00Z) Confirmed `datax-rs/app-server/Cargo.toml` still depends directly on `datax-core`.
- [x] (2026-07-09 00:00Z) Confirmed `datax-rs/core-api/src/lib.rs` still re-exports inherited thread-management APIs.
- [x] (2026-07-09 00:00Z) Confirmed focused `rg` matches for `datax_core`, `ThreadManager`, `CodexThread`, `ThreadId`, `TurnItem`, and `RolloutItem` across app-server, app-server-protocol, core-api, and thread-store.
- [x] (2026-07-10 00:00Z) Phase 1.8 mechanically renamed the app-server-facing substrate to Datax names, including `ChatManager`, `DataxChat`, `ChatId`, `InteractionMessage`, `RolloutMessage`, `ChatState`, and `ChatStore`.
- [x] (2026-07-10 00:00Z) Reclassified the old remediation slices as superseded by Phase 1.8 for native Datax naming; remaining Phase 2 bridge work must use a fresh downstream boundary inventory.
- [ ] Record user-provided validation results for the Phase 1.8 build, format, fix, schema, and targeted test commands.

## Surprises & Discoveries

- Observation: The app-server crate has a direct `datax-core` dependency.
  Evidence: `datax-rs/app-server/Cargo.toml` contains `datax-core = { workspace = true }`.
- Observation: The app-server-facing runtime surface still uses inherited types at multiple layers.
  Evidence: `datax-rs/app-server/src/models.rs`, `datax-rs/app-server/src/message_processor.rs`, `datax-rs/app-server/src/request_processors/chat_processor.rs`, `datax-rs/app-server/src/request_processors/interaction_processor.rs`, `datax-rs/app-server/src/request_processors/thread_lifecycle.rs`, `datax-rs/app-server/src/thread_state.rs`, and `datax-rs/app-server/src/dynamic_tools.rs` contain direct `datax_core`, `ThreadManager`, `CodexThread`, or `ThreadId` references.
- Observation: The protocol layer already contains useful Datax message types, but conversion still depends on inherited event and item types.
  Evidence: `datax-rs/app-server-protocol/src/protocol/v2/message.rs` implements conversion from `datax_protocol::items::TurnItem`; `datax-rs/app-server-protocol/src/protocol/thread_history.rs` builds `Interaction` and `Message` values from `RolloutItem`.
- Observation: Generated schema fixtures and tests still contain thread and item naming.
  Evidence: focused search matches schema files under `datax-rs/app-server-protocol/schema/` and tests under `datax-rs/app-server/tests/suite/v2/`.

## Decision Log

- Decision: Do not proceed to Phase 2.2 until this remediation plan has at least one implementation slice that establishes a Datax-owned primitive boundary.
  Rationale: Inventorying a downstream Codex boundary before the Datax app-server boundary is corrected would preserve the wrong center of gravity.
  Date/Author: 2026-07-09 / Codex
- Decision: Treat `datax-rs/core` as a dependency to be removed from app-server-facing Chat, Interaction, and Message paths, not as a renamed Datax runtime foundation.
  Rationale: Phase 1 migration intent was to move to Datax primitives; retaining inherited thread-management APIs as the substrate contradicts that intent.
  Date/Author: 2026-07-09 / Codex
- Decision: Use a transitional Datax compatibility boundary where behavior cannot be removed in one reviewable slice.
  Rationale: The current references span request processing, event mapping, persistence, tests, and generated schemas. A staged translation boundary reduces risk while keeping the end state explicit.
  Date/Author: 2026-07-09 / Codex
- Decision: Supersede the old implementation slices with the Phase 1.8 mechanical migration result.
  Rationale: The slices targeted old Datax-facing names that are no longer the current substrate. Re-running them would create confusing duplicate work. Phase 2 should now focus on bridge isolation and runtime-link semantics from the Datax-named baseline.
  Date/Author: 2026-07-10 / Codex

## Outcomes & Retrospective

This remediation plan is now historical guidance. The expected old outcome, removing direct inherited names from app-server-facing surfaces, was handled by Phase 1.8's mechanical migration. The remaining active outcome is to ensure future Phase 2 code maps Datax `Chat` / `Interaction` / `Message` to downstream Codex `Thread` / `Turn` / `Item` only inside `AgentAdapter` and `codex-runtime`.

## Context and Orientation

The public Datax protocol is mostly in `datax-rs/app-server-protocol/src/protocol/common.rs` and `datax-rs/app-server-protocol/src/protocol/v2/`. Public methods already use `chat`, `interaction`, and `message` naming.

The current app-server execution path starts in `datax-rs/app-server/src/message_processor.rs`, which constructs or receives a `ThreadManager`. Request processors then use `ThreadManager` and `CodexThread` to create, resume, steer, interrupt, unload, and inspect sessions. Per-chat active state lives in `datax-rs/app-server/src/thread_state.rs`, status projection in `datax-rs/app-server/src/thread_status.rs`, and lifecycle helpers in `datax-rs/app-server/src/request_processors/thread_lifecycle.rs`.

The protocol conversion path maps inherited runtime items into Datax messages. Important files are `datax-rs/app-server-protocol/src/protocol/v2/message.rs`, `datax-rs/app-server-protocol/src/protocol/thread_history.rs`, and `datax-rs/app-server-protocol/src/protocol/event_mapping.rs`.

Persistence still exposes thread-shaped concepts through `datax-rs/thread-store`. This may remain internally transitional, but the Datax app-server boundary must distinguish Datax ids from downstream runtime ids.

## File Inventory

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/runtime_machinery_remediation_execplan.md` | `Completed` | This remediation plan. |
| `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md` | `Completed` | Updated to mark inherited runtime machinery as a blocker before Phase 2.2. |
| `datax-rs/app-server/Cargo.toml` | `Planned` | Direct `datax-core` dependency must shrink or move behind a compatibility boundary. |
| `datax-rs/app-server/src/models.rs` | `Planned` | Currently stores `Arc<ThreadManager>` as app-server state; primary remediation entry point. |
| `datax-rs/app-server/src/message_processor.rs` | `Planned` | Constructs app-server request processors with inherited runtime manager; should depend on Datax primitive boundary. |
| `datax-rs/app-server/src/request_processors/chat_processor.rs` | `Planned` | Chat start/resume/list/read path still uses inherited runtime machinery. |
| `datax-rs/app-server/src/request_processors/interaction_processor.rs` | `Planned` | Interaction start/steer/interrupt path still uses inherited runtime machinery. |
| `datax-rs/app-server/src/request_processors/thread_lifecycle.rs` | `Planned` | Lifecycle helper should become compatibility translation or be renamed/replaced by Datax lifecycle semantics. |
| `datax-rs/app-server/src/thread_state.rs` | `Planned` | Active runtime handle storage should not expose `CodexThread` as the Datax app-server-facing model. |
| `datax-rs/app-server/src/thread_status.rs` | `Planned` | Status projection should consume Datax runtime facts rather than thread watch types. |
| `datax-rs/app-server/src/dynamic_tools.rs` | `Planned` | Dynamic tool output currently targets `CodexThread`; move behind Datax runtime session handle. |
| `datax-rs/app-server-protocol/src/protocol/v2/message.rs` | `Planned` | Inherited `TurnItem` conversion should be isolated behind explicit translation. |
| `datax-rs/app-server-protocol/src/protocol/thread_history.rs` | `Planned` | Inherited `RolloutItem` conversion should be isolated behind explicit translation. |
| `datax-rs/app-server-protocol/src/protocol/event_mapping.rs` | `Planned` | Event mapping should be named as compatibility translation if inherited events remain. |
| `datax-rs/thread-store/src/lib.rs` | `Planned` | Persistence boundary should distinguish Datax product ids from downstream runtime ids. |
| `datax-rs/core-api/src/lib.rs` | `Planned` | Do not treat this facade as the Datax primitive runtime API. Reduce or stop app-server-facing dependency on inherited re-exports. |

## Remediation Architecture

Historical proposal: introduce a Datax-owned runtime boundary before `AgentAdapter`. Phase 1.8 supplied the native Datax naming layer mechanically. Do not implement the old proposal literally unless a current source scan finds a new native Datax-facing Codex leak.

Suggested first module shape:

- `datax-rs/app-server/src/runtime.rs` owns Datax runtime-facing traits and structs.
- `DataxRuntime` is the app-server-facing interface.
- `ChatRuntimeId`, `InteractionRuntimeId`, and `RuntimeLink` make Datax ids and downstream runtime ids explicit.
- `ActiveChatRuntime` or similar owns operations that request processors need: start chat, resume chat, start interaction, steer interaction, interrupt interaction, inject messages, list active chats, read runtime history, and subscribe to status.
- `legacy_core_runtime.rs` or `datax_compat_runtime.rs` contains the temporary implementation that calls current `datax_core` types.

This keeps request processors Datax-shaped immediately while leaving the deeper runtime replacement staged.

## Implementation Slices

The slices below are superseded for native Datax naming by Phase 1.8. They remain as context for why bridge code must be isolated.

Superseded slice 1: Add the Datax primitive runtime boundary.

Create app-server-local runtime types and move constructor surfaces away from raw `Arc<ThreadManager>`. The compatibility implementation may still wrap `ThreadManager`, but request processors should receive the Datax runtime abstraction. This slice should be small enough to compile without changing public JSON-RPC schemas.

Superseded slice 2: Move event and history conversion behind explicit compatibility translation.

Rename or extract the conversion code that maps `TurnItem`, `RolloutItem`, and `EventMsg::Turn*` into Datax messages. The Datax protocol module should expose Datax `Message` and `Interaction` results; inherited item names should be contained in one clearly named translation area.

Superseded slice 3: Replace active state and status surfaces.

Move `CodexThread` ownership out of `thread_state.rs`, `thread_status.rs`, and dynamic tool handling surfaces. These modules should receive Datax runtime facts and handles instead of inherited runtime objects.

Superseded slice 4: Split persistence identity.

Introduce explicit runtime-link metadata for persisted records. Datax `chat_id` and `interaction_id` must be distinct from downstream runtime identifiers even if a transitional one-to-one mapping is used.

Superseded slice 5: Remove or narrow direct app-server dependency on `datax-core`.

After request processors and state modules consume the Datax runtime boundary, remove `datax-core` from app-server-facing code or keep it only inside the explicit compatibility implementation. The final dependency state should be visible in `datax-rs/app-server/Cargo.toml`.

## Concrete Steps

From the repository root, inspect the current direct references:

    rg -n "datax_core|datax-core|CodexThread|ThreadManager|ThreadId|TurnItem|RolloutItem" datax-rs/app-server datax-rs/app-server-protocol datax-rs/core-api datax-rs/thread-store

Inspect the primary app-server construction path:

    sed -n '1,220p' datax-rs/app-server/src/models.rs
    sed -n '1,260p' datax-rs/app-server/src/message_processor.rs
    sed -n '1,260p' datax-rs/app-server/src/request_processors/chat_processor.rs
    sed -n '1,260p' datax-rs/app-server/src/request_processors/interaction_processor.rs
    sed -n '1,260p' datax-rs/app-server/src/request_processors/thread_lifecycle.rs

Do not implement these superseded slices as the next milestone. Start Phase 2 bridge work with the downstream Codex boundary inventory from the current Datax-named baseline. Keep public protocol schemas unchanged unless a future implementation phase intentionally changes them.

After code changes in `datax-rs`, run:

    cd datax-rs
    just fmt
    just test -p datax-app-server
    just test -p datax-app-server-protocol
    just fix -p datax-app-server

If protocol types or generated schemas change, also run:

    cd datax-rs
    just write-app-server-schema
    just test -p datax-app-server-protocol

## Validation Matrix

| Area | Command | Status | Expected Result |
| --- | --- | --- | --- |
| Reference inventory | `rg -n "datax_core|datax-core|CodexThread|ThreadManager|ThreadId|TurnItem|RolloutItem" datax-rs/app-server datax-rs/app-server-protocol datax-rs/core-api datax-rs/thread-store` | `Historical` | Confirmed the original remediation scope before Phase 1.8. Repeat the current focused search from the mechanical migration plan before starting Phase 2.2. |
| Markdown whitespace | `git diff --check` | `Pending` | No output. |
| Formatting | `cd datax-rs && just fmt` | `Deferred` | Not run for the Milestone 8 planning update per user instruction. Required only after future Rust code changes. |
| App-server tests | `cd datax-rs && just test -p datax-app-server` | `Deferred` | Not run for the Milestone 8 planning update per user instruction. Required after future app-server runtime boundary changes. |
| Protocol tests | `cd datax-rs && just test -p datax-app-server-protocol` | `Deferred` | Not run for the Milestone 8 planning update per user instruction. Required if future protocol conversion or schema-related code changes. |
| App-server fix/lint | `cd datax-rs && just fix -p datax-app-server` | `Deferred` | Not run for the Milestone 8 planning update per user instruction. Required before finalizing future app-server Rust changes. |

## Validation and Acceptance

This remediation is accepted for native naming when current source scans show request processors no longer receive or own direct `ThreadManager` and `CodexThread` app-server-facing surfaces, old `TurnItem` / `RolloutItem` names are not native Datax message/history contracts, and remaining inherited references are documented as compatibility, provenance, downstream runtime bridge terms, protected exceptions, external dependencies, unrelated English, or owning-slice leftovers.

Phase 2.2 may start from the Phase 1.8 Datax-named baseline. It must not start by adding downstream Codex calls to the Datax app-server; it should first inventory and classify the downstream Codex boundary.

## Idempotence and Recovery

Each slice should be independently reviewable. If a slice fails, revert only that slice and keep the plan updated with the failure reason. Do not remove public Datax protocol methods as a shortcut. Do not rename generated schema files mechanically unless the protocol shape intentionally changes and schema generation is run.

## Rollback or Recovery Note

Rollback for this planning-only file is to remove `docs/plans/Phase-2/datax_phase2_1_architecture_baseline/runtime_machinery_remediation_execplan.md` and update the architecture baseline to explain why remediation was deferred. Rollback for future code slices must preserve user data and persisted chat history.

## Open Questions

- Should the first implementation slice use an app-server-local `runtime` module, or should it create a new crate immediately?
- Which persisted fields are Datax product identifiers versus downstream runtime identifiers today?
- Which app-server tests should become the canonical coverage for Chat, Interaction, and Message primitive behavior after the boundary is introduced?
- Can `datax-rs/core-api` be retired from app-server-facing usage, or does it need a temporary compatibility facade with Datax naming?

## Artifacts and Notes

Branch:

    datax/phase2-1-architecture-baseline

Related architecture baseline:

    docs/plans/Phase-2/datax_phase2_1_architecture_baseline/architecture_baseline_execplan.md

Revision note:

    2026-07-09 / Codex: Created remediation plan after user clarified that inherited runtime machinery should not have survived as the Datax app-server-facing substrate.

## Interfaces and Dependencies

The desired new local interface is a Datax primitive runtime boundary consumed by app-server request processors. It should use Datax concepts: chat, interaction, message, runtime link, status, approval, and artifact. It must not expose downstream Codex `Thread`, `Turn`, or `Item` as app-server-facing concepts.

The temporary implementation may call inherited runtime machinery while slices are in progress, but those calls must be contained in a named compatibility module and must not define the public or app-server-facing Datax model.
