# Phase 1.5 Persistence, Fixtures, and Snapshots Rename

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` from the repository root. It also follows `docs/plans/Recommended-Datax-Migration-Execution-Model.md`, which requires a milestone branch, GitHub issue, draft pull request, file inventory, explicit validation commands, staged implementation, and current rename exception tracking.

## Purpose / Big Picture

Phase 1.5 finishes the local persistence side of the Datax migration. After this milestone, new Datax runs should resolve the default home directory, config-layer descriptions, state database override environment variable, project config folders, and owned fixture or snapshot text using Datax naming instead of Codex naming. This is still a migration-only phase: behavior should remain equivalent except for the source-of-truth names and paths that a fresh Datax install exposes.

The observable result is that the code no longer defaults to `~/.codex` or `CODEX_HOME` for Datax-owned persistence, project config discovery looks for `.datax/config.toml`, generated config/app-server schema descriptions mention Datax paths, and owned snapshots or fixtures that show those strings are updated. Protected sandbox identifiers remain unchanged.

## Progress

- [x] (2026-07-07 00:00Z) Created branch `datax/migration-phase1-5-persistence-fixtures` from `main` after Phase 1.4 was merged.
- [x] (2026-07-07 00:00Z) Read `PLANS.md`, `docs/plans/Recommended-Datax-Migration-Execution-Model.md`, and `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`.
- [x] (2026-07-07 00:00Z) Performed broad and focused searches for `.codex`, `CODEX_HOME`, `CODEX_SQLITE_HOME`, config persistence, fixture, and snapshot surfaces.
- [x] (2026-07-07 00:00Z) Identified the initial file inventory and dependency order before implementation edits.
- [x] (2026-07-07 00:00Z) Created GitHub issue #9 and draft PR #10 for this milestone.
- [x] (2026-07-07 00:00Z) Renamed the Datax-owned home, config, state, and project-folder persistence sources.
- [x] (2026-07-07 00:00Z) Updated generated config and app-server schema artifacts for Datax persistence descriptions without running generators.
- [x] (2026-07-07 00:00Z) Updated owned fixtures and snapshots that encode the renamed persistence strings, excluding `.codex-plugin` manifest paths.
- [x] (2026-07-07 00:00Z) Corrected accidental internal field/member rewrites reported by user build output, including `codex_linux_sandbox_exe` and `CodexThread.codex` member accesses.
- [x] (2026-07-07 00:00Z) Corrected additional accidental `Config.codex_self_exe` member rewrites reported by user build output.
- [x] (2026-07-07 00:00Z) Corrected user-reported `cargo build` failures in `datax-tui` where TUI app-server client code still referenced pre-Phase-1.4 protocol fields such as `thread_id`, `turn_id`, `item_id`, `turns`, and `items`.
- [x] (2026-07-07 00:00Z) Corrected remaining accidental `datax_linux_sandbox_exe` internal field rewrites in CLI, core, and exec-server test/debug call sites.
- [x] (2026-07-07 00:00Z) Ran allowed formatting and static checks after the `datax-tui` build-log follow-up; expensive tests/builds remain deferred to the user.
- [x] (2026-07-07 00:00Z) Corrected user-reported `cargo build` failures in `datax-exec` where app-server protocol call sites still used old thread/turn/item fields, and restored TUI-internal names that had been over-renamed during the previous protocol boundary pass.
- [x] (2026-07-07 00:00Z) Ran `just fmt`, `git diff --check`, and targeted stale-symbol scans after the `datax-exec` build-log follow-up; expensive tests/builds remain deferred to the user.
- [x] (2026-07-07 00:00Z) Updated validation ownership after user instruction: Codex will no longer run `just fmt`, build, check, test, or lint commands for this phase unless explicitly authorized for a specific command.
- [x] (2026-07-07 00:00Z) Corrected user-reported `cargo build` failures in `datax-app-server-test-client` where app-server test-client code still used old protocol boundary names and collided websocket `Message` with protocol `Message`.
- [x] (2026-07-07 00:00Z) Committed and pushed the latest app-server-test-client follow-up fixes.

## Surprises & Discoveries

- Observation: The codebase still has many internal `codex_home` variable and function names. These are implementation identifiers, not necessarily user-visible persistence names.
  Evidence: Focused `rg` output shows hundreds of `codex_home` variable references across config, app-server, thread-store, and state tests. Renaming all of them would exceed the Phase 1.5 persistence boundary and risk churn unrelated to behavior.
- Observation: The canonical default home resolver still uses `CODEX_HOME` and `~/.codex`.
  Evidence: `codex-rs/utils/home-dir/src/lib.rs` reads `std::env::var("CODEX_HOME")` and appends `.codex` when the env var is absent.
- Observation: Generated schema descriptions still mention `$CODEX_HOME` and `.codex`.
  Evidence: `codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json` and related generated schema files contain descriptions for `$CODEX_HOME/config.toml` and `.codex/` project folders.
- Observation: Some `.snap.new` files already exist under TUI snapshot folders before this milestone starts.
  Evidence: `find codex-rs -path '*snapshots*' -type f -name '*.snap'` output included sibling `.snap.new` paths in the working tree scan. This milestone will not accept or delete pre-existing pending snapshots unless a changed source requires it.
- Observation: The default project metadata carveout is declared in `codex-rs/protocol/src/permissions.rs`, and linux sandbox only consumes the resulting filesystem policy.
  Evidence: `rg -n "PROTECTED_METADATA_CODEX_PATH_NAME|append_default_read_only_project_root_subpath_if_no_explicit_rule" codex-rs/protocol/src/permissions.rs` shows `.codex` being added to the default protected project-root subpaths.
- Observation: The broad fixture/snapshot pass must not rewrite Rust member access or Cargo metadata just because it contains a dotted `codex` segment.
  Evidence: Static checks found and corrected accidental internal member rewrites such as `codex_error_info`, `codex_home`, and the `datax-protocol` dependency alias before formatting.
- Observation: User build output later exposed additional accidental internal member rewrites outside the first static-check set.
  Evidence: `cargo build` reported missing `datax_linux_sandbox_exe` and missing `.datax` fields on `CodexThread` and `GuardianReviewSession`; those were restored to existing internal field names while retaining Datax filesystem path strings.
- Observation: User build output exposed the same issue for the executable path field on `Config`.
  Evidence: `cargo build` reported missing `datax_self_exe`; the app-server, CLI, and exec-server references were restored to `codex_self_exe`.
- Observation: Once `cargo build` reached the TUI crate, it exposed app-server protocol API names that had not been updated with the Phase 1.4 chat/interaction/message terminology.
  Evidence: The attached build log reported missing fields and variants in `datax-tui`, including `thread_id`, `turn_id`, `item_id`, `turns`, `items`, `ThreadSettings`, `ThreadStartSource`, and `ChatRealtimeItemAdded`; the TUI app-server client surfaces were aligned to `chat_id`, `interaction_id`, `message_id`, `interactions`, `messages`, `ChatSettings`, `ChatStartSource`, and `ChatRealtimeMessageAdded` while preserving TUI-internal `thread_id` and `turn_id` names.
- Observation: Once `cargo build` reached `datax-exec`, it exposed another app-server protocol boundary that still used pre-Phase-1.4 field names.
  Evidence: The attached build log reported missing `thread_id`, `turn_id`, `items`, `turns`, `thread_source`, and `parent_thread_id` fields in `exec`; the exec app-server request/notification code was aligned to `chat_id`, `interaction_id`, `messages`, `interactions`, `chat_source`, and `parent_chat_id`, while JSONL output structs and `SessionConfiguredEvent` kept their existing internal thread/turn terminology.
- Observation: The previous TUI protocol pass accidentally renamed a few local TUI struct fields and local function parameters that were not app-server protocol fields.
  Evidence: The attached build log reported missing local variables and fields such as `turn_id`, `item_id`, `AppExitInfo.thread_id`, `PlanModeNudgeScope::Chat`, and `resume_picker::Row.thread_id`; these were restored as TUI-internal names.
- Observation: Once `cargo build` reached `datax-app-server-test-client`, it exposed another client-facing app-server protocol surface that still used pre-Phase-1.4 names and had an import-name collision.
  Evidence: The attached build log reported missing `thread_id`, `turn_id`, `item_id`, and `parent_thread_id` fields, missing `thread_start` and `turn_start` methods, and a duplicate `Message` import; the app-server test client now constructs/destructures current chat/interaction/message protocol fields and aliases `tungstenite::Message` as `WebSocketMessage`.

## Decision Log

- Decision: Rename user-visible and source-of-truth persistence names in this milestone, but leave broad internal `codex_home` Rust variable names in place unless touching a local line for clarity or test expectation.
  Rationale: The plan requires path/config defaults to move to Datax. A mechanical variable rename across every caller would be large, risky, and not needed to change behavior.
  Date/Author: 2026-07-07 / Codex.
- Decision: Treat `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` as protected exceptions and do not modify them.
  Rationale: Repository instructions explicitly protect these sandbox identifiers because they are tied to sandbox behavior.
  Date/Author: 2026-07-07 / Codex.
- Decision: Defer test/build execution to the user, but keep every command explicit in this plan.
  Rationale: The user requested that tests not be executed by Codex during migration phases because they are expensive and can destabilize WSL.
  Date/Author: 2026-07-07 / Codex.
- Decision: Also defer `just fmt` execution to the user for subsequent follow-up fixes in this phase.
  Rationale: The user explicitly requested that Codex stop running `just fmt`; Codex will document the command and make targeted source edits only.
  Date/Author: 2026-07-07 / Codex.

## Outcomes & Retrospective

Implementation updated the Datax-owned persistence defaults and fixtures so fresh runs use `DATAX_HOME`, `DATAX_SQLITE_HOME`, `~/.datax`, `.datax`, and `/etc/datax` for owned local state, config, and project metadata. The pass also updated generated schema descriptions, app-server examples, CLI/test fixtures, sandbox carveout expectations, and TUI snapshots that render those persistence paths.

The `.codex-plugin` manifest directory remains unchanged as a plugin format exception. Internal Rust identifiers such as `codex_home` also remain when they are implementation API names rather than persisted path strings. During the broad pass, static scans caught and corrected accidental internal rewrites before formatting.

Expensive generation, build, lint, and test commands remain deferred to the user per migration instructions. The exact commands are listed in the validation sections below.

## Baseline

The milestone starts from `main` at merge commit `eef144315e Merge pull request #8 from mbellary/datax/migration-phase1-4-app-server-protocol`. Phase 1.4 app-server protocol tests passed according to the user before this phase began. The starting branch for this milestone is `datax/migration-phase1-5-persistence-fixtures`.

