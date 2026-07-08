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
- [x] (2026-07-06T16:42:31Z) Performed initial dependency discovery across `datax-rs/app-server-protocol`, `datax-rs/app-server`, generated schemas, generated TypeScript bindings, and v2 integration tests.
- [x] (2026-07-06T16:42:31Z) Created GitHub issue #7 and draft PR #8 for Phase 1.4.
- [x] (2026-07-07T00:00:00Z) Renamed protocol source modules and exported v2 request, response, notification, and shared model types from thread/turn/item to chat/interaction/message terminology.
- [x] (2026-07-07T00:00:00Z) Updated app-server request dispatch, processor modules, downstream protocol consumers, README examples, and v2 integration tests to use the renamed public protocol symbols and method strings.
- [x] (2026-07-07T00:00:00Z) Regenerated app-server JSON schema fixtures and TypeScript bindings; user ran `just write-app-server-schema` and pushed commit `dd5fd69a44`.
- [x] (2026-07-07T00:00:00Z) Attempted `just write-app-server-schema`; the sandbox run failed on missing crate downloads, then the escalated run downloaded dependencies and compiled for several minutes before WSL became unstable. Schema generation is deferred to the user handoff command below.
- [x] (2026-07-07T00:00:00Z) Cleaned up remaining public schema/docs leaks after schema generation: `numTurns` now advertises as `numInteractions`, rollback and non-steerable error schema values use chat/interaction naming, and generated TypeScript/JSON descriptions no longer expose old thread/turn names for Chat/Interaction API docs.
- [x] (2026-07-07T00:00:00Z) Ran `just fmt` from `datax-rs` after code changes.
- [x] (2026-07-07T00:00:00Z) Completed lightweight acceptance searches; targeted app-server tests remain staged/deferred per the migration execution model.
- [x] (2026-07-07T00:00:00Z) Fixed deferred `just test -p datax-app-server` compile fallout in `datax-tools`, where the request-plugin-install helper still constructed `McpServerElicitationRequestParams` with obsolete `thread_id` and `turn_id` fields instead of `chat_id` and `interaction_id`.
- [x] (2026-07-07T00:00:00Z) Fixed deferred `just test -p datax-app-server` compile fallout in `datax-analytics`, where analytics ingestion still read old app-server public fields such as `thread_id`, `turn_id`, `item_id`, and `target_item_id` from renamed chat, interaction, and message protocol payloads.
- [x] (2026-07-07T00:00:00Z) Fixed deferred `just test -p datax-app-server` compile fallout in core, app-server, analytics tests, and TUI where app-server protocol helper names and core permission scope variants still used the removed `ThreadHistoryBuilder`, `McpServerElicitationRequestParams.thread_id`, `McpServerElicitationRequestParams.turn_id`, and `PermissionGrantScope::Interaction` symbols.
- [x] (2026-07-07T00:00:00Z) Fixed deferred `just test -p datax-app-server` compile fallout inside app-server implementation and test support where public chat/interaction/message protocol names had been mechanically applied to internal core, thread-store, state, and rollout adapters that still require `thread_id`, `turn_id`, `item_id`, `items`, and `thread_source`.
- [x] (2026-07-07T00:00:00Z) Fixed the next deferred `just test -p datax-app-server` compile batch in app-server test modules, response routing tests, filters, token usage replay, and extension test fixtures where enum variants needed explicit namespaces or internal test fixtures still used public chat/interaction/message field names.
- [x] (2026-07-07T00:00:00Z) Fixed deferred `just test -p datax-app-server` compile fallout in the app-server integration suite where v2 test requests and notifications needed generated enum variant imports, and thread-store/core fixtures still used public chat field names instead of internal thread field names.

## Surprises & Discoveries

- Observation: The public protocol rename is broader than only `protocol/v2/thread.rs`, `protocol/v2/turn.rs`, and `protocol/v2/item.rs`.
  Evidence: `rg -n "thread/|turn/|item/|Thread|Turn|Item" datax-rs/app-server-protocol/src/protocol/v2 datax-rs/app-server/src/request_processors datax-rs/app-server/tests/suite/v2 datax-rs/app-server/README.md | cut -d: -f1 | sort -u` returned protocol helper files, request processors, README examples, and many v2 suite files.

