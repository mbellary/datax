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
- [x] (2026-07-09 00:00Z) Completed Milestone 1 on branch `codex/phase1-8-m1-boundary-inventory`; GitHub issue #17 and draft PR #18 were created.
- [x] (2026-07-09 00:00Z) Inventory all remaining Datax-facing Codex/Thread/Turn/Item names and classify them as mechanical rename targets, compatibility aliases, downstream Codex bridge terms, provenance, protected sandbox identifiers, external dependencies, or unrelated English.
- [x] (2026-07-09 00:00Z) Started Milestone 2 on branch `codex/phase1-8-m2-protocol-internals`; GitHub issue #19 tracks the protocol-internals slice.
- [x] (2026-07-09 00:00Z) Renamed v2 app-server protocol Rust fields whose wire names were already Datax-oriented: `thread -> chat`, `turn -> interaction`, `expected_turn_id -> expected_interaction_id`, `initial_turns_page -> initial_interactions_page`, and `review_thread_id -> review_chat_id`.
- [x] Rename Datax-facing protocol and app-server internals according to the four-rule mapping without adding new product behavior.
- [x] (2026-07-09 00:00Z) Started Milestone 3 on branch `codex/phase1-8-m3-protocol-primitives`; GitHub issue #21 tracks the protocol primitive and event rename slice.
- [x] (2026-07-09 00:00Z) Renamed the exported protocol primitive `ThreadId -> ChatId` and the protocol source module `thread_id.rs -> chat_id.rs`.
- [x] (2026-07-09 00:00Z) Renamed Datax-facing protocol event identifiers from turn vocabulary to interaction vocabulary: `turn_id -> interaction_id`, `TurnStarted -> InteractionStarted`, `TurnComplete -> InteractionComplete`, and `TurnAborted -> InteractionAborted`.
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

- Observation: The Milestone 1 focused boundary scan found a large but classifiable rename surface across `datax-rs/app-server`, `datax-rs/app-server-protocol`, `datax-rs/thread-store`, `datax-rs/protocol`, and `datax-rs/core-api`.
  Evidence: the scan found 1172 `thread_id`, 742 `ThreadId`, 446 `turn_id`, 200 `RolloutItem`, 117 `parent_thread_id`, 110 `TurnItem`, 96 `StoredThread`, 89 `TurnStarted`, 59 `TurnComplete`, 59 `ThreadManager`, 49 `CodexThread`, 36 `ItemCompleted`, 29 `TurnAborted`, 27 `AppendThreadItemsParams`, 23 `ItemStarted`, 15 `StoredTurn`, 13 `NewThread`, 7 `StartThreadOptions`, 6 `ListTurnsParams`, and 1 `numTurns` match in those boundary crates and generated/schema artifacts.

- Observation: `gh auth status` may report an invalid stored token even when issue and PR creation are still usable in this environment.
  Evidence: Milestone 1 issue #17, PR #18, and Milestone 2 issue #19 were created successfully despite the earlier `gh auth status` warning.

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

- Decision: Implement this ExecPlan one milestone per GitHub branch, issue, and pull request.
  Rationale: The rename is mechanical but broad. One milestone per branch/issue/PR keeps review scope clear, lets long-running tests be owned by the user, and prevents later runtime or persistence changes from being mixed into the inventory/protocol slices.
  Date/Author: 2026-07-09 / Codex

- Decision: Treat `docs/plans/datax_mechanical_protocol_migration/mechanical_protocol_migration_execplan.md` as the canonical plan path.
  Rationale: The plan was moved out of the Phase 2 folder because this migration is now Phase 1.8 work. Any shorthand reference to `docs/plans/mechanical_protocol_migration_execplan.md` should resolve to this file unless a future repository move updates the path.
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

## Milestone Operating Model

Each milestone is implemented as a separate GitHub unit:

    branch: codex/phase1-8-mN-short-name
    issue: Phase 1.8 Milestone N: short name
    pull request: Phase 1.8 Milestone N: short name