Known constraint: Codex must not run expensive build, check, lint, or test commands in this phase. After the 2026-07-07 user instruction, Codex must also not run `just fmt` unless the user explicitly authorizes that exact command. Codex may run `git diff --check` and static `rg` checks. The user will run the documented format, build, lint, and test commands and report results.

## Context and Orientation

The repository root is `/home/mbellary/wsl/projects/datax`. Rust code lives under `codex-rs`. Despite the directory name, crate packages have already been migrated to `datax-*` in earlier phases.

Persistence in this plan means local files and directories that Datax creates or reads for user configuration, state databases, logs, auth material, rollouts, and project-local configuration. The most important source files are:

- `codex-rs/utils/home-dir/src/lib.rs`, which resolves the default Datax home directory.
- `codex-rs/config/src/config_toml.rs`, `codex-rs/config/src/types.rs`, `codex-rs/config/src/loader/mod.rs`, and `codex-rs/config/src/state.rs`, which describe and load config layers.
- `codex-rs/state/src/lib.rs`, which declares the SQLite override environment variable.
- `codex-rs/linux-sandbox/src/bwrap.rs` and `codex-rs/linux-sandbox/tests/suite/landlock.rs`, which protect hidden project-owned folders from sandbox writes.
- Generated schema files under `codex-rs/core/config.schema.json` and `codex-rs/app-server-protocol/schema/json/`, which expose config descriptions to clients.