- Observation: Generated schema artifacts are part of the milestone surface.
  Evidence: `datax-rs/app-server-protocol/schema/json/v2` and `datax-rs/app-server-protocol/schema/typescript/v2` contain files such as `ThreadStartParams.json`, `ThreadStartParams.ts`, `TurnStartParams.json`, `TurnStartParams.ts`, `ThreadItem.json`, `ThreadStatusChangedNotification.json`, and related exports.

- Observation: The app-server protocol types are consumed outside the app-server crates.
  Evidence: Source updates touched downstream consumers in `datax-rs/analytics`, `datax-rs/exec`, `datax-rs/external-agent-sessions`, and `datax-rs/tui` because those crates import `datax_app_server_protocol` request, response, and notification types.

- Observation: Schema generation is the first expensive command in this phase and should be user-run for this checkpoint.
  Evidence: `just write-app-server-schema` first failed in the sandbox with `Could not resolve host: static.crates.io`; the escalated run downloaded crates and continued compiling for multiple minutes before the user's WSL setup became unstable.

- Observation: Generated schemas still contained a few old public terms after the first successful schema generation.
  Evidence: Follow-up searches found generated `numTurns`, `threadRollbackFailed`, `activeTurnNotSteerable`, `turnKind`, and public Chat/Interaction descriptions using thread/turn wording. Source comments and generated artifacts were updated so the advertised schema now uses `numInteractions`, `chatRollbackFailed`, `activeInteractionNotSteerable`, `interactionKind`, and chat/interaction wording.

- Observation: The `datax-tools` crate constructs app-server elicitation protocol payloads used by app-server tests and is part of the Phase 1.4 compile surface.
  Evidence: The user-run `just test -p datax-app-server` reported `McpServerElicitationRequestParams` has no `thread_id` or `turn_id` fields in `datax-rs/tools/src/request_plugin_install.rs`; those fields are now `chat_id` and `interaction_id`.

- Observation: The `datax-analytics` crate consumes app-server v2 request, response, and notification types and maps them into the analytics event schema.
  Evidence: The user-run `just test -p datax-app-server` reported compile errors in `datax-rs/analytics/src/reducer.rs`, `datax-rs/analytics/src/events.rs`, `datax-rs/analytics/src/facts.rs`, and `datax-rs/analytics/src/lib.rs` for stale app-server fields and renamed state types.

- Observation: `just test -p datax-app-server` compiles broader downstream test and UI/support code than the app-server crate alone.
  Evidence: A user-run compile reported errors in `datax-rs/core/src/thread_manager.rs`, `datax-rs/core/src/session/mcp.rs`, `datax-rs/core/src/session/mod.rs`, and `datax-rs/core/src/mcp_tool_call.rs`; follow-up static searches also found stale core permission enum variants in app-server, analytics, and TUI test/support files.

- Observation: App-server source is an adapter boundary, not a pure rename surface.
  Evidence: A user-run compile reported 184 app-server errors where internal crates still expose `ThreadSortKey`, `ThreadGoal`, `ThreadGoalStatus`, `StoredThread.thread_id`, `StoredThreadHistory.items`, `ThreadStoreError::ThreadNotFound { thread_id }`, core event `turn_id`, and realtime `item_id`, while app-server v2 public payloads intentionally expose `chat_id`, `interaction_id`, `message_id`, and `chat_source`.

- Observation: App-server unit tests instantiate both public protocol payloads and internal core/store fixtures in the same files.
  Evidence: The follow-up user-run compile reported stale internal fields in `datax-rs/app-server/src/bespoke_event_handling.rs` and `datax-rs/app-server/src/request_processors/chat_processor_tests.rs`, while adjacent assertions still correctly reference public app-server fields such as `target_message_id`, `interaction_id`, and `chat_source`.

- Observation: The app-server integration suite also mixes generated public app-server requests with internal persistence fixtures.
  Evidence: The user-run compile reported `ClientRequest` variants such as `ChatStart` and `InteractionStart` used unqualified in v2 suite files, plus stale thread-store fixture fields in `chat_read.rs`, `chat_unarchive.rs`, `remote_thread_store.rs`, and `conversation_summary.rs`.

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

