# Mechanically Migrate Codex Protocol Concepts to Datax Concepts

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This repository contains `PLANS.md` at the repository root. Maintain this document in accordance with `PLANS.md`: keep it self-contained, update it as discoveries occur, and make each milestone independently verifiable.

## Purpose / Big Picture

Datax is a migrated fork of Codex for data engineering. The migration goal is to keep the same lightweight terminal-agent capabilities while changing the product protocol vocabulary from Codex names to Datax names.

The controlling rule is mechanical and compositional:

    Codex  -> Datax
    Thread -> Chat
    Turn   -> Interaction
    Item   -> Message

After this plan is implemented, a developer should be able to inspect Datax-facing code and see Datax names for the same roles Codex used to provide. `ThreadManager` becomes `ChatManager`; `TurnItem` becomes `InteractionMessage`; `RolloutItem` becomes `RolloutMessage`. The capabilities remain. The product vocabulary changes. Later Phase 2 work can then add a downstream Codex bridge that maps Datax protocol back to Codex protocol in one explicit place.

## Progress

- [x] (2026-07-09 00:00Z) Captured the product requirement as a four-rule mechanical migration: `Codex -> Datax`, `Thread -> Chat`, `Turn -> Interaction`, and `Item -> Message`.
- [x] (2026-07-09 00:00Z) Clarified that inherited runtime capabilities are still required in Datax; they should be renamed and owned in Datax terms rather than removed.
- [x] (2026-07-09 00:00Z) Clarified composed names: `ThreadManager -> ChatManager`, `ThreadId -> ChatId`, `TurnItem -> InteractionMessage`, and `RolloutItem -> RolloutMessage`.
- [ ] Inventory all remaining Datax-facing Codex/Thread/Turn/Item names and classify them as mechanical rename targets, compatibility aliases, downstream Codex bridge terms, provenance, or unrelated English.
- [ ] Rename Datax-facing protocol and app-server internals according to the four-rule mapping without adding new product behavior.
- [ ] Rename persistence and history types according to the same mapping while preserving compatibility with existing stored records.
- [ ] Regenerate affected app-server schemas and run targeted tests.
- [ ] Update Phase 2 bridge plans so downstream Codex integration starts only after Datax owns the Datax-named protocol and runtime contracts.

## Surprises & Discoveries

- Observation: Some public protocol files already expose Datax wire names while Rust internals still use Codex-era field names.
  Evidence: `datax-rs/app-server-protocol/src/protocol/v2/chat.rs` has `ChatStartResponse` serialized as `chat` while the Rust field was named `thread`; `datax-rs/app-server-protocol/src/protocol/v2/interaction.rs` has `InteractionStartResponse` serialized as `interaction` while the Rust field was named `turn`.

- Observation: The app-server still uses Codex-named live runtime machinery in Datax request processors.
  Evidence: `datax-rs/app-server/src/request_processors/chat_processor.rs` uses `ThreadManager`, `CodexThread`, `ThreadId`, `RolloutItem`, and `TurnItem`; `datax-rs/app-server/src/request_processors/interaction_processor.rs` uses `ThreadManager`, `CodexThread`, `ThreadId`, and core `Op` values.

- Observation: Persistence still uses Thread/Turn/Item vocabulary as first-class Datax-facing store names.
  Evidence: `datax-rs/thread-store/src/types.rs` defines `CreateThreadParams`, `StoredThread`, `StoredTurn`, `StoredThreadHistory`, and `AppendThreadItemsParams`; `datax-rs/protocol/src/protocol.rs` defines `RolloutItem` and event fields such as `turn_id`.

## Decision Log

- Decision: Treat this plan as a mechanical migration plan, not an architecture redesign.
  Rationale: The user requirement is to preserve Codex-derived capabilities while changing the product protocol vocabulary. The problem is scattered old names, not the existence of manager, session, event, persistence, or history machinery.
  Date/Author: 2026-07-09 / Codex

- Decision: Apply the four-rule mapping compositionally to compound names.
  Rationale: A direct composed mapping avoids ambiguous replacements. For example, `TurnItem` maps to `InteractionMessage`, not to generic `Message`; `RolloutItem` maps to `RolloutMessage`, not to a new history-record abstraction.
  Date/Author: 2026-07-09 / Codex