Project config folder means a repository-local folder containing `config.toml`. Before the migration, the folder is `.codex`. For Datax, owned project config should use `.datax`.

## Rename Exception Register

The following references may remain after this milestone:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` in Rust code and tests. These are protected sandbox identifiers.
- Internal Rust variable or helper names such as `codex_home` and `find_codex_home` when the runtime behavior or user-visible text is already Datax-owned. These will be recorded as implementation-debt exceptions and can be mechanically cleaned up in a later targeted phase if desired.
- External upstream service paths such as ChatGPT backend `/codex/` URLs, OpenAI-hosted entitlement strings, model names such as `gpt-5-codex`, and provenance links to `openai/codex`.
- Crate, directory, or snapshot filenames that still contain historical module names and are not user-visible persistence behavior in this milestone.
- Protected or third-party examples under vendor, plugin samples, or JSON schema fixtures where `codex` is input data for an external service rather than Datax product identity.

## Public Surface Checklist

This milestone touches config keys and paths, persisted state paths, generated schemas, fixtures, and snapshots. It should not change CLI arguments, app-server method names, package names, crate names, or app-server model types unless a generated description must be refreshed after source comments change.

Specific public surfaces to review:

- Default home directory: `DATAX_HOME` and `~/.datax`.
- SQLite override environment variable: `DATAX_SQLITE_HOME`.
- Project config folder: `.datax/config.toml`.
- Generated config schema descriptions.
- App-server config read/write schema descriptions that mention user and project config layers.
- Snapshot or fixture text that renders these paths to users.

## Dependency Order

First update the source-of-truth constants and comments for Datax-owned persistence. This includes home directory resolution, SQLite override env names, config-layer descriptions, and project config folder discovery.

Second update callers and tests whose expectations are directly tied to those source names. This includes linux sandbox hidden-folder protection and config loader tests for project folders.

Third regenerate schema artifacts only after the source comments and schema inputs are stable. Config schema changes require `just write-config-schema`; app-server protocol schema descriptions require `just write-app-server-schema`.

Fourth update snapshots and fixtures that are owned by this phase. If a fixture is historical upstream input, leave it unchanged and document it as an exception.

Finally run formatting and static checks. Test/build commands remain deferred for the user.

## File Inventory

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/persistence_fixtures_rename_execplan.md` | `Completed` | Living ExecPlan for this milestone. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/github_issue.md` | `Completed` | Records milestone issue #9. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/pull_request.md` | `Completed` | Records draft PR #10. |
| `codex-rs/utils/home-dir/src/lib.rs` | `Completed` | Source of default home env var and fallback directory; should become `DATAX_HOME` and `.datax` while keeping the existing public helper name unless required. |
| `codex-rs/state/src/lib.rs` | `Completed` | Source of SQLite override env var; should become `DATAX_SQLITE_HOME`. |
| `codex-rs/config/src/config_toml.rs` | `Completed` | Config schema comments mention `~/.codex`, `$CODEX_HOME`, and `.codex`; source descriptions drive generated schema. |
| `codex-rs/config/src/types.rs` | `Completed` | Config type comments mention `CODEX_HOME`; source descriptions drive generated schema. |
| `codex-rs/config/src/loader/mod.rs` | `Completed` | Config loader docs and project folder discovery mention `/etc/codex`, `$CODEX_HOME`, and `.codex`. |
| `codex-rs/config/src/loader/layer_io.rs` | `Completed` | Managed config default path still uses `/etc/codex/managed_config.toml`. |
| `codex-rs/config/src/state.rs` | `Completed` | Config layer comments expose `.codex` and `$CODEX_HOME` descriptions. |
| `codex-rs/config/src/loader/README.md` | `Completed` | Build-adjacent developer README documents config layer order and project folder names. |
| `codex-rs/config/src/cloud_config_layers_tests.rs` | `Completed` | Test expectations include `/home/alice/.codex/config.toml`. |
| `codex-rs/config/src/loader/tests.rs` | `Completed` | Loader tests may encode project config folder behavior. |
| `codex-rs/config/src/config_requirements.rs` | `Completed` | Requirement tests include `com.openai.codex` and `com.codex`; inspect whether these are managed-domain examples or product-owned defaults. |
| `codex-rs/core/src/config/config_tests.rs` | `Completed` | Core config tests likely assert home and config layer behavior. |
| `codex-rs/core/src/config/config_loader_tests.rs` | `Completed` | Core config loader tests likely assert project `.codex` discovery. |
| `codex-rs/core/src/config/permissions_tests.rs` | `Completed` | Test fixture creates a `.codex` home path; inspect whether this is just a temporary directory or user-visible expectation. |
| `codex-rs/core/config.schema.json` | `Completed` | Generated config schema must be refreshed if config comments change. |
| `codex-rs/app-server-protocol/schema/json/v1/InitializeResponse.json` | `Completed` | Generated schema currently describes `$CODEX_HOME`; source may need regeneration. |
| `codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json` | `Completed` | Generated schema currently describes `$CODEX_HOME` and `.codex`. |
| `codex-rs/app-server-protocol/schema/json/v2/ConfigWriteResponse.json` | `Completed` | Generated schema currently describes `$CODEX_HOME` and `.codex`. |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json` | `Completed` | Aggregate generated schema currently contains persistence descriptions. |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json` | `Completed` | Aggregate generated v2 schema currently contains persistence descriptions. |
| `codex-rs/app-server-protocol/schema/typescript/InitializeResponse.ts` | `Completed` | Generated TypeScript docs may contain `$CODEX_HOME`. |
| `codex-rs/protocol/src/config_types.rs` | `Completed` | Profile-v2 comments mention `$CODEX_HOME`; source descriptions may flow into schemas. |
| `codex-rs/protocol/src/protocol.rs` | `Completed` | Sandbox policy docs and tests mention `.codex`; inspect for Datax-owned project metadata protection. |
| `codex-rs/protocol/src/permissions.rs` | `Completed` | Source of default protected project metadata path; should protect `.datax` instead of `.codex` for Datax project config. |
| `codex-rs/linux-sandbox/src/bwrap.rs` | `Completed` | Sandbox hidden-project folder protection currently includes `.codex`; Datax project folder should be protected. |
| `codex-rs/linux-sandbox/tests/suite/landlock.rs` | `Completed` | Landlock tests create and assert `.codex` protection. |
| `codex-rs/linux-sandbox/README.md` | `Completed` | README documents `.codex` protection and may need `.datax`. |
| `codex-rs/linux-sandbox/src/proxy_routing.rs` | `Completed` | Reads `CODEX_HOME` for temp proxy path; should follow the Datax home env if this is product-owned runtime behavior. |
| `codex-rs/network-proxy/src/certs.rs` | `Completed` | Error text says `CODEX_HOME`; inspect and update if using Datax home resolver. |
| `codex-rs/network-proxy/src/socks5.rs` | `Completed` | Comment mentions shared test `CODEX_HOME`; inspect for user-visible or source-of-truth impact. |
| `codex-rs/app-server-transport/src/transport/mod.rs` | `Completed` | Error text says failed to resolve `CODEX_HOME`; update if home resolver moves to `DATAX_HOME`. |
| `codex-rs/thread-store/src/local/update_thread_metadata.rs` | `Completed` | Contains `https://github.com/openai/codex` origin URL test data; inspect whether provenance exception or Datax-owned fixture. |
| `codex-rs/analytics/src/analytics_client_tests.rs` | `Completed` | Tests include `.codex/skills` paths; inspect whether path anonymization expectations should become `.datax`. |
| `codex-rs/tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__hooks_browser_view__tests__hooks_browser_events.snap` | `Completed` | Snapshot text includes "Codex ends its turn"; likely owned UI snapshot rename. |
| `codex-rs/tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__hooks_browser_view__tests__hooks_browser_events_with_issues.snap` | `Completed` | Snapshot text includes "Codex ends its turn"; likely owned UI snapshot rename. |
| `codex-rs/tui/src/snapshots/codex_tui__model_migration__tests__model_migration_prompt.snap` | `Completed` | Snapshot contains model migration product text; inspect whether belongs to fixture/snapshot rename. |
| `codex-rs/tui/src/app_server_session.rs` | `Completed` | Follow-up from user `cargo build`; app-server request builders now use current chat/interaction/message protocol fields. |
| `codex-rs/tui/src/app/app_server_event_targets.rs` | `Completed` | Follow-up from user `cargo build`; notification/request routing now reads current chat-scoped protocol fields. |
| `codex-rs/tui/src/app/app_server_requests.rs` | `Completed` | Follow-up from user `cargo build`; pending app-server request tracking now reads current message ids while preserving internal item ids. |
| `codex-rs/tui/src/app/background_requests.rs` | `Completed` | Follow-up from user `cargo build`; feedback, MCP inventory, and apps-list requests now use current app-server protocol fields. |
| `codex-rs/tui/src/app/thread_events.rs` | `Completed` | Follow-up from user `cargo build`; thread event replay reads current interaction/message fields. |
| `codex-rs/tui/src/app/thread_settings.rs` | `Completed` | Follow-up from user `cargo build`; settings update params now use `chat_id`. |
| `codex-rs/tui/src/chatwidget.rs` | `Completed` | Follow-up from user `cargo build`; approval conversion helpers map protocol ids into existing TUI event shapes. |
| `codex-rs/tui/src/chatwidget/protocol.rs` | `Completed` | Follow-up from user `cargo build`; notification handling now uses current protocol field names and realtime notification variant. |
| `codex-rs/tui/src/chatwidget/replay.rs` | `Completed` | Follow-up from user `cargo build`; replay now consumes `Interaction.messages` and emits `InteractionCompletedNotification.chat_id`. |
| `codex-rs/tui/src/chatwidget/tool_requests.rs` | `Completed` | Follow-up from user `cargo build`; app-server request params now read `chat_id`, `interaction_id`, and `message_id` while TUI approval requests keep internal field names. |
| `codex-rs/tui/src/resume_picker.rs` | `Completed` | Follow-up from user `cargo build`; transcript preview reads `Chat.interactions`. |
| `codex-rs/tui/src/thread_transcript.rs` | `Completed` | Follow-up from user `cargo build`; transcript rendering reads `Chat.interactions` and `Interaction.messages`. |
| `codex-rs/exec/src/lib.rs` | `Completed` | Follow-up from user `cargo build`; in-process app-server requests and notifications now use current chat/interaction/message protocol fields while preserving exec-internal thread/turn output terms. |
| `codex-rs/exec/src/event_processor_with_human_output.rs` | `Completed` | Follow-up from user `cargo build`; final-message recovery now reads `Interaction.messages`. |
| `codex-rs/exec/src/event_processor_with_jsonl_output.rs` | `Completed` | Follow-up from user `cargo build`; JSONL processor now imports the correct local `ThreadItem` type and reads `Interaction.messages`. |
| `codex-rs/exec/src/lib_tests.rs` | `Completed` | Follow-up fixture alignment for current app-server protocol field names. |
| `codex-rs/exec/src/event_processor_with_human_output_tests.rs` | `Completed` | Follow-up fixture alignment for current app-server protocol field names while preserving `SessionConfiguredEvent` internal fields. |
| `codex-rs/exec/src/event_processor_with_jsonl_output_tests.rs` | `Completed` | Follow-up fixture alignment for current app-server protocol field names. |
| `codex-rs/app-server-test-client/src/lib.rs` | `Completed` | Follow-up from user `cargo build`; app-server test-client request builders, approval handlers, and websocket transport now use current protocol boundary names and avoid the `Message` import collision. |
| `codex-rs/app-server-test-client/src/plugin_analytics_smoke.rs` | `Completed` | Follow-up from user `cargo build`; plugin analytics smoke helper now calls current `chat_start` and `interaction_start` helpers with `chat_id`. |
| `codex-rs/app-server-test-client/src/request_user_input.rs` | `Completed` | Follow-up from user `cargo build`; request-user-input prompt reads current `chat_id`, `interaction_id`, and `message_id` fields. |
| `codex-rs/app-server-test-client/src/request_user_input_tests.rs` | `Completed` | Follow-up test fixture alignment for current request-user-input protocol field names. |
| `codex-rs/cli/src/debug_sandbox.rs` | `Completed` | Corrected accidental internal `datax_linux_sandbox_exe` field rewrite back to `codex_linux_sandbox_exe`. |
| `codex-rs/core/tests/common/lib.rs` | `Completed` | Corrected accidental internal `datax_linux_sandbox_exe` field rewrite back to `codex_linux_sandbox_exe`. |
| `codex-rs/core/tests/suite/apply_patch_cli.rs` | `Completed` | Corrected accidental internal `datax_linux_sandbox_exe` field rewrite back to `codex_linux_sandbox_exe`. |
| `codex-rs/exec-server/tests/common/mod.rs` | `Completed` | Corrected accidental internal `datax_linux_sandbox_exe` field rewrite back to `codex_linux_sandbox_exe`. |
| `codex-rs/tui/tests/fixtures/oss-story.jsonl` | `Not Required` | Large recorded story fixture contains incidental words such as "returned" and `codex_event`; this is not Datax persistence behavior for Phase 1.5. |
| `codex-rs/tools/tests/fixtures/json_schema_policy/*.json` | `Not Required` | Third-party tool schemas use generic JSON Schema words such as `items` or vendor fields like `thread_ts`; not Datax product persistence. |
| `codex-rs/vendor/bubblewrap/**` | `Not Required` | Third-party vendor files; not Datax product-owned text. |