Phase 1.4 completed the migration-only app-server protocol rename at the public API boundary. App-server v2 source, request dispatch, downstream consumers, README examples, integration tests, generated JSON schemas, and generated TypeScript bindings now use Chat, Interaction, and Message terminology for the public protocol. Internal runtime/core names such as `ThreadId`, `TurnStartedEvent`, thread-store helpers, and selected Rust field names hidden behind serde/TS renames remain documented exceptions for later cleanup phases.

Build and test execution remains staged. The user ran schema generation and will run the targeted test commands from the Validation Matrix during the post-implementation validation pass.

## Context and Orientation

The app-server protocol is the JSON-RPC API used by Datax clients to start, read, resume, fork, and interact with sessions. A JSON-RPC method is a string such as `thread/start`; Phase 1.4 changes those public method strings to Datax migration terminology, for example `chat/start`. The protocol types are Rust structs and enums in `datax-rs/app-server-protocol/src/protocol/v2/`. They are exported to JSON schema files under `datax-rs/app-server-protocol/schema/json/` and TypeScript files under `datax-rs/app-server-protocol/schema/typescript/`.

The app-server implementation lives in `datax-rs/app-server/`. Request dispatch and business logic live mainly in `datax-rs/app-server/src/request_processors.rs` and files under `datax-rs/app-server/src/request_processors/`. Integration tests live under `datax-rs/app-server/tests/suite/v2/`; these tests exercise the public JSON-RPC methods and must be renamed with the public protocol.

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
| `docs/plans/datax_migration_phase1_4_app_server_protocol/app_server_protocol_rename_execplan.md` | `Completed` | Living ExecPlan for Phase 1.4; updated with GitHub artifact URLs. |
| `docs/plans/datax_migration_phase1_4_app_server_protocol/github_issue.md` | `Completed` | GitHub issue body used to create issue #7. |
| `docs/plans/datax_migration_phase1_4_app_server_protocol/pull_request.md` | `Completed` | Draft PR body used to create PR #8. |
| `datax-rs/app-server-protocol/src/protocol/v2/mod.rs` | `Completed` | Module exports for thread, turn, and item protocol modules must change to chat, interaction, and message modules. |
| `datax-rs/app-server-protocol/src/protocol/v2/thread.rs` | `Completed` | Primary public thread/chat request, response, and notification types. Expected to be moved or replaced by `chat.rs`. |
| `datax-rs/app-server-protocol/src/protocol/v2/turn.rs` | `Completed` | Primary public turn/interaction types. Expected to be moved or replaced by `interaction.rs`. |
| `datax-rs/app-server-protocol/src/protocol/v2/item.rs` | `Completed` | Primary public item/message types. Expected to be moved or replaced by `message.rs`. |
| `datax-rs/app-server-protocol/src/protocol/v2/thread_data.rs` | `Completed` | Thread data/history model references public thread/message names and likely needs chat naming. |
| `datax-rs/app-server-protocol/src/protocol/thread_history.rs` | `Completed` | Builds public history turns and items; must adapt to `Interaction` and `Message` type names if those public types are renamed. |
| `datax-rs/app-server-protocol/src/protocol/item_builders.rs` | `Completed` | Helper builders for public item/message values. |
| `datax-rs/app-server-protocol/src/protocol/item_builders_tests.rs` | `Completed` | Tests for item/message builders must follow renamed type names. |
| `datax-rs/app-server-protocol/src/protocol/common.rs` | `Completed` | Method registry and experimental API gates include method strings such as `thread/start` and `turn/start`. |
| `datax-rs/app-server-protocol/src/protocol/event_mapping.rs` | `Completed` | Maps core events to public app-server notifications. |
| `datax-rs/app-server-protocol/src/protocol/v2/tests.rs` | `Completed` | Protocol shape tests must use renamed public methods and schema types. |
| `datax-rs/app-server-protocol/src/export.rs` | `Completed` | Schema export logic and tests contain `ThreadId` examples and may require generated-name updates. |
| `datax-rs/app-server-protocol/src/schema_fixtures.rs` | `Completed` | Schema fixture support may reference generated type names. |
| `datax-rs/app-server-protocol/tests/schema_fixtures.rs` | `Completed` | Fixture validation tests must match regenerated schema outputs. |
| `datax-rs/app-server-protocol/schema/json/**` | `Completed` | Generated JSON schema files must be regenerated after source protocol rename. |
| `datax-rs/app-server-protocol/schema/typescript/**` | `Completed` | Generated TypeScript bindings must be regenerated after source protocol rename. |
| `datax-rs/app-server/src/request_processors.rs` | `Completed` | Request dispatch and protocol imports need method/type renames. |
| `datax-rs/app-server/src/request_processors/thread_processor.rs` | `Completed` | Thread request handling becomes chat request handling. Expected to be moved or replaced by `chat_processor.rs`. |
| `datax-rs/app-server/src/request_processors/turn_processor.rs` | `Completed` | Turn request handling becomes interaction request handling. Expected to be moved or replaced by `interaction_processor.rs`. |
| `datax-rs/app-server/src/request_processors/thread_delete.rs` | `Completed` | Public delete route and type names must align with chat naming. |
| `datax-rs/app-server/src/request_processors/thread_goal_processor.rs` | `Completed` | Public goal route/type names must align with chat naming if exposed through app-server v2. |
| `datax-rs/app-server/src/request_processors/thread_lifecycle.rs` | `Completed` | Thread lifecycle helpers publish public notifications and must align with chat naming at the API boundary. |
| `datax-rs/app-server/src/request_processors/thread_resume_redaction.rs` | `Completed` | Resume/read public payload helpers may expose thread/message names. |
| `datax-rs/app-server/src/request_processors/thread_summary.rs` | `Completed` | Summary request helpers may expose public thread naming. |
| `datax-rs/app-server/src/request_processors/external_agent_session_import.rs` | `Completed` | External-agent import writes through internal thread-store APIs, so store params use `thread_id`, `parent_thread_id`, `thread_source`, and `items` behind public app-server chat terminology. |
| `datax-rs/app-server/src/request_processors/feedback_processor.rs` | `Completed` | Feedback snapshot uses internal `thread_id` while app-server request context keeps public chat naming. |
| `datax-rs/app-server/src/request_processors/token_usage_replay.rs` | `Completed` | Token usage replay emits public chat token usage notifications and imports the generated notification enum variant explicitly. |
| `datax-rs/app-server/src/request_processors/thread_processor_tests.rs` | `Completed` | Processor tests must use renamed public methods and types. |
| `datax-rs/app-server/src/request_processors/chat_processor_tests.rs` | `Completed` | Chat processor tests construct internal `StoredThread`, `SessionMeta`, `ThreadConfigSnapshot`, and `TurnStartedEvent` fixtures that still use internal field names. |
| `datax-rs/app-server/src/request_processors/thread_summary_tests.rs` | `Completed` | Summary tests must follow renamed public naming if touched by source changes. |
| `datax-rs/app-server/src/thread_state.rs` | `Completed` | Runtime state currently stores public `Thread` and `Turn` values; rename only if required by app-server protocol type changes. |
| `datax-rs/app-server/src/thread_status.rs` | `Completed` | Status notifications are public app-server surface and likely become chat status notifications. |
| `datax-rs/app-server/src/in_process.rs` | `Completed` | In-process client tests and examples call public methods such as `thread/start`. |
| `datax-rs/app-server/src/message_processor.rs` | `Completed` | Coordinates app-server events and may publish public thread/chat notifications. |
| `datax-rs/app-server/src/message_processor_tracing_tests.rs` | `Completed` | Tracing tests construct `ClientRequest` and match `ServerNotification` variants, so renamed variants are now explicitly namespaced. |
| `datax-rs/app-server/src/outgoing_message.rs` | `Completed` | Response routing tests construct generated `ClientResponsePayload` variants explicitly. |
| `datax-rs/app-server/src/request_serialization.rs` | `Completed` | Serialization scopes may expose thread naming. |
| `datax-rs/app-server/src/attestation.rs` | `Completed` | Attestation context uses internal `thread_id`; app-server helper passes it through without public schema exposure. |
| `datax-rs/app-server/src/extensions.rs` | `Completed` | Extension event sink adapts internal `ThreadGoalUpdatedEvent.thread_id` and `turn_id` into public chat goal notifications. |
| `datax-rs/app-server/src/filters.rs` | `Completed` | Filter tests construct internal `SubAgentSource::ThreadSpawn` fixtures with `parent_thread_id`. |
| `datax-rs/app-server/README.md` | `Completed` | API contract documentation and examples must use `chat`, `interaction`, and `message`. |
| `datax-rs/app-server/tests/common/rollout.rs` | `Completed` | App-server test support constructs core `SessionMeta`, which still uses `parent_thread_id` and `thread_source` internally. |
| `datax-rs/app-server/tests/suite/v2/thread_*.rs` | `Completed` | Integration tests for public thread methods must be renamed or updated to chat methods. |
| `datax-rs/app-server/tests/suite/v2/turn_*.rs` | `Completed` | Integration tests for public turn methods must be renamed or updated to interaction methods. |
| `datax-rs/app-server/tests/suite/v2/mod.rs` | `Completed` | Test module list must track any renamed test files. |
| `datax-rs/app-server/tests/suite/v2/*.rs` | `Completed` | Non-thread test files that call `thread/start`, `turn/start`, or inspect thread/turn/item notifications must be updated. Exact files are identified with `rg -n "thread/|turn/|item/|Thread|Turn|Item" datax-rs/app-server/tests/suite/v2`. |
| `datax-rs/app-server/tests/suite/v2/chat_fork.rs` | `Completed` | Token usage notification matches now import generated `ServerNotification` variants explicitly. |
| `datax-rs/app-server/tests/suite/v2/chat_list.rs` | `Completed` | Tests use public `ChatSortKey` at the API boundary and internal `ThreadSortKey` plus `ThreadsPage.items` for core repair helpers. |
| `datax-rs/app-server/tests/suite/v2/chat_read.rs` | `Completed` | Tests import generated `ClientRequest` variants and use internal thread-store fixture field names. |
| `datax-rs/app-server/tests/suite/v2/chat_resume.rs` | `Completed` | Tests import generated notification variants and keep core rollout events/session metadata on internal `turn_id`, `parent_thread_id`, and `thread_source` fields. |
| `datax-rs/app-server/tests/suite/v2/chat_unarchive.rs` | `Completed` | Tests import generated `ClientRequest` variants and use internal thread-store fixture field names. |
| `datax-rs/app-server/tests/suite/v2/remote_thread_store.rs` | `Completed` | In-process remote store tests import generated request/notification variants and use internal thread-store fixture field names. |
| `datax-rs/app-server/tests/suite/conversation_summary.rs` | `Completed` | Conversation summary tests use internal thread-store fixture field names when seeding an in-memory store. |
| `datax-rs/app-server-client/README.md` | `Completed` | Inspected because it documents app-server client usage; update only if it references renamed public methods. |
| `datax-rs/tui/**` | `Completed` | Direct `datax_app_server_protocol` imports and usages were updated to the renamed app-server API types. Internal TUI thread/session vocabulary remains deferred. |
| `datax-rs/core/**` | `Completed` | Small compatibility edits were required where core constructs or consumes app-server protocol types. Internal `ThreadId`, thread manager, and turn execution vocabulary remain deferred. |
| `datax-rs/analytics/src/reducer.rs` | `Completed` | App-server public `chat_id`, `interaction_id`, `message_id`, `parent_chat_id`, and `chat_source` fields are mapped into the existing analytics event schema fields. |
| `datax-rs/analytics/src/events.rs` | `Completed` | Goal event status type updated to the state crate's internal `ThreadGoalStatus`; analytics event field names remain unchanged. |
| `datax-rs/analytics/src/facts.rs` | `Completed` | Goal fact status type updated to the state crate's internal `ThreadGoalStatus`; analytics fact field names remain unchanged. |
| `datax-rs/analytics/src/lib.rs` | `Completed` | Public re-export updated from removed `TurnStatus` to existing `InteractionStatus`. |
| `datax-rs/analytics/src/analytics_client_tests.rs` | `Completed` | Test expectations that construct core permission approval responses now use internal `PermissionGrantScope::Turn`; app-server public `Interaction` naming remains unchanged. |
| `datax-rs/analytics/**` | `Completed` | Direct app-server protocol request/response/notification type usage was updated to the renamed public API types while analytics fact names remain internal. |
| `datax-rs/exec/**` | `Completed` | Direct app-server protocol request/response/notification type usage was updated to the renamed public API types while exec event names remain internal. |
| `datax-rs/external-agent-sessions/**` | `Completed` | Direct app-server protocol message type usage was updated to the renamed public API type. |
| `datax-rs/app-server/src/bespoke_event_handling.rs` | `Completed` | App-server v2 permissions response handling now maps public `Interaction` scope to core `PermissionGrantScope::Turn`; core fallback/test expectations use the internal variant name. |
| `datax-rs/core/src/thread_manager.rs` | `Completed` | Core snapshot helper imports the renamed app-server protocol `ChatHistoryBuilder`; internal thread manager naming remains deferred. |
| `datax-rs/core/src/session/mcp.rs` | `Completed` | Converts app-server elicitation `interaction_id` into the internal `ElicitationRequestEvent.turn_id` field. |
| `datax-rs/core/src/mcp_tool_call.rs` | `Completed` | App-server elicitation request constructors now use `chat_id` and `interaction_id`. |
| `datax-rs/core/src/mcp_tool_call_tests.rs` | `Completed` | App-server elicitation request expectations now use `chat_id` and `interaction_id`. |
| `datax-rs/core/src/session/mod.rs` | `Completed` | Core permission responses now use the internal `PermissionGrantScope::Turn` variant. |
| `datax-rs/core/src/session/tests.rs` | `Completed` | Core tests updated for app-server elicitation fields and core permission scope variant rename. |
| `datax-rs/core/src/session/tests/guardian_tests.rs` | `Completed` | Guardian permission response tests now use internal `PermissionGrantScope::Turn`. |
| `datax-rs/tui/src/bottom_pane/approval_overlay.rs` | `Completed` | TUI permission approval responses submit the core `PermissionGrantScope::Turn` variant while UI behavior remains unchanged. |
| `datax-rs/tools/src/request_plugin_install.rs` | `Completed` | Downstream app-server elicitation protocol constructor updated from obsolete `thread_id` and `turn_id` fields to `chat_id` and `interaction_id`. |
| `datax-rs/tools/src/request_plugin_install_tests.rs` | `Completed` | Focused unit expectations updated to match the renamed app-server elicitation protocol payload fields. |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` references | `Not Required` | Protected sandbox identifiers are excluded from all migration rename operations. |

## Rename Exception Register

The following old terminology may remain after Phase 1.4:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`, because repository instructions explicitly protect these sandbox contracts.
- Internal `datax-core` runtime modules and types such as `codex_thread`, `ThreadId`, and turn execution internals when they are not exposed directly as app-server v2 schema names.
- Persisted rollout files, fixture names, and snapshots that are not regenerated app-server protocol artifacts. Phase 1.5 owns persistence, fixtures, and snapshots.
- Upstream provenance, license, historical references, and external names that do not represent active Datax product or public app-server API naming.
- Lowercase English words such as "turn" when used in ordinary prose outside the public protocol meaning, for example "turn on" or "turn into".

