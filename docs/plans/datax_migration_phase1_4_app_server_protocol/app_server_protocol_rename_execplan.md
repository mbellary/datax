# Phase 1.4 App-Server Protocol Rename

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` and the migration execution model in `docs/plans/Recommended-Datax-Migration-Execution-Model.md`.

## Purpose / Big Picture

Phase 1.4 renames the public app-server model from Thread, Turn, and Item terminology to Chat, Interaction, and Message terminology. After this milestone, Datax clients should call app-server v2 methods such as `chat/start`, `chat/read`, `interaction/start`, `interaction/interrupt`, and `message/*` notifications instead of the old thread, turn, and item names. The change is migration-only: behavior stays equivalent, but the public protocol names, request and response types, generated JSON schemas, generated TypeScript bindings, app-server README examples, and v2 integration tests move to the Datax terminology.

This milestone deliberately targets the app-server public API boundary. Internal runtime concepts in `datax-core` may continue to use thread or turn terminology where they represent existing execution engine internals, unless a public app-server type or method cannot compile without a local adapter rename.

## Progress

- [x] (2026-07-06T16:42:31Z) User confirmed Phase 1.3 tests passed and approved proceeding to the next phase.
- [x] (2026-07-06T16:42:31Z) Confirmed `main` was clean and created branch `datax/migration-phase1-4-app-server-protocol`.
- [x] (2026-07-06T16:42:31Z) Re-read `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`, `docs/plans/Recommended-Datax-Migration-Execution-Model.md`, and `PLANS.md`.
- [x] (2026-07-06T16:42:31Z) Identified Phase 1.4 as app-server model and protocol rename: `thread` to `chat`, `turn` to `interaction`, and `item` to `message` at the public app-server API boundary.
- [x] (2026-07-06T16:42:31Z) Performed initial dependency discovery across `codex-rs/app-server-protocol`, `codex-rs/app-server`, generated schemas, generated TypeScript bindings, and v2 integration tests.
- [ ] Create GitHub issue and draft PR for Phase 1.4 after the initial planning commit is pushed.
- [ ] Rename protocol source modules and exported v2 request, response, notification, and shared model types.
- [ ] Update app-server request dispatch, processor modules, and route method strings from thread and turn resources to chat and interaction resources.
- [ ] Regenerate app-server JSON schema fixtures and TypeScript bindings.
- [ ] Update app-server README examples and v2 integration tests to use the renamed public protocol.
- [ ] Run `just fmt` from `codex-rs` after code changes.
- [ ] Record all deferred validation commands and final outcome notes before the milestone exits.

## Surprises & Discoveries

- Observation: The public protocol rename is broader than only `protocol/v2/thread.rs`, `protocol/v2/turn.rs`, and `protocol/v2/item.rs`.
  Evidence: `rg -n "thread/|turn/|item/|Thread|Turn|Item" codex-rs/app-server-protocol/src/protocol/v2 codex-rs/app-server/src/request_processors codex-rs/app-server/tests/suite/v2 codex-rs/app-server/README.md | cut -d: -f1 | sort -u` returned protocol helper files, request processors, README examples, and many v2 suite files.

- Observation: Generated schema artifacts are part of the milestone surface.
  Evidence: `codex-rs/app-server-protocol/schema/json/v2` and `codex-rs/app-server-protocol/schema/typescript/v2` contain files such as `ThreadStartParams.json`, `ThreadStartParams.ts`, `TurnStartParams.json`, `TurnStartParams.ts`, `ThreadItem.json`, `ThreadStatusChangedNotification.json`, and related exports.

## Decision Log

- Decision: Limit Phase 1.4 to the app-server public API boundary and directly required server/test/generated artifacts.
  Rationale: The migration plan assigns persistence, fixtures, and snapshots to Phase 1.5. Internal engine modules such as `datax-core` thread management are not the public app-server protocol and should not be renamed in this phase unless required to keep the app-server boundary compiling.
  Date/Author: 2026-07-06 / Codex

- Decision: Do not provide backwards-compatible `thread/*`, `turn/*`, or `item/*` aliases in Phase 1.4.
  Rationale: The Phase 1 migration policy explicitly allows breaking API changes and does not require Codex client compatibility. Temporary aliases would add behavior not required for the clean Datax baseline.
  Date/Author: 2026-07-06 / Codex

- Decision: Treat `ThreadId` from `datax_protocol` as an internal/session identifier unless it is exposed by app-server v2 schema files.
  Rationale: The public app-server protocol should expose `chatId` naming, but core session identity may still be represented internally by `ThreadId` until a later internal concept cleanup phase. This keeps the change bounded to public API behavior.
  Date/Author: 2026-07-06 / Codex

## Outcomes & Retrospective

This section will be completed when the Phase 1.4 implementation is finished. The expected outcome is a migration-only app-server protocol rename with generated schema and TypeScript artifacts updated in the same branch.

## Context and Orientation

The app-server protocol is the JSON-RPC API used by Datax clients to start, read, resume, fork, and interact with sessions. A JSON-RPC method is a string such as `thread/start`; Phase 1.4 changes those public method strings to Datax migration terminology, for example `chat/start`. The protocol types are Rust structs and enums in `codex-rs/app-server-protocol/src/protocol/v2/`. They are exported to JSON schema files under `codex-rs/app-server-protocol/schema/json/` and TypeScript files under `codex-rs/app-server-protocol/schema/typescript/`.

The app-server implementation lives in `codex-rs/app-server/`. Request dispatch and business logic live mainly in `codex-rs/app-server/src/request_processors.rs` and files under `codex-rs/app-server/src/request_processors/`. Integration tests live under `codex-rs/app-server/tests/suite/v2/`; these tests exercise the public JSON-RPC methods and must be renamed with the public protocol.

The key public rename mapping for this milestone is:

| Old Public Name | New Public Name | Notes |
| --- | --- | --- |
| `thread/*` | `chat/*` | Public v2 methods and notifications that refer to a user-visible conversation/session. |
| `turn/*` | `interaction/*` | Public v2 methods and notifications that refer to one user/agent exchange. |
| `item/*` | `message/*` | Public v2 notifications and types that refer to stream or history entries. |
| `Thread` | `Chat` | Public app-server v2 schema type. |
| `ThreadId` / `threadId` | `ChatId` / `chatId` | Public app-server v2 field/type naming. Internal `datax_protocol::ThreadId` may remain behind adapters. |
| `Turn` | `Interaction` | Public app-server v2 schema type. |
| `TurnStatus` | `InteractionStatus` | Public app-server v2 status naming. |
| `ThreadItem` | `Message` | Public app-server v2 message/history item type. |

## Baseline

Starting branch: `main`.

Milestone branch: `datax/migration-phase1-4-app-server-protocol`.

Current repository state at branch creation: clean.

Known validation policy: builds and tests are staged. Do not run long build or test commands during this milestone unless the user explicitly requests them. The exact commands must still be recorded here so the user can run them later.

## File Inventory

The table below tracks files and file sets that belong to Phase 1.4. Rows marked `Pending` must become `Completed`, `Failed`, or `Not Required` before this milestone exits. A row may represent a generated file set when the exact files are produced by the schema generator; the command in `Remarks Notes` identifies the file set.

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/datax_migration_phase1_4_app_server_protocol/app_server_protocol_rename_execplan.md` | `In-Progress` | Living ExecPlan for Phase 1.4. |
| `docs/plans/datax_migration_phase1_4_app_server_protocol/github_issue.md` | `Pending` | GitHub issue body for milestone scope and acceptance criteria. |
| `docs/plans/datax_migration_phase1_4_app_server_protocol/pull_request.md` | `Pending` | Draft PR body for milestone summary and staged validation notes. |
| `codex-rs/app-server-protocol/src/protocol/v2/mod.rs` | `Pending` | Module exports for thread, turn, and item protocol modules must change to chat, interaction, and message modules. |
| `codex-rs/app-server-protocol/src/protocol/v2/thread.rs` | `Pending` | Primary public thread/chat request, response, and notification types. Expected to be moved or replaced by `chat.rs`. |
| `codex-rs/app-server-protocol/src/protocol/v2/turn.rs` | `Pending` | Primary public turn/interaction types. Expected to be moved or replaced by `interaction.rs`. |
| `codex-rs/app-server-protocol/src/protocol/v2/item.rs` | `Pending` | Primary public item/message types. Expected to be moved or replaced by `message.rs`. |
| `codex-rs/app-server-protocol/src/protocol/v2/thread_data.rs` | `Pending` | Thread data/history model references public thread/message names and likely needs chat naming. |
| `codex-rs/app-server-protocol/src/protocol/thread_history.rs` | `Pending` | Builds public history turns and items; must adapt to `Interaction` and `Message` type names if those public types are renamed. |
| `codex-rs/app-server-protocol/src/protocol/item_builders.rs` | `Pending` | Helper builders for public item/message values. |
| `codex-rs/app-server-protocol/src/protocol/item_builders_tests.rs` | `Pending` | Tests for item/message builders must follow renamed type names. |
| `codex-rs/app-server-protocol/src/protocol/common.rs` | `Pending` | Method registry and experimental API gates include method strings such as `thread/start` and `turn/start`. |
| `codex-rs/app-server-protocol/src/protocol/event_mapping.rs` | `Pending` | Maps core events to public app-server notifications. |
| `codex-rs/app-server-protocol/src/protocol/v2/tests.rs` | `Pending` | Protocol shape tests must use renamed public methods and schema types. |
| `codex-rs/app-server-protocol/src/export.rs` | `Pending` | Schema export logic and tests contain `ThreadId` examples and may require generated-name updates. |
| `codex-rs/app-server-protocol/src/schema_fixtures.rs` | `Pending` | Schema fixture support may reference generated type names. |
| `codex-rs/app-server-protocol/tests/schema_fixtures.rs` | `Pending` | Fixture validation tests must match regenerated schema outputs. |
| `codex-rs/app-server-protocol/schema/json/**` | `Pending` | Generated JSON schema files must be regenerated after source protocol rename. |
| `codex-rs/app-server-protocol/schema/typescript/**` | `Pending` | Generated TypeScript bindings must be regenerated after source protocol rename. |
| `codex-rs/app-server/src/request_processors.rs` | `Pending` | Request dispatch and protocol imports need method/type renames. |
| `codex-rs/app-server/src/request_processors/thread_processor.rs` | `Pending` | Thread request handling becomes chat request handling. Expected to be moved or replaced by `chat_processor.rs`. |
| `codex-rs/app-server/src/request_processors/turn_processor.rs` | `Pending` | Turn request handling becomes interaction request handling. Expected to be moved or replaced by `interaction_processor.rs`. |
| `codex-rs/app-server/src/request_processors/thread_delete.rs` | `Pending` | Public delete route and type names must align with chat naming. |
| `codex-rs/app-server/src/request_processors/thread_goal_processor.rs` | `Pending` | Public goal route/type names must align with chat naming if exposed through app-server v2. |
| `codex-rs/app-server/src/request_processors/thread_lifecycle.rs` | `Pending` | Thread lifecycle helpers publish public notifications and must align with chat naming at the API boundary. |
| `codex-rs/app-server/src/request_processors/thread_resume_redaction.rs` | `Pending` | Resume/read public payload helpers may expose thread/message names. |
| `codex-rs/app-server/src/request_processors/thread_summary.rs` | `Pending` | Summary request helpers may expose public thread naming. |
| `codex-rs/app-server/src/request_processors/thread_processor_tests.rs` | `Pending` | Processor tests must use renamed public methods and types. |
| `codex-rs/app-server/src/request_processors/thread_summary_tests.rs` | `Pending` | Summary tests must follow renamed public naming if touched by source changes. |
| `codex-rs/app-server/src/thread_state.rs` | `Pending` | Runtime state currently stores public `Thread` and `Turn` values; rename only if required by app-server protocol type changes. |
| `codex-rs/app-server/src/thread_status.rs` | `Pending` | Status notifications are public app-server surface and likely become chat status notifications. |
| `codex-rs/app-server/src/in_process.rs` | `Pending` | In-process client tests and examples call public methods such as `thread/start`. |
| `codex-rs/app-server/src/message_processor.rs` | `Pending` | Coordinates app-server events and may publish public thread/chat notifications. |
| `codex-rs/app-server/src/request_serialization.rs` | `Pending` | Serialization scopes may expose thread naming. |
| `codex-rs/app-server/README.md` | `Pending` | API contract documentation and examples must use `chat`, `interaction`, and `message`. |
| `codex-rs/app-server/tests/suite/v2/thread_*.rs` | `Pending` | Integration tests for public thread methods must be renamed or updated to chat methods. |
| `codex-rs/app-server/tests/suite/v2/turn_*.rs` | `Pending` | Integration tests for public turn methods must be renamed or updated to interaction methods. |
| `codex-rs/app-server/tests/suite/v2/mod.rs` | `Pending` | Test module list must track any renamed test files. |
| `codex-rs/app-server/tests/suite/v2/*.rs` | `Pending` | Non-thread test files that call `thread/start`, `turn/start`, or inspect thread/turn/item notifications must be updated. Exact files are identified with `rg -n "thread/|turn/|item/|Thread|Turn|Item" codex-rs/app-server/tests/suite/v2`. |
| `codex-rs/app-server-client/README.md` | `Pending` | Inspected because it documents app-server client usage; update only if it references renamed public methods. |
| `codex-rs/tui/**` | `Not Required` | Initial search found many internal TUI references to thread and turn concepts, but Phase 1.4 is scoped to app-server public protocol. TUI rename is deferred unless compilation forces an app-server type import update. |
| `codex-rs/core/**` | `Not Required` | Internal runtime thread and turn modules remain behind app-server adapters in this milestone unless compilation requires small compatibility edits. |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` references | `Not Required` | Protected sandbox identifiers are excluded from all migration rename operations. |

## Rename Exception Register

The following old terminology may remain after Phase 1.4:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`, because repository instructions explicitly protect these sandbox contracts.
- Internal `datax-core` runtime modules and types such as `codex_thread`, `ThreadId`, and turn execution internals when they are not exposed directly as app-server v2 schema names.
- Persisted rollout files, fixture names, and snapshots that are not regenerated app-server protocol artifacts. Phase 1.5 owns persistence, fixtures, and snapshots.
- Upstream provenance, license, historical references, and external names that do not represent active Datax product or public app-server API naming.
- Lowercase English words such as "turn" when used in ordinary prose outside the public protocol meaning, for example "turn on" or "turn into".

## Public Surface Checklist

This milestone intentionally touches app-server protocol methods, request and response type names, notification type names, generated JSON schemas, generated TypeScript bindings, app-server README API examples, and v2 app-server integration tests. It does not intentionally touch CLI flags, config keys, package names, crate names, persisted local state compatibility, product behavior, or UI layouts. If a touched public method is experimental, update the experimental API marker string in `codex-rs/app-server-protocol/src/protocol/common.rs` at the same time as the method rename.

## Dependency Order

First, rename the source protocol types and method strings in `codex-rs/app-server-protocol/src/protocol/v2/`, `common.rs`, event mapping, and builder helpers. Second, update app-server request processors and dispatch to compile against the renamed public types while preserving internal runtime behavior. Third, update tests and README examples to call the new public method strings. Fourth, regenerate JSON schema and TypeScript artifacts from the renamed source. Fifth, run `just fmt` from `codex-rs` and static searches to identify only documented exceptions. Long build and test commands remain deferred unless the user explicitly requests them.

## Plan of Work

Create the GitHub issue and draft PR immediately after the initial ExecPlan commit so all subsequent implementation commits attach to Phase 1.4.

Use `git mv` for source files whose module names are part of the public protocol, for example moving `protocol/v2/thread.rs` to `protocol/v2/chat.rs`, `protocol/v2/turn.rs` to `protocol/v2/interaction.rs`, and `protocol/v2/item.rs` to `protocol/v2/message.rs` if inspection confirms those modules are public-boundary modules rather than internal helpers. Rename request processor modules in the same way only when their filenames are public-routing concepts rather than internal persistence helpers.

Apply the type rename in a dependency-aware order. Do not blindly replace every `ThreadId` in the repository. Public app-server v2 schema fields should become `chat_id` in Rust and `chatId` on the wire, but internal calls into `datax_protocol::ThreadId` should convert at the boundary and may retain the internal type name.

Regenerate app-server schema artifacts after the Rust source compiles sufficiently for the generation commands. Include generated JSON and TypeScript changes in this same branch so reviewers can verify the public contract changed with the source.

## Concrete Steps

From the repository root, confirm the branch:

    git status --short --branch

Create the GitHub issue and draft PR after committing this ExecPlan:

    gh issue create --title "Phase 1.4: App-server model and protocol rename" --body-file docs/plans/datax_migration_phase1_4_app_server_protocol/github_issue.md
    gh pr create --draft --title "Phase 1.4: App-server model and protocol rename" --body-file docs/plans/datax_migration_phase1_4_app_server_protocol/pull_request.md

Use `rg` to maintain the file inventory:

    rg -n "thread/|turn/|item/|Thread|Turn|Item|thread_id|turn_id|item_id|threadId|turnId|itemId" codex-rs/app-server-protocol codex-rs/app-server

Use `rg` to check for generated artifacts that still expose old public names:

    rg -n "thread/|turn/|item/|Thread|Turn|Item|threadId|turnId|itemId" codex-rs/app-server-protocol/schema/json codex-rs/app-server-protocol/schema/typescript

After source edits, run formatter from `codex-rs`:

    just fmt

## Validation Matrix

| Command | Working Directory | Status | Expected Result |
| --- | --- | --- | --- |
| `git diff --check` | repository root | `Deferred` | No whitespace errors. |
| `rg -n "Data[X]" .` | repository root | `Deferred` | No forbidden mixed-case spelling. |
| `rg -n "thread/|turn/|item/" codex-rs/app-server-protocol/src codex-rs/app-server/src codex-rs/app-server/tests/suite/v2 codex-rs/app-server/README.md` | repository root | `Deferred` | No old public app-server method strings remain except documented deferrals. |
| `rg -n "ThreadStart|ThreadRead|ThreadList|ThreadResume|ThreadFork|TurnStart|TurnInterrupt|ThreadItem|TurnStatus" codex-rs/app-server-protocol/src codex-rs/app-server/src codex-rs/app-server/tests/suite/v2` | repository root | `Deferred` | No old public app-server v2 schema type names remain except documented internal adapters. |
| `rg -n "threadId|turnId|itemId" codex-rs/app-server-protocol/schema/json codex-rs/app-server-protocol/schema/typescript` | repository root | `Deferred` | Generated app-server schema artifacts expose `chatId`, `interactionId`, or `messageId` where applicable. |
| `just fmt` | `codex-rs` | `Pending` | Rust formatting completes after source edits. |
| `just write-app-server-schema` | `codex-rs` | `Deferred` | App-server schema and TypeScript artifacts regenerate with renamed public names. |
| `just write-app-server-schema --experimental` | `codex-rs` | `Deferred` | Experimental app-server schema artifacts regenerate with renamed experimental method markers. |
| `just test -p datax-app-server-protocol` | `codex-rs` | `Deferred` | Protocol schema and TypeScript fixture tests pass. |
| `just test -p datax-app-server` | `codex-rs` | `Deferred` | App-server request processor and integration tests pass with renamed public methods. |

## Validation and Acceptance

The commands in this section are intentionally explicit so the user can run them during the staged migration validation pass. Commands marked deferred should not be run by Codex during this milestone unless the user explicitly asks.

From the repository root, run the whitespace check and expect no output:

    git diff --check

From the repository root, run the forbidden spelling check and expect no output:

    rg -n "Data[X]" .

From the repository root, verify old public method strings are gone from app-server source, tests, and README except documented deferrals:

    rg -n "thread/|turn/|item/" codex-rs/app-server-protocol/src codex-rs/app-server/src codex-rs/app-server/tests/suite/v2 codex-rs/app-server/README.md

From the repository root, verify old public v2 schema type names are gone from app-server source and v2 tests except documented internal adapters:

    rg -n "ThreadStart|ThreadRead|ThreadList|ThreadResume|ThreadFork|TurnStart|TurnInterrupt|ThreadItem|TurnStatus" codex-rs/app-server-protocol/src codex-rs/app-server/src codex-rs/app-server/tests/suite/v2

From the repository root, verify generated schema artifacts no longer expose old public id field names:

    rg -n "threadId|turnId|itemId" codex-rs/app-server-protocol/schema/json codex-rs/app-server-protocol/schema/typescript

From `codex-rs`, run the formatter and expect it to complete:

    just fmt

From `codex-rs`, regenerate app-server schema artifacts and expect checked-in JSON and TypeScript outputs to reflect chat, interaction, and message naming:

    just write-app-server-schema

From `codex-rs`, regenerate experimental app-server schema artifacts and expect experimental markers to use the renamed public methods:

    just write-app-server-schema --experimental

From `codex-rs`, run the targeted protocol tests and expect them to pass:

    just test -p datax-app-server-protocol

From `codex-rs`, run the targeted app-server tests and expect them to pass:

    just test -p datax-app-server

Acceptance for this milestone requires that Datax app-server v2 exposes chat, interaction, and message naming in Rust protocol types, JSON-RPC method strings, generated JSON schemas, generated TypeScript bindings, README examples, and v2 integration tests, with old public thread, turn, and item names either removed or documented as deferred internal exceptions.

## Idempotence and Recovery

Most edits in this milestone are renames and generated artifact updates. If a source rename goes wrong, use `git status --short` to identify moved and modified files, then inspect the specific diff before making corrective edits. Do not use broad destructive commands. If generated schema output is inconsistent, rerun the exact generation commands from `codex-rs` after source names are corrected.

Because public method aliases are intentionally not added, a temporary compile failure during the implementation is expected until protocol source, app-server dispatch, tests, and generated artifacts are updated together. Keep commits focused so a broken band can be isolated by file group.

## Artifacts and Notes

GitHub issue and draft PR URLs will be recorded here after creation.

Initial discovery command:

    rg -n "thread/|turn/|item/|Thread|Turn|Item" codex-rs/app-server-protocol/src/protocol/v2 codex-rs/app-server/src/request_processors codex-rs/app-server/tests/suite/v2 codex-rs/app-server/README.md | cut -d: -f1 | sort -u

This command found the protocol v2 modules, request processors, v2 suite tests, and app-server README as the primary public API surface.

## Interfaces and Dependencies

At the end of this milestone, the app-server v2 protocol should define public Rust types such as `ChatStartParams`, `ChatStartResponse`, `InteractionStartParams`, `InteractionStartResponse`, `InteractionStatus`, and `Message` or more specific message notification names. The generated TypeScript bindings under `codex-rs/app-server-protocol/schema/typescript/v2/` should export matching type names.

App-server request dispatch should route method strings such as `chat/start`, `chat/read`, `chat/list`, `chat/resume`, `chat/fork`, `interaction/start`, `interaction/interrupt`, and any renamed message notifications or request surfaces defined by the protocol. Internal calls into `datax_core` and `datax_protocol` may keep engine-level thread and turn identifiers behind boundary conversion code.

## Change Notes

- 2026-07-06: Created the initial Phase 1.4 ExecPlan, file inventory, dependency order, validation matrix, and acceptance commands before implementation edits. This records the approved staged-test policy and keeps the protocol rename bounded to app-server public API surfaces.