The inventory will be updated as implementation discovers additional exact test, fixture, generated, or snapshot files.

## Plan of Work

Update `codex-rs/utils/home-dir/src/lib.rs` so the Datax home resolver honors `DATAX_HOME` and falls back to `~/.datax`. Keep the public function name initially, because it is a Rust API used widely and changing it would be broad internal churn outside the behavior needed for this milestone. Update error messages and tests to assert `DATAX_HOME`.

Update `codex-rs/state/src/lib.rs` so `SQLITE_HOME_ENV` is `DATAX_SQLITE_HOME`. Then update documentation and generated schema descriptions that mention the old SQLite override.

Update config loader source comments and project folder discovery from `.codex` to `.datax`, including any tests that create project-local config folders. Update system paths from `/etc/codex` to `/etc/datax` where they are Datax-owned defaults. For macOS or Windows managed preference identifiers, inspect first: if they are compatibility IDs or MDM domains that must remain temporarily, record them in the exception register.

Update sandbox path protection to protect `.datax` in project roots. The default filesystem policy is declared in `codex-rs/protocol/src/permissions.rs`; linux sandbox tests should then match the generated policy behavior. If `.codex` protection is still needed as a compatibility hardening exception for old checkouts, keep it only with a clear comment; otherwise replace the Datax-owned hidden folder.