## Public Surface Checklist

This milestone intentionally touches app-server protocol methods, request and response type names, notification type names, generated JSON schemas, generated TypeScript bindings, app-server README API examples, and v2 app-server integration tests. It does not intentionally touch CLI flags, config keys, package names, crate names, persisted local state compatibility, product behavior, or UI layouts. If a touched public method is experimental, update the experimental API marker string in `datax-rs/app-server-protocol/src/protocol/common.rs` at the same time as the method rename.

## Dependency Order

First, rename the source protocol types and method strings in `datax-rs/app-server-protocol/src/protocol/v2/`, `common.rs`, event mapping, and builder helpers. Second, update app-server request processors and dispatch to compile against the renamed public types while preserving internal runtime behavior. Third, update tests and README examples to call the new public method strings. Fourth, regenerate JSON schema and TypeScript artifacts from the renamed source. Fifth, run `just fmt` from `datax-rs` and static searches to identify only documented exceptions. Long build and test commands remain deferred unless the user explicitly requests them.

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

    rg -n "thread/|turn/|item/|Thread|Turn|Item|thread_id|turn_id|item_id|threadId|turnId|itemId" datax-rs/app-server-protocol datax-rs/app-server

Use `rg` to check for generated artifacts that still expose old public names:

    rg -n "thread/|turn/|item/|Thread|Turn|Item|threadId|turnId|itemId" datax-rs/app-server-protocol/schema/json datax-rs/app-server-protocol/schema/typescript