Before starting a milestone, create or switch to the milestone branch from a clean `main`, create the GitHub issue, and link the issue in the pull request body. The branch must contain only that milestone's intended files. Commit and push the branch after local edits. Open the PR as a draft unless the user asks for ready-for-review.

The user will run the long build, format, and test commands. For each milestone, update this plan with the exact commands the user should run, the assumptions behind the milestone, and any compatibility exceptions discovered. If a command is not applicable because the milestone is documentation-only or inventory-only, say so explicitly in the milestone notes.

When GitHub CLI authentication is unavailable, continue local milestone work only if the scope is still clear. Record the blocker in this plan, then create the issue, push, and open the PR after authentication is restored.

## Milestone 1 Inventory Summary

Milestone 1 classified the current rename surface without changing Rust behavior.

Mechanical rename targets:

- App-server live runtime and state: `ThreadManager`, `CodexThread`, `NewThread`, `StartThreadOptions`, `ThreadId`, `thread_id`, `parent_thread_id`, and related helper names in `datax-rs/app-server/src/request_processors/chat_processor.rs`, `datax-rs/app-server/src/request_processors/interaction_processor.rs`, `datax-rs/app-server/src/thread_state.rs`, `datax-rs/app-server/src/thread_status.rs`, and adjacent request processors.
- App-server protocol internals: Rust fields named `thread` in Datax chat responses, Rust fields named `turn` in Datax interaction responses, `expected_turn_id`, and generated schema/type names such as `ThreadId.ts`.
- Core protocol primitives and events: `datax-rs/protocol/src/thread_id.rs`, `EventMsg::TurnStarted`, `EventMsg::TurnComplete`, `EventMsg::TurnAborted`, `EventMsg::ItemStarted`, `EventMsg::ItemCompleted`, event structs using `thread_id` and `turn_id`, `TurnItem`, and `RolloutItem`.
- Persistence and history: `StoredThread`, `StoredTurn`, `StoredThreadHistory`, `StoredTurnStatus`, `StoredTurnItemsView`, `AppendThreadItemsParams`, `ListTurnsParams`, local store modules under `datax-rs/thread-store/src/local`, and the thread-store README.
- Core re-export boundary: `datax-rs/core-api/src/lib.rs` re-exports `CodexThread`, `NewThread`, `StartThreadOptions`, `ThreadManager`, and `ThreadId`.

Compatibility aliases:

- `numTurns` in `datax-rs/app-server-protocol/src/protocol/v2/chat.rs` is already an explicit serde alias for `numInteractions`; keep it documented as compatibility while active.
- Legacy rollout compatibility in `datax-rs/protocol/src/protocol.rs` may need old event or field names behind explicit conversion/deserialization code so existing stored records remain readable.
- Existing generated schema files reflect current source names and should change only in the same milestone as the source protocol rename that generates them.

Downstream Codex bridge terms:

- Future Phase 2 names such as `AgentAdapter`, `codex-runtime`, and downstream Codex `Thread`/`Turn`/`Item` are allowed only in bridge planning or bridge implementation. No such bridge should be introduced during Phase 1.8.

Provenance and external dependency terms:

- Documentation or comments referring to upstream Codex services, Codex-managed auth, ChatGPT/Codex backend behavior, or upstream compatibility should be reviewed case by case. If the term describes Datax's product behavior, rename it. If it describes an external upstream dependency or historical source, classify it explicitly.
- `toml_edit::Item` in `datax-rs/app-server/src/config_manager_service.rs` and generic Rust iterator `Item` associated types are unrelated English/API terms and are not protocol migration targets.

Protected sandbox identifiers:

- Do not modify `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`, `CODEX_SANDBOX_ENV_VAR`, or code/comments whose only purpose is the protected sandbox environment contract.

Milestone 1 assumptions:

- This milestone is inventory and planning only; it does not rename code, regenerate schema, or alter runtime behavior.
- The canonical mapping remains `Codex -> Datax`, `Thread -> Chat`, `Turn -> Interaction`, and `Item -> Message`, applied compositionally.
- Datax still needs active chat management, live chat/session handles, persistence, history, event streams, resumability, and request processors.
- The branch for this milestone is `codex/phase1-8-m1-boundary-inventory`.