Regenerate `codex-rs/core/config.schema.json` with `just write-config-schema` if config schema comments changed. Regenerate app-server schema artifacts with `just write-app-server-schema` if app-server protocol schemas still contain old persistence descriptions after source changes.

Update owned snapshots and fixtures that render Datax-owned persistence strings. Do not update third-party fixtures, upstream provenance links, model names, or protected sandbox identifiers.

## Concrete Steps

From the repository root, confirm the branch:

    git status --short --branch

Expected result:

    ## datax/migration-phase1-5-persistence-fixtures

From the repository root, inspect persistence names:

    rg -n 'CODEX_HOME|CODEX_SQLITE_HOME|DATAX_HOME|DATAX_SQLITE_HOME|\\.codex|\\.datax' codex-rs/utils/home-dir codex-rs/config/src codex-rs/core/src/config codex-rs/state/src codex-rs/linux-sandbox/src codex-rs/app-server-transport/src codex-rs/network-proxy/src

Expected result after implementation: only documented exceptions remain for `CODEX_HOME`, `CODEX_SQLITE_HOME`, or `.codex` in these source-of-truth paths.

From `codex-rs`, regenerate config schema if config comments change:

    just write-config-schema

Expected result: `codex-rs/core/config.schema.json` is updated or unchanged consistently with source comments.