After source edits, run formatter from `datax-rs`:

    just fmt

## Validation Matrix

| Command | Working Directory | Status | Expected Result |
| --- | --- | --- | --- |
| `git diff --check` | repository root | `Completed` | No whitespace errors. |
| `rg -n "Data[X]" .` | repository root | `Completed` | No forbidden mixed-case spelling. |
| `rg -n "thread/|turn/|item/" datax-rs/app-server-protocol/src datax-rs/app-server/src datax-rs/app-server/tests/suite/v2 datax-rs/app-server/README.md` | repository root | `Completed` | No old public app-server method strings remain except documented deferrals. |
| `rg -n "ThreadStart|ThreadRead|ThreadList|ThreadResume|ThreadFork|TurnStart|TurnInterrupt|ThreadItem|TurnStatus" datax-rs/app-server-protocol/src datax-rs/app-server/src datax-rs/app-server/tests/suite/v2` | repository root | `Completed` | Only documented internal/core event names and export test fixture names remain. |
| `rg -n "threadId|turnId|itemId|numTurns|threadRollbackFailed|activeTurnNotSteerable|turnKind|NonSteerableTurnKind" datax-rs/app-server-protocol/schema/json datax-rs/app-server-protocol/schema/typescript` | repository root | `Completed` | Generated app-server schema artifacts expose `chatId`, `interactionId`, `messageId`, `numInteractions`, `chatRollbackFailed`, `activeInteractionNotSteerable`, and `interactionKind` where applicable. |
| `just fmt` | `datax-rs` | `Completed` | Rust formatting completes after source edits. |
| `just write-app-server-schema` | `datax-rs` | `Completed` | App-server schema and TypeScript artifacts regenerate with renamed public names. User ran this command and pushed commit `dd5fd69a44`. |
| `just write-app-server-schema --experimental` | `datax-rs` | `Deferred` | Experimental app-server schema artifacts regenerate with renamed experimental method markers. |
| `just test -p datax-app-server-protocol` | `datax-rs` | `Deferred` | Protocol schema and TypeScript fixture tests pass. |
| `just test -p datax-app-server` | `datax-rs` | `Deferred` | App-server request processor and integration tests pass with renamed public methods. User-run compile fallout in `datax-tools`, `datax-analytics`, `datax-core`, app-server support code, and TUI permission response construction was fixed; command awaits user rerun. |
| `rg -n "ThreadNotFound \{\\s*chat_id|Store(Read|Archive|Delete).*Params \{\\s*chat_id|AppendThreadItemsParams \{\\s*chat_id|CreateThreadParams \{\\s*chat_id|GoalSetRequest \{\\s*chat_id|Op::ChatSettings|Op::UserInput \{\\s*messages|SessionMeta \{[\\s\\S]*chat_source|NewThread \{\\s*chat_id|ThreadGoalUpdatedEvent.*chat_id" datax-rs/app-server/src datax-rs/app-server/tests/common -g '*.rs'` | repository root | `Completed` | No stale internal constructor patterns remain in the app-server files affected by the latest user-run compile failure. |

