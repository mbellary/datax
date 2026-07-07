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
- [ ] Create the GitHub issue and draft pull request for this milestone.
- [ ] Rename the Datax-owned home, config, state, and project-folder persistence sources.
- [ ] Update generated config and app-server schema artifacts if source descriptions change.
- [ ] Update owned fixtures and snapshots that encode the renamed persistence strings.
- [ ] Run allowed formatting/static checks, document deferred test/build commands, commit, and push.

## Surprises & Discoveries

- Observation: The codebase still has many internal `codex_home` variable and function names. These are implementation identifiers, not necessarily user-visible persistence names.
  Evidence: Focused `rg` output shows hundreds of `codex_home` variable references across config, app-server, thread-store, and state tests. Renaming all of them would exceed the Phase 1.5 persistence boundary and risk churn unrelated to behavior.
- Observation: The canonical default home resolver still uses `CODEX_HOME` and `~/.codex`.
  Evidence: `codex-rs/utils/home-dir/src/lib.rs` reads `std::env::var("CODEX_HOME")` and appends `.codex` when the env var is absent.
- Observation: Generated schema descriptions still mention `$CODEX_HOME` and `.codex`.
  Evidence: `codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json` and related generated schema files contain descriptions for `$CODEX_HOME/config.toml` and `.codex/` project folders.
- Observation: Some `.snap.new` files already exist under TUI snapshot folders before this milestone starts.
  Evidence: `find codex-rs -path '*snapshots*' -type f -name '*.snap'` output included sibling `.snap.new` paths in the working tree scan. This milestone will not accept or delete pre-existing pending snapshots unless a changed source requires it.

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

## Outcomes & Retrospective

Not completed yet. This section will record the final renamed surfaces, generated artifacts, deferred validation commands, and any remaining exceptions before the milestone branch is ready to merge.

## Baseline

The milestone starts from `main` at merge commit `eef144315e Merge pull request #8 from mbellary/datax/migration-phase1-4-app-server-protocol`. Phase 1.4 app-server protocol tests passed according to the user before this phase began. The starting branch for this milestone is `datax/migration-phase1-5-persistence-fixtures`.

Known constraint: Codex must not run the expensive test commands in this phase. Codex may run `just fmt`, `git diff --check`, and static `rg` checks. The user will run the documented build and test commands and report results.

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
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/persistence_fixtures_rename_execplan.md` | `In-Progress` | Living ExecPlan for this milestone. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/github_issue.md` | `Pending` | Record the milestone issue once created. |
| `docs/plans/datax_migration_phase1_5_persistence_fixtures/pull_request.md` | `Pending` | Record the draft PR once created. |
| `codex-rs/utils/home-dir/src/lib.rs` | `Pending` | Source of default home env var and fallback directory; should become `DATAX_HOME` and `.datax` while keeping the existing public helper name unless required. |
| `codex-rs/state/src/lib.rs` | `Pending` | Source of SQLite override env var; should become `DATAX_SQLITE_HOME`. |
| `codex-rs/config/src/config_toml.rs` | `Pending` | Config schema comments mention `~/.codex`, `$CODEX_HOME`, and `.codex`; source descriptions drive generated schema. |
| `codex-rs/config/src/types.rs` | `Pending` | Config type comments mention `CODEX_HOME`; source descriptions drive generated schema. |
| `codex-rs/config/src/loader/mod.rs` | `Pending` | Config loader docs and project folder discovery mention `/etc/codex`, `$CODEX_HOME`, and `.codex`. |
| `codex-rs/config/src/loader/layer_io.rs` | `Pending` | Managed config default path still uses `/etc/codex/managed_config.toml`. |
| `codex-rs/config/src/state.rs` | `Pending` | Config layer comments expose `.codex` and `$CODEX_HOME` descriptions. |
| `codex-rs/config/src/loader/README.md` | `Pending` | Build-adjacent developer README documents config layer order and project folder names. |
| `codex-rs/config/src/cloud_config_layers_tests.rs` | `Pending` | Test expectations include `/home/alice/.codex/config.toml`. |
| `codex-rs/config/src/loader/tests.rs` | `Pending` | Loader tests may encode project config folder behavior. |
| `codex-rs/config/src/config_requirements.rs` | `Pending` | Requirement tests include `com.openai.codex` and `com.codex`; inspect whether these are managed-domain examples or product-owned defaults. |
| `codex-rs/core/src/config/config_tests.rs` | `Pending` | Core config tests likely assert home and config layer behavior. |
| `codex-rs/core/src/config/config_loader_tests.rs` | `Pending` | Core config loader tests likely assert project `.codex` discovery. |
| `codex-rs/core/src/config/permissions_tests.rs` | `Pending` | Test fixture creates a `.codex` home path; inspect whether this is just a temporary directory or user-visible expectation. |
| `codex-rs/core/config.schema.json` | `Pending` | Generated config schema must be refreshed if config comments change. |
| `codex-rs/app-server-protocol/schema/json/v1/InitializeResponse.json` | `Pending` | Generated schema currently describes `$CODEX_HOME`; source may need regeneration. |
| `codex-rs/app-server-protocol/schema/json/v2/ConfigReadResponse.json` | `Pending` | Generated schema currently describes `$CODEX_HOME` and `.codex`. |
| `codex-rs/app-server-protocol/schema/json/v2/ConfigWriteResponse.json` | `Pending` | Generated schema currently describes `$CODEX_HOME` and `.codex`. |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json` | `Pending` | Aggregate generated schema currently contains persistence descriptions. |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json` | `Pending` | Aggregate generated v2 schema currently contains persistence descriptions. |
| `codex-rs/app-server-protocol/schema/typescript/InitializeResponse.ts` | `Pending` | Generated TypeScript docs may contain `$CODEX_HOME`. |
| `codex-rs/linux-sandbox/src/bwrap.rs` | `Pending` | Sandbox hidden-project folder protection currently includes `.codex`; Datax project folder should be protected. |
| `codex-rs/linux-sandbox/tests/suite/landlock.rs` | `Pending` | Landlock tests create and assert `.codex` protection. |
| `codex-rs/linux-sandbox/README.md` | `Pending` | README documents `.codex` protection and may need `.datax`. |
| `codex-rs/linux-sandbox/src/proxy_routing.rs` | `Pending` | Reads `CODEX_HOME` for temp proxy path; should follow the Datax home env if this is product-owned runtime behavior. |
| `codex-rs/network-proxy/src/certs.rs` | `Pending` | Error text says `CODEX_HOME`; inspect and update if using Datax home resolver. |
| `codex-rs/network-proxy/src/socks5.rs` | `Pending` | Comment mentions shared test `CODEX_HOME`; inspect for user-visible or source-of-truth impact. |
| `codex-rs/app-server-transport/src/transport/mod.rs` | `Pending` | Error text says failed to resolve `CODEX_HOME`; update if home resolver moves to `DATAX_HOME`. |
| `codex-rs/thread-store/src/local/update_thread_metadata.rs` | `Pending` | Contains `https://github.com/openai/codex` origin URL test data; inspect whether provenance exception or Datax-owned fixture. |
| `codex-rs/analytics/src/analytics_client_tests.rs` | `Pending` | Tests include `.codex/skills` paths; inspect whether path anonymization expectations should become `.datax`. |
| `codex-rs/tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__hooks_browser_view__tests__hooks_browser_events.snap` | `Pending` | Snapshot text includes "Codex ends its turn"; likely owned UI snapshot rename. |
| `codex-rs/tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__hooks_browser_view__tests__hooks_browser_events_with_issues.snap` | `Pending` | Snapshot text includes "Codex ends its turn"; likely owned UI snapshot rename. |
| `codex-rs/tui/src/snapshots/codex_tui__model_migration__tests__model_migration_prompt.snap` | `Pending` | Snapshot contains model migration product text; inspect whether belongs to fixture/snapshot rename. |
| `codex-rs/tui/tests/fixtures/oss-story.jsonl` | `Not Required` | Large recorded story fixture contains incidental words such as "returned" and `codex_event`; this is not Datax persistence behavior for Phase 1.5. |
| `codex-rs/tools/tests/fixtures/json_schema_policy/*.json` | `Not Required` | Third-party tool schemas use generic JSON Schema words such as `items` or vendor fields like `thread_ts`; not Datax product persistence. |
| `codex-rs/vendor/bubblewrap/**` | `Not Required` | Third-party vendor files; not Datax product-owned text. |