From `codex-rs`, regenerate app-server schema artifacts if generated app-server descriptions change:

    just write-app-server-schema

Expected result: generated app-server schema JSON and TypeScript files are updated or unchanged consistently with source comments.

From `codex-rs`, format:

    just fmt

Expected result: formatting completes successfully.

## Validation Matrix

| Command | Working Directory | Status | Expected Result |
| --- | --- | --- | --- |
| `git diff --check` | repository root | Completed | No whitespace errors. |
| `rg -n 'Data[X]' docs codex-rs --glob '!vendor/**'` | repository root | Completed | No forbidden mixed-case product spelling. |
| Literal persistence string scan documented below | repository root | Completed | Only `.codex-plugin` manifest format exceptions remain. |
| `just write-config-schema` | `codex-rs` | Deferred | Config schema regenerated if source descriptions changed. |
| `just write-app-server-schema` | `codex-rs` | Deferred | App-server schema artifacts regenerated if generated descriptions changed. |
| `just fmt` | `codex-rs` | Deferred to user for latest follow-up | Formatting completes successfully. |
| `cargo build` | `codex-rs` | Deferred | Workspace build reaches completion after the user-reported follow-up fixes. |
| `just fix -p datax-utils-home-dir` | `codex-rs` | Deferred | Lints for home-dir changes pass or are fixed. |
| `just fix -p datax-config` | `codex-rs` | Deferred | Lints for config changes pass or are fixed. |
| `just fix -p datax-state` | `codex-rs` | Deferred | Lints for state changes pass or are fixed. |
| `just fix -p datax-linux-sandbox` | `codex-rs` | Deferred | Lints for linux sandbox changes pass or are fixed. |
| `just test -p datax-utils-home-dir` | `codex-rs` | Deferred | Home-dir tests pass. |
| `just test -p datax-config` | `codex-rs` | Deferred | Config tests pass. |
| `just test -p datax-core` | `codex-rs` | Deferred | Core tests pass. |
| `just test -p datax-state` | `codex-rs` | Deferred | State tests pass. |
| `just test -p datax-linux-sandbox` | `codex-rs` | Deferred | Linux sandbox tests pass. |
| `just test -p datax-app-server-protocol` | `codex-rs` | Deferred | Protocol schema fixture tests pass if generated artifacts changed. |
| `just test -p datax-tui` | `codex-rs` | Deferred | TUI snapshot tests pass if snapshots changed. |

## Validation and Acceptance

From the repository root, run the whitespace check and expect no output:

    git diff --check

From the repository root, run the forbidden spelling check for this phase plan and expect no output:

    rg -n 'Data[X]' docs/plans/datax_migration_phase1_5_persistence_fixtures

From the repository root, run the literal persistence-name source check and expect only `.codex-plugin` manifest format exceptions:

    rg -n '"CODEX_HOME"|`CODEX_HOME`|CODEX_SQLITE_HOME|"/etc/codex|`/etc/codex|~/\.codex|"\\.codex"|"\\.codex/|/\\.codex' codex-rs --glob '!vendor/**'

From `codex-rs`, regenerate the config schema when config comments or config types changed:

    just write-config-schema

From `codex-rs`, regenerate app-server schema artifacts when app-server generated descriptions changed:

    just write-app-server-schema