## Validation and Acceptance

The commands in this section are intentionally explicit so the user can run them during the staged migration validation pass. Commands marked deferred should not be run by Codex during this milestone unless the user explicitly asks.

From the repository root, run the whitespace check and expect no output:

    git diff --check

From the repository root, run the forbidden spelling check and expect no output:

    rg -n "Data[X]" .

From the repository root, verify old public method strings are gone from app-server source, tests, and README except documented deferrals:

    rg -n "thread/|turn/|item/" datax-rs/app-server-protocol/src datax-rs/app-server/src datax-rs/app-server/tests/suite/v2 datax-rs/app-server/README.md

From the repository root, verify old public v2 schema type names are gone from app-server source and v2 tests except documented internal adapters:

    rg -n "ThreadStart|ThreadRead|ThreadList|ThreadResume|ThreadFork|TurnStart|TurnInterrupt|ThreadItem|TurnStatus" datax-rs/app-server-protocol/src datax-rs/app-server/src datax-rs/app-server/tests/suite/v2

From the repository root, verify generated schema artifacts no longer expose old public id field names or old public error/parameter names:

    rg -n "threadId|turnId|itemId|numTurns|threadRollbackFailed|activeTurnNotSteerable|turnKind|NonSteerableTurnKind" datax-rs/app-server-protocol/schema/json datax-rs/app-server-protocol/schema/typescript