- Decision: Keep compatibility and downstream Codex bridge terms explicit and isolated.
  Rationale: Existing stored data and future downstream Codex app-server integration may still require Codex protocol names. Those names are acceptable only when the code is clearly compatibility/provenance/bridge code, not Datax's native product model.
  Date/Author: 2026-07-09 / Codex

- Decision: Do not add data-engineering product features during this migration.
  Rationale: Product features should be built on stable Datax vocabulary after the mechanical migration is complete.
  Date/Author: 2026-07-09 / Codex

## Outcomes & Retrospective

This plan is newly created and has not yet been implemented. Its immediate outcome is alignment on the migration invariant: Datax should keep the Codex-derived behavior but rename Datax-facing protocol, runtime, persistence, and history concepts using the four-rule mapping.

The main remaining risk is migration breadth. Many Rust modules, generated schemas, tests, and persisted fixtures may depend on old names. The work should therefore proceed in narrow, testable slices: inventory first, then protocol/internal field names, then app-server runtime names, then persistence/history names, then bridge planning.

## Context and Orientation

This repository is a fork of the Codex codebase. Codex used protocol and runtime concepts named `Thread`, `Turn`, and `Item`. Datax is intended to use equivalent product concepts named `Chat`, `Interaction`, and `Message`.

The mapping is literal:

    CodexThread              -> DataxChat or Chat, depending on local naming clarity
    ThreadManager            -> ChatManager
    ThreadId                 -> ChatId
    NewThread                -> NewChat
    StartThreadOptions       -> StartChatOptions
    TurnItem                 -> InteractionMessage
    TurnContext              -> InteractionContext
    TurnStarted              -> InteractionStarted
    TurnComplete             -> InteractionComplete
    TurnAborted              -> InteractionAborted
    turn_id                  -> interaction_id
    RolloutItem              -> RolloutMessage
    StoredThread             -> StoredChat
    StoredTurn               -> StoredInteraction
    StoredTurnStatus         -> StoredInteractionStatus
    StoredThreadHistory      -> StoredChatHistory
    AppendThreadItemsParams  -> AppendChatMessagesParams
    ListTurnsParams          -> ListInteractionsParams
    ItemStarted              -> MessageStarted
    ItemCompleted            -> MessageCompleted

This is not a request to delete runtime machinery. Datax still needs a manager for active work, a live chat/session handle, durable identifiers, event streams, persistence, resumability, rollout/history records, and request processors. The requirement is that Datax-facing code names those concepts with Datax vocabulary.

`Datax-facing code` means code that defines or implements the Datax product protocol, Datax app-server behavior, Datax CLI/TUI behavior, Datax persistence interfaces, or Datax runtime contracts. This code should follow the four-rule mapping.

`Compatibility code` means code that reads old stored files, supports old wire aliases, or preserves migration continuity. It may mention old names, but the file/module/type should make that compatibility purpose clear.

`Downstream Codex bridge code` means future Phase 2 code that talks to an external Codex app-server. It may use Codex `Thread`/`Turn`/`Item` names because its job is to translate between Datax and Codex. That bridge is not part of the native Datax model.

`Provenance` means license history, upstream repository links, historical comments, or package names that must remain Codex-related for legal or external reasons.

Known important files:

`datax-rs/app-server-protocol/src/protocol/v2/chat.rs` defines v2 chat protocol types and currently contains some Rust internals still named after threads.

`datax-rs/app-server-protocol/src/protocol/v2/interaction.rs` defines v2 interaction protocol types and currently contains some Rust internals still named after turns.

`datax-rs/app-server/src/request_processors/chat_processor.rs` handles `chat/*` requests and currently uses `ThreadManager`, `CodexThread`, `ThreadId`, `RolloutItem`, and `TurnItem`.

`datax-rs/app-server/src/request_processors/interaction_processor.rs` handles `interaction/*` requests and currently uses `ThreadManager`, `CodexThread`, and `ThreadId`.

`datax-rs/app-server/src/thread_state.rs` tracks live state using inherited thread and turn names.

`datax-rs/thread-store/src/types.rs` defines persistence types currently named around threads and turns.