From `codex-rs`, run the formatter and expect it to complete:

    just fmt

From `codex-rs`, run the workspace build and expect it to complete:

    cargo build

From `codex-rs`, run lints for changed crates and expect them to pass:

    just fix -p datax-utils-home-dir
    just fix -p datax-config
    just fix -p datax-state
    just fix -p datax-linux-sandbox

From `codex-rs`, run targeted tests and expect them to pass:

    just test -p datax-utils-home-dir
    just test -p datax-config
    just test -p datax-core
    just test -p datax-state
    just test -p datax-linux-sandbox
    just test -p datax-app-server-protocol
    just test -p datax-tui

Codex will not run the deferred test/build/lint commands unless the user explicitly asks. The user will run the commands and report failures for follow-up fixes.

## Idempotence and Recovery

The source edits are ordinary text changes and can be retried safely. Schema generation commands are idempotent: rerunning them should either leave generated files unchanged or update them to match current source comments and types.

If generated artifacts drift unexpectedly, inspect the source comments and type changes first, then rerun the generator from `codex-rs`. If a rename causes a broad compile failure, keep the implementation boundary narrow: fix direct persistence rename fallout and document unrelated internal `codex_home` cleanup as deferred.

Rollback is a normal branch rollback: revert the milestone commits or reset the branch before merge. Generated schema files and snapshots must be reverted together with the source changes that caused them.

## Artifacts and Notes

GitHub issue: https://github.com/mbellary/datax/issues/9

Draft pull request: https://github.com/mbellary/datax/pull/10

Initial searches used:

    rg -n 'CODEX_HOME|CODEX_SQLITE_HOME|DATAX_HOME|DATAX_SQLITE_HOME|\.codex|\.datax' codex-rs/utils/home-dir codex-rs/config/src codex-rs/core/src/config codex-rs/state/src codex-rs/linux-sandbox/src codex-rs/app-server-transport/src codex-rs/network-proxy/src

Important initial evidence:

    codex-rs/utils/home-dir/src/lib.rs reads CODEX_HOME and defaults to .codex
    codex-rs/state/src/lib.rs defines SQLITE_HOME_ENV as CODEX_SQLITE_HOME
    codex-rs/config/src/loader/mod.rs documents project .codex/config.toml layers
    codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json describes $CODEX_HOME and .codex

## Interfaces and Dependencies

The existing `datax_utils_home_dir::find_codex_home` function remains the entry point unless this plan is later expanded to mechanically rename the Rust API. Its behavior at the end of this milestone must be:

    pub fn find_codex_home() -> std::io::Result<AbsolutePathBuf>

It should read `DATAX_HOME` when present and non-empty, validate that the path exists and is a directory, canonicalize it, and otherwise return the user's home directory joined with `.datax`.

The existing `datax_state::SQLITE_HOME_ENV` constant remains the entry point for SQLite home override lookup. Its value at the end of this milestone must be:

    pub const SQLITE_HOME_ENV: &str = "DATAX_SQLITE_HOME";

The config loader should treat `.datax/config.toml` as the project-local config folder. Generated schema descriptions should reflect Datax persistence names after regeneration.

## Change Notes

2026-07-07: Created the initial ExecPlan, inventory, dependency order, validation matrix, and acceptance commands before implementation edits. This records the scope and prevents a blind mechanical rewrite.

2026-07-07: Added GitHub issue #9 and draft PR #10 links after creating the milestone tracking artifacts. This keeps the plan, branch, issue, and PR aligned before implementation starts.

2026-07-07: Added `codex-rs/protocol/src/permissions.rs`, `codex-rs/protocol/src/protocol.rs`, and `codex-rs/protocol/src/config_types.rs` to the inventory after discovering the sandbox policy source and protocol comments that feed persistence behavior and generated descriptions.

## Expanded File Inventory