From `datax-rs`, run the formatter and expect it to complete:

    just fmt

From `datax-rs`, regenerate app-server schema artifacts and expect checked-in JSON and TypeScript outputs to reflect chat, interaction, and message naming:

    just write-app-server-schema

Current status: Codex attempted this command on 2026-07-07. The sandbox run failed because Cargo could not resolve `static.crates.io`; an escalated retry downloaded dependencies but ran long enough to destabilize WSL. The user then ran this command manually and pushed commit `dd5fd69a44` with generated schema artifacts. Codex pulled that commit and completed follow-up public schema naming cleanup in this branch.

From `datax-rs`, regenerate experimental app-server schema artifacts and expect experimental markers to use the renamed public methods:

    just write-app-server-schema --experimental

From `datax-rs`, run the targeted protocol tests and expect them to pass:

    just test -p datax-app-server-protocol

From `datax-rs`, run the targeted app-server tests and expect them to pass:

    just test -p datax-app-server

Acceptance for this milestone requires that Datax app-server v2 exposes chat, interaction, and message naming in Rust protocol types, JSON-RPC method strings, generated JSON schemas, generated TypeScript bindings, README examples, and v2 integration tests, with old public thread, turn, and item names either removed or documented as deferred internal exceptions.

## Idempotence and Recovery