The inventory will be updated as implementation discovers additional exact test, fixture, generated, or snapshot files.

## Plan of Work

Update `codex-rs/utils/home-dir/src/lib.rs` so the Datax home resolver honors `DATAX_HOME` and falls back to `~/.datax`. Keep the public function name initially, because it is a Rust API used widely and changing it would be broad internal churn outside the behavior needed for this milestone. Update error messages and tests to assert `DATAX_HOME`.

Update `codex-rs/state/src/lib.rs` so `SQLITE_HOME_ENV` is `DATAX_SQLITE_HOME`. Then update documentation and generated schema descriptions that mention the old SQLite override.

Update config loader source comments and project folder discovery from `.codex` to `.datax`, including any tests that create project-local config folders. Update system paths from `/etc/codex` to `/etc/datax` where they are Datax-owned defaults. For macOS or Windows managed preference identifiers, inspect first: if they are compatibility IDs or MDM domains that must remain temporarily, record them in the exception register.

Update sandbox path protection to protect `.datax` in project roots. If `.codex` protection is still needed as a compatibility hardening exception for old checkouts, keep it only with a clear comment; otherwise replace the Datax-owned hidden folder.

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
| `git diff --check` | repository root | Deferred | No whitespace errors. |
| `rg -n 'Data[X]' docs/plans/datax_migration_phase1_5_persistence_fixtures` | repository root | Deferred | No forbidden mixed-case product spelling. |
| `rg -n 'CODEX_HOME|CODEX_SQLITE_HOME|\\.codex' codex-rs/utils/home-dir codex-rs/config/src codex-rs/core/src/config codex-rs/state/src codex-rs/linux-sandbox/src codex-rs/app-server-transport/src codex-rs/network-proxy/src` | repository root | Deferred | Only documented exceptions remain. |
| `just write-config-schema` | `codex-rs` | Deferred | Config schema regenerated if source descriptions changed. |
| `just write-app-server-schema` | `codex-rs` | Deferred | App-server schema artifacts regenerated if generated descriptions changed. |
| `just fmt` | `codex-rs` | Pending | Formatting completes successfully. |
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

From the repository root, run the persistence-name source check and expect only documented exceptions:

    rg -n 'CODEX_HOME|CODEX_SQLITE_HOME|\\.codex' codex-rs/utils/home-dir codex-rs/config/src codex-rs/core/src/config codex-rs/state/src codex-rs/linux-sandbox/src codex-rs/app-server-transport/src codex-rs/network-proxy/src

From `codex-rs`, regenerate the config schema when config comments or config types changed:

    just write-config-schema

From `codex-rs`, regenerate app-server schema artifacts when app-server generated descriptions changed:

    just write-app-server-schema

From `codex-rs`, run the formatter and expect it to complete:

    just fmt

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

GitHub issue and draft PR links will be added after creation.

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