Milestone 1 user-run commands:

    cd /home/mbellary/wsl/projects/datax
    git diff --check

No `just fmt` or Rust tests are required for Milestone 1 because it is documentation-only. The inventory commands in `Concrete Steps` may be rerun to confirm the counts and classification.

## Milestone 2 Protocol Internals Summary

Milestone 2 renames v2 app-server protocol Rust fields that already serialize as Datax protocol concepts. The active wire contract is preserved by keeping existing `serde` and `ts-rs` renames.

Mechanical protocol-internal mappings completed:

- `ChatStartResponse.thread -> ChatStartResponse.chat`
- `ChatResumeResponse.thread -> ChatResumeResponse.chat`
- `ChatResumeParams.initial_turns_page -> ChatResumeParams.initial_interactions_page`
- `ChatResumeResponse.initial_turns_page -> ChatResumeResponse.initial_interactions_page`
- `ChatForkResponse.thread -> ChatForkResponse.chat`
- `ChatMetadataUpdateResponse.thread -> ChatMetadataUpdateResponse.chat`
- `ChatUnarchiveResponse.thread -> ChatUnarchiveResponse.chat`
- `ChatRollbackResponse.thread -> ChatRollbackResponse.chat`
- `ChatSearchResult.thread -> ChatSearchResult.chat`
- `ChatReadResponse.thread -> ChatReadResponse.chat`
- `ChatStartedNotification.thread -> ChatStartedNotification.chat`
- `InteractionStartResponse.turn -> InteractionStartResponse.interaction`
- `InteractionStartedNotification.turn -> InteractionStartedNotification.interaction`
- `InteractionCompletedNotification.turn -> InteractionCompletedNotification.interaction`
- `InteractionSteerParams.expected_turn_id -> InteractionSteerParams.expected_interaction_id`
- `ReviewStartResponse.turn -> ReviewStartResponse.interaction`
- `ReviewStartResponse.review_thread_id -> ReviewStartResponse.review_chat_id`

Compatibility retained:

- The wire names for all renamed fields remain Datax names such as `chat`, `interaction`, `expectedInteractionId`, `initialInteractionsPage`, and `reviewChatId`.
- `ChatRollbackParams.num_interactions` still accepts the legacy `numTurns` alias as an explicit compatibility alias.

Intentionally deferred to later milestones:

- Local runtime variables, helper functions, and state structs still using `thread`, `turn`, or `initial_turns_page` remain for Milestones 4 and 6.
- Event and history primitives such as `TurnStarted`, `TurnComplete`, `TurnItem`, and `RolloutItem` remain for Milestones 3 and 5.

Milestone 2 assumptions:

- This milestone changes Rust protocol and app-server call-site names only; it does not change JSON-RPC method names, JSON payload field names, generated TypeScript names, persistence format, or runtime behavior.
- Local variables named `thread` or `turn` may remain when they refer to runtime machinery that has not yet been mechanically migrated in this slice.
- The branch for this milestone is `codex/phase1-8-m2-protocol-internals`; the tracking issue is #19.

Milestone 2 user-run commands:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just write-app-server-schema
    just test -p datax-app-server-protocol
    just test -p datax-app-server

Milestone 2 command assumptions:

- `just fmt` is required because Rust code and tests changed.
- `just write-app-server-schema` is required because v2 protocol Rust field names and TypeScript exports changed while preserving the existing wire names.
- `just test -p datax-app-server-protocol` validates protocol serialization, schema fixture, and TypeScript export behavior.
- `just test -p datax-app-server` validates the app-server request processors, notifications, and v2 integration tests updated for the new Rust field names.
- No complete workspace `just test` is required for this milestone unless the focused commands expose broader protocol/core breakage.

## Milestone 3 Protocol Primitives Summary

Milestone 3 renames the exported Datax protocol primitive and interaction lifecycle events while preserving old serialized names through explicit compatibility aliases.

Mechanical primitive and event mappings completed:

- `ThreadId -> ChatId`
- `thread_id -> chat_id` for Datax-facing protocol fields and call sites touched by the exported primitive rename.
- `turn_id -> interaction_id` for Datax-facing protocol fields and call sites touched by the interaction event rename.
- `EventMsg::TurnStarted -> EventMsg::InteractionStarted`
- `TurnStartedEvent -> InteractionStartedEvent`
- `EventMsg::TurnComplete -> EventMsg::InteractionComplete`
- `TurnCompleteEvent -> InteractionCompleteEvent`
- `EventMsg::TurnAborted -> EventMsg::InteractionAborted`
- `TurnAbortedEvent -> InteractionAbortedEvent`
- `TurnAbortReason -> InteractionAbortReason`

Compatibility retained:

- `InteractionStarted`, `InteractionComplete`, and `InteractionAborted` accept old `task_*` and/or `turn_*` event tags where those tags were previously used by v1 rollout/event compatibility.
- Renamed persisted fields such as `chat_id`, `parent_chat_id`, and `interaction_id` accept old `thread_id`, `parent_thread_id`, and `turn_id` through explicit serde aliases in protocol structs.
- Generated schema and TypeScript artifacts are intentionally not hand-edited in this milestone; the user-run schema command should regenerate them from source.
- SQL migration files, generated hook schemas, generated protobuf files, and historical protocol v1 documentation may still contain old storage or compatibility names until their owning milestones update or explicitly classify them.

Intentionally deferred to later milestones:

- `ThreadManager`, `CodexThread`, `NewThread`, and `StartThreadOptions` remain for Milestone 4.
- `TurnItem`, `RolloutItem`, `StoredThread`, `StoredTurn`, and related history/store names remain for Milestone 5.
- Request processor and state filenames such as `thread_state.rs` and `thread_status.rs` remain for Milestone 6 unless changed by direct compile fallout from `ChatId`.

Milestone 3 assumptions:

- This milestone is a mechanical rename of existing concepts only; it does not add new Datax behavior.
- `ChatId` remains a UUID-backed identifier with the same serialization shape as `ThreadId`.
- Old event/field names are compatibility inputs, not Datax-native output names.
- The branch for this milestone is `codex/phase1-8-m3-protocol-primitives`; the tracking issue is #21.

Milestone 3 user-run commands:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just write-app-server-schema
    just test -p datax-protocol
    just test -p datax-app-server-protocol
    just test -p datax-app-server

Milestone 3 command assumptions:

- `just fmt` is required because Rust source changed across protocol and downstream call sites.
- `just write-app-server-schema` is required because the exported `ChatId` primitive and interaction event names affect generated schema and TypeScript artifacts.
- `just test -p datax-protocol` validates primitive serialization, event compatibility aliases, and rollout protocol behavior.
- `just test -p datax-app-server-protocol` validates API schema/export behavior after the protocol primitive rename.
- `just test -p datax-app-server` validates app-server call sites updated for `ChatId` and interaction event names.
- The user asked Codex not to run build/test commands during this milestone, so these commands are listed for user execution rather than run by Codex.

## Plan of Work

Milestone 1 is a boundary inventory. Search Datax-facing Rust and protocol files for `Codex`, `Thread`, `Turn`, `Item`, and common snake_case forms such as `thread_id` and `turn_id`. Classify each occurrence as a mechanical rename target, compatibility alias, downstream Codex bridge term, provenance, protected sandbox identifier, external dependency, or unrelated English. This milestone should update this plan with a concise inventory summary before code changes begin.

Milestone 2 renames the app-server protocol internals where the wire name is already Datax but the Rust field or type name remains Codex-era. For example, `ChatStartResponse` should have a Rust field named `chat`, and `InteractionStartResponse` should have a Rust field named `interaction`. Preserve intentional serde/TypeScript aliases such as `numTurns` only where compatibility is required, and document those aliases as compatibility.

Milestone 3 introduces or renames Datax primitive identifiers and event names. `ThreadId` becomes `ChatId` in Datax-facing code. `turn_id` becomes `interaction_id`. `EventMsg::TurnStarted`, `EventMsg::TurnComplete`, and `EventMsg::TurnAborted` become Datax-facing interaction event names. Any old event names needed for stored compatibility must be isolated behind conversion code.