`datax-rs/protocol/src/thread_id.rs` defines `ThreadId`.

`datax-rs/protocol/src/protocol.rs` defines `RolloutItem` and event structures that include turn/item vocabulary.

`datax-rs/core-api/src/lib.rs` re-exports inherited thread-management APIs.

## Plan of Work

Milestone 1 is a boundary inventory. Search Datax-facing Rust and protocol files for `Codex`, `Thread`, `Turn`, `Item`, and common snake_case forms such as `thread_id` and `turn_id`. Classify each occurrence as a mechanical rename target, compatibility alias, downstream Codex bridge term, provenance, protected sandbox identifier, external dependency, or unrelated English. This milestone should update this plan with a concise inventory summary before code changes begin.

Milestone 2 renames the app-server protocol internals where the wire name is already Datax but the Rust field or type name remains Codex-era. For example, `ChatStartResponse` should have a Rust field named `chat`, and `InteractionStartResponse` should have a Rust field named `interaction`. Preserve intentional serde/TypeScript aliases such as `numTurns` only where compatibility is required, and document those aliases as compatibility.

Milestone 3 introduces or renames Datax primitive identifiers and event names. `ThreadId` becomes `ChatId` in Datax-facing code. `turn_id` becomes `interaction_id`. `EventMsg::TurnStarted`, `EventMsg::TurnComplete`, and `EventMsg::TurnAborted` become Datax-facing interaction event names. Any old event names needed for stored compatibility must be isolated behind conversion code.

Milestone 4 migrates live runtime names. The `ThreadManager` role becomes `ChatManager`. The `CodexThread` live handle role becomes a Datax chat/session handle, preferably `DataxChat` or `ChatSession` based on existing local naming clarity. Start/resume/interrupt/listener functions should be renamed according to the mapping while preserving behavior.

Milestone 5 migrates message and history names. `TurnItem` becomes `InteractionMessage`; `RolloutItem` becomes `RolloutMessage`. Stored types follow the same rule: `StoredThread` becomes `StoredChat`, `StoredTurn` becomes `StoredInteraction`, and `StoredThreadHistory` becomes `StoredChatHistory`. Existing stored data should remain readable through explicit compatibility conversions.

Milestone 6 updates app-server request processors and state tracking to depend on Datax-named contracts. `chat_processor.rs`, `interaction_processor.rs`, `thread_state.rs`, dynamic tool response handling, and goal/status tracking should speak in chat/interaction/message terms. The behavior should remain equivalent to the current app-server behavior.

Milestone 7 validates generated protocol and tests. Regenerate app-server schema fixtures if protocol shapes or TypeScript exports change. Run focused tests for every changed crate. If common, core, or protocol crates are changed broadly, ask before running the complete workspace test suite.

Milestone 8 updates Phase 2 bridge planning. Only after Datax-facing code owns Datax vocabulary should later plans introduce `AgentAdapter` and `codex-runtime` to map Datax `Chat`/`Interaction`/`Message` back to downstream Codex `Thread`/`Turn`/`Item`.

## Concrete Steps

Start each work session from the repository root:

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch

Run the initial inventory searches:

    rg -n "CodexThread|ThreadManager|ThreadId|NewThread|StartThreadOptions|TurnItem|RolloutItem|StoredThread|StoredTurn|StoredThreadHistory|AppendThreadItemsParams|ListTurnsParams|TurnStarted|TurnComplete|TurnAborted|ItemStarted|ItemCompleted|thread_id|turn_id" datax-rs -g '*.rs' -g 'Cargo.toml'

    rg -n "\\bCodex\\b|\\bThread\\b|\\bTurn\\b|\\bItem\\b|thread/|turn/|numTurns|parent_thread_id" datax-rs/app-server datax-rs/app-server-protocol datax-rs/thread-store datax-rs/protocol datax-rs/core-api -g '*.rs' -g '*.ts' -g '*.json' -g 'README.md'

Expected result at the current baseline: these searches find many matches. During Milestone 1, matches are evidence to classify, not failures.

After Rust edits, format from `datax-rs`:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt

For app-server protocol changes, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p codex-app-server-protocol

For app-server implementation changes, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p codex-app-server