Generated from the current branch change set after implementation. The primary inventory above records the source groups; this appendix records every currently changed file for focused tracking.

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `codex-rs/analytics/src/analytics_client_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-client/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-daemon/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-daemon/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-daemon/src/remote_control_client.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/ClientRequest.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/v1/InitializeResponse.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/v2/ConfigWriteResponse.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/json/v2/LoginAccountParams.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/schema/typescript/InitializeResponse.ts` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/src/protocol/common.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/src/protocol/v1.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/src/protocol/v2/config.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-protocol/src/protocol/v2/tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-test-client/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server-transport/src/transport/mod.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/src/config/external_agent_config.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/src/config/external_agent_config_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/src/config_manager_service.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/src/request_processors/feedback_doctor_report.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/common/rollout.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/common/test_app_server.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/strict_config.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/chat_resume.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/chat_start.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/command_exec.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/config_rpc.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/connection_handling_websocket.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/experimental_feature_list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/external_agent_config.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/hooks_list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/mcp_server_status.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/permission_profile_list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/plugin_list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/recommended_plugins.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/remote_control.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/app-server/tests/suite/v2/skills_list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/arg0/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/debug_sandbox.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/doctor.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/doctor/output/detail.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/doctor/thread_inventory.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/main.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/marketplace_cmd.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/mcp_cmd.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/plugin_cmd.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/src/sandbox_setup.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/app_server.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/debug_clear_memories.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/debug_models.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/delete.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/exec_server.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/execpolicy.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/features.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/login.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/marketplace_add.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/marketplace_remove.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/marketplace_upgrade.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/mcp_add_remove.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/mcp_list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/plugin_cli.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/sandbox_network_proxy.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/cli/tests/update.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/cloud_config_layers_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/config_requirements.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/config_toml.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/loader/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/loader/layer_io.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/loader/macos.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/loader/mod.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/requirements_layers/stack_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/state.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/strict_config_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/tui_keymap.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/config/src/types.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/manager.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/manager_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/marketplace_upgrade/activation.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/remote/remote_installed_plugin_sync.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/remote_bundle.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/store.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-plugins/src/store_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-skills/src/loader.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-skills/src/loader_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-skills/src/render.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core-skills/src/service_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/config.schema.json` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/agent/control.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/agent/control/residency.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/agent/control/residency_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/agent/control/spawn.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/agent/control_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/agents_md_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/codex_thread.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/config/config_loader_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/config/config_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/config/mod.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/config/permissions_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/config/schema.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/exec_policy_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/exec_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/guardian/review_session.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/mcp_tool_call_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/network_proxy_loader.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/safety_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/session/tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/thread_manager_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/tools/handlers/multi_agents_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/src/tools/sandboxing.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/common/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/common/test_codex_exec.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/abort_tasks.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/apply_patch_cli.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/cli_stream.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/client.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/client_websockets.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/collaboration_instructions.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/compact.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/compact_remote.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/compact_remote_parity.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/live_cli.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/model_visible_layout.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/pending_input.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/permissions_messages.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/request_compression.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/resume.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/review.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/rmcp_client.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/rollout_list_find.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/shell_snapshot.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/subagent_notifications.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/truncation.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/user_shell_cmd.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/core/tests/suite/windows_sandbox.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec-server/src/environment.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec-server/src/fs_sandbox.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec-server/testing/wine_exec_server.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec-server/tests/common/exec_server.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec-server/tests/common/mod.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec/src/cli.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/exec/tests/suite/sandbox.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/ext/skills/tests/skills_extension.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/external-agent-migration/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/hooks/src/engine/discovery.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/hooks/src/engine/mod_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/install-context/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/linux-sandbox/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/linux-sandbox/src/bwrap.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/linux-sandbox/src/proxy_routing.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/linux-sandbox/tests/suite/landlock.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/login/src/assets/success.html` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/login/src/auth/storage.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/login/src/auth/storage_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/mcp-server/src/codex_tool_config.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/mcp-server/tests/common/mcp_process.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/memories/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/memories/write/src/startup_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/message-history/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/model-provider-info/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/network-proxy/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/network-proxy/src/certs.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/network-proxy/src/socks5.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/protocol/src/config_types.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/protocol/src/permissions.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/protocol/src/protocol.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/responses-api-proxy/README.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rmcp-client/src/oauth.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rmcp-client/tests/streamable_http_oauth_startup.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rmcp-client/tests/streamable_http_test_support.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rollout-trace/src/reducer/code_cell.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rollout-trace/src/reducer/tool.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rollout/src/list.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/rollout/src/recorder.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/sandboxing/src/seatbelt.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/sandboxing/src/seatbelt_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/imagegen/SKILL.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/imagegen/references/cli.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/imagegen/references/codex-network.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/imagegen/references/image-api.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/imagegen/references/prompting.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/imagegen/references/sample-prompts.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/openai-docs/SKILL.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/skill-creator/SKILL.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/skill-installer/SKILL.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/skill-installer/scripts/install-skill-from-github.py` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/assets/samples/skill-installer/scripts/list-skills.py` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/skills/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/state/src/bin/logs_client.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/state/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/test-binary-support/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/app/startup_prompts.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/app/tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/bottom_pane/list_selection_view.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/chatwidget/tests/history_replay.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/debug_config.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/external_agent_config_migration.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/goal_files.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/keymap.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/markdown_render_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/pets/ambient.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/pets/asset_pack.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/pets/mod.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/pets/model.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/render/highlight.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__debug_config__tests__debug_config_effective_sandbox_modes_with_deny_read.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__debug_config__tests__debug_config_requirement_sources.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__external_agent_config_migration__tests__external_agent_config_migration_customize.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__external_agent_config_migration__tests__external_agent_config_migration_customize_action.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__external_agent_config_migration__tests__external_agent_config_migration_customize_action_windows.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__external_agent_config_migration__tests__external_agent_config_migration_customize_windows.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/snapshots/codex_tui__markdown_render__markdown_render_tests__table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot.snap` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/src/theme_picker.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/tests/suite/resize_reflow.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/tui/tooltips.txt` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/utils/cli/src/config_override.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/utils/cli/src/shared_options.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/utils/home-dir/src/lib.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/utils/sandbox-summary/src/sandbox_summary.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/sandbox_smoketests.py` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/allow.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/bin/command_runner/win/cwd_junction.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/bin/setup_main/win.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/cap.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/helper_materialization.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/identity.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/setup.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/spawn_prep.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/workspace_acl.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/wrapper.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `codex-rs/windows-sandbox-rs/src/wrapper_tests.rs` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/persistence_fixtures_rename_execplan.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/github_issue.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/pull_request.md` | `Completed` | Phase 1.5 persistence, fixture, schema, or snapshot update. |