Milestone 4 migrates live runtime names. The `ThreadManager` role becomes `ChatManager`. The `CodexThread` live handle role becomes a Datax chat/session handle, preferably `DataxChat` or `ChatSession` based on existing local naming clarity. Start/resume/interrupt/listener functions should be renamed according to the mapping while preserving behavior.

Milestone 5 migrates message and history names. `TurnItem` becomes `InteractionMessage`; `RolloutItem` becomes `RolloutMessage`. Stored types follow the same rule: `StoredThread` becomes `StoredChat`, `StoredTurn` becomes `StoredInteraction`, and `StoredThreadHistory` becomes `StoredChatHistory`. Existing stored data should remain readable through explicit compatibility conversions.

Milestone 6 updates app-server request processors and state tracking to depend on Datax-named contracts. `chat_processor.rs`, `interaction_processor.rs`, `thread_state.rs`, dynamic tool response handling, and goal/status tracking should speak in chat/interaction/message terms. The behavior should remain equivalent to the current app-server behavior.

Milestone 7 validates generated protocol and tests. Regenerate app-server schema fixtures if protocol shapes or TypeScript exports change. Run focused tests for every changed crate. If common, core, or protocol crates are changed broadly, ask before running the complete workspace test suite.

Milestone 8 updates Phase 2 bridge planning. Only after Datax-facing code owns Datax vocabulary should later plans introduce `AgentAdapter` and `codex-runtime` to map Datax `Chat`/`Interaction`/`Message` back to downstream Codex `Thread`/`Turn`/`Item`.

For Milestone 2, the user should run:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just write-app-server-schema
    just test -p datax-app-server-protocol
    just test -p datax-app-server

For Milestone 3, the user should run:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just write-app-server-schema
    just test -p datax-protocol
    just test -p datax-app-server-protocol
    just test -p datax-app-server

For Milestone 4, the user should run:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just fix -p datax-core
    just test -p datax-core
    just test -p datax-app-server

For Milestone 5, the user should run:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just test -p datax-protocol
    just test -p datax-thread-store
    just test -p datax-app-server

For Milestone 6, the user should run:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just fix -p datax-app-server
    just test -p datax-app-server

For Milestone 7, the user should run the focused commands from the changed crates, plus schema generation when protocol shapes changed:

    cd /home/mbellary/wsl/projects/datax
    git diff --check
    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fmt
    just write-app-server-schema
    just test -p datax-app-server-protocol
    just test -p datax-app-server

If Milestone 7 changes common, core, or protocol broadly, ask before running the complete workspace test suite:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test

For Milestone 8, the user should run:

    cd /home/mbellary/wsl/projects/datax
    git diff --check

No Rust tests are required for Milestone 8 unless the bridge planning update includes code or generated artifacts.

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
    just test -p datax-app-server-protocol

For app-server implementation changes, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p datax-app-server

For protocol schema changes, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just write-app-server-schema
    just test -p datax-app-server-protocol

Before finalizing a large app-server change, run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just fix -p datax-app-server

Do not run `cargo test` directly in `datax-rs`. Use `just test` according to repository instructions.

## Validation and Acceptance

The migration is accepted when Datax-facing code consistently follows the four-rule mapping and existing behavior still works.

Run:

    cd /home/mbellary/wsl/projects/datax
    rg -n "ThreadManager|CodexThread|ThreadId|TurnItem|RolloutItem|StoredThread|StoredTurn|StoredThreadHistory|EventMsg::Turn|thread_id|turn_id" datax-rs/app-server datax-rs/app-server-protocol/src/protocol/v2 datax-rs/thread-store datax-rs/core-api -g '*.rs'

Expected final result: remaining matches are only in explicitly named compatibility code, downstream Codex bridge code, provenance, or unrelated non-protocol usage. Datax request processors and native Datax protocol files should use `ChatManager`, `ChatId`, `InteractionMessage`, `RolloutMessage`, and interaction/message event names.

Run:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    just test -p datax-app-server-protocol
    just test -p datax-app-server

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