Most edits in this milestone are renames and generated artifact updates. If a source rename goes wrong, use `git status --short` to identify moved and modified files, then inspect the specific diff before making corrective edits. Do not use broad destructive commands. If generated schema output is inconsistent, rerun the exact generation commands from `datax-rs` after source names are corrected.

Because public method aliases are intentionally not added, a temporary compile failure during the implementation is expected until protocol source, app-server dispatch, tests, and generated artifacts are updated together. Keep commits focused so a broken band can be isolated by file group.

## Artifacts and Notes

GitHub issue: https://github.com/mbellary/datax/issues/7

Draft pull request: https://github.com/mbellary/datax/pull/8

Initial discovery command:

    rg -n "thread/|turn/|item/|Thread|Turn|Item" datax-rs/app-server-protocol/src/protocol/v2 datax-rs/app-server/src/request_processors datax-rs/app-server/tests/suite/v2 datax-rs/app-server/README.md | cut -d: -f1 | sort -u

This command found the protocol v2 modules, request processors, v2 suite tests, and app-server README as the primary public API surface.

## Interfaces and Dependencies

At the end of this milestone, the app-server v2 protocol should define public Rust types such as `ChatStartParams`, `ChatStartResponse`, `InteractionStartParams`, `InteractionStartResponse`, `InteractionStatus`, and `Message` or more specific message notification names. The generated TypeScript bindings under `datax-rs/app-server-protocol/schema/typescript/v2/` should export matching type names.

App-server request dispatch should route method strings such as `chat/start`, `chat/read`, `chat/list`, `chat/resume`, `chat/fork`, `interaction/start`, `interaction/interrupt`, and any renamed message notifications or request surfaces defined by the protocol. Internal calls into `datax_core` and `datax_protocol` may keep engine-level thread and turn identifiers behind boundary conversion code.

## Change Notes

- 2026-07-06: Created the initial Phase 1.4 ExecPlan, file inventory, dependency order, validation matrix, and acceptance commands before implementation edits. This records the approved staged-test policy and keeps the protocol rename bounded to app-server public API surfaces.
- 2026-07-06: Created and recorded GitHub issue #7 and draft PR #8 so subsequent implementation commits are attached to the milestone.