For protocol schema changes, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just write-app-server-schema
    just test -p codex-app-server-protocol

Before finalizing a large app-server change, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fix -p codex-app-server

Do not run `cargo test` directly in `datax-rs`. Use `just test` according to repository instructions.

## Validation and Acceptance

The migration is accepted when Datax-facing code consistently follows the four-rule mapping and existing behavior still works.

Run:

    cd /home/mbellary/wsl/projects/datax
    rg -n "ThreadManager|CodexThread|ThreadId|TurnItem|RolloutItem|StoredThread|StoredTurn|StoredThreadHistory|EventMsg::Turn|thread_id|turn_id" datax-rs/app-server datax-rs/app-server-protocol/src/protocol/v2 datax-rs/thread-store datax-rs/core-api -g '*.rs'

Expected final result: remaining matches are only in explicitly named compatibility code, downstream Codex bridge code, provenance, or unrelated non-protocol usage. Datax request processors and native Datax protocol files should use `ChatManager`, `ChatId`, `InteractionMessage`, `RolloutMessage`, and interaction/message event names.

Run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p codex-app-server-protocol
    just test -p codex-app-server

Expected result: both commands pass. A Datax client should still be able to use `chat/*`, `interaction/*`, and `message/*` app-server methods, with Datax protocol payloads and events.

If persistence files are changed, add or update tests proving existing stored records remain readable. The acceptance behavior is: a stored record written before this migration can be loaded and projected as Datax `Chat`/`Interaction`/`RolloutMessage` history.

If schema fixtures are changed, inspect the generated diff and confirm that active v2 API names remain Datax-oriented. Compatibility aliases may remain, but they should be intentional and documented.

## Idempotence and Recovery

Inventory searches and validation commands are safe to run repeatedly. Formatting with `just fmt` is safe after Rust edits.

The migration should be staged additively where possible. Prefer introducing Datax-named aliases or wrapper conversions first, then migrating call sites, then removing old exposed names after tests pass. Do not delete broad inherited machinery in one sweep.

If a rename breaks tests, identify whether the failure is caused by Rust type names, serde wire names, generated TypeScript exports, stored fixture compatibility, or runtime event conversion. Fix the narrow cause. Do not revert unrelated worktree changes.

## Artifacts and Notes

Current branch when this plan was created:

    datax/phase2-1-architecture-baseline

This plan supersedes earlier wording that implied Datax should remove inherited runtime machinery. The corrected statement is: Datax keeps the machinery and mechanically migrates its Datax-facing names and contracts.

The future bridge flow remains:

    Datax Chat / Interaction / Message
        -> AgentAdapter
        -> codex-runtime
        -> downstream Codex Thread / Turn / Item

That bridge is a later phase. This plan prepares for it by making Datax's side of the mapping clean.

## Interfaces and Dependencies

The final Datax-facing names should follow these roles:

    pub struct ChatId(...);

`ChatId` is the Datax durable identifier for a chat. It is the mechanical replacement for Datax-facing `ThreadId`.

    pub struct InteractionId(...);

`InteractionId` identifies one exchange inside a chat. It is the mechanical replacement for Datax-facing `turn_id` and any old turn identifier type.

    pub struct ChatManager { ... }

`ChatManager` owns the role previously held by `ThreadManager`: creating, loading, resuming, interrupting, and tracking chats and interactions.

    pub enum InteractionMessage { ... }

`InteractionMessage` owns the role previously held by `TurnItem`: messages or events that belong to a single interaction.

    pub enum RolloutMessage { ... }

`RolloutMessage` owns the role previously held by `RolloutItem`: durable history messages used to replay or project chat history.

    pub struct StoredChat { ... }
    pub struct StoredInteraction { ... }
    pub struct StoredChatHistory { ... }

These own the roles previously held by `StoredThread`, `StoredTurn`, and `StoredThreadHistory`.

No new public Datax-facing interface should expose `Thread`, `Turn`, or `Item` unless it is clearly marked as compatibility or downstream Codex bridge code.

## Revision Notes

- 2026-07-09 / Codex: Created this ExecPlan to capture the corrected migration direction as a mechanical protocol/domain rename governed by four rules.
