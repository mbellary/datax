# Phase 1.6 Test Isolation Pass

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` from the repository root. It also follows `docs/plans/Recommended-Datax-Migration-Execution-Model.md`, which requires a milestone branch, GitHub issue, draft pull request, file inventory, explicit validation commands, staged implementation, and current rename exception tracking.

## Purpose / Big Picture

Phase 1.6 turns the renamed Datax codebase from "the targeted phase tests pass when hand-fed fixes" into a testable migration baseline. The user-visible result is not a new feature; it is a cleaner repository where CI/test helper scripts, developer validation commands, and public app-server-over-MCP reference text no longer point at stale Codex package names or old `thread/*` and `turn/*` API names that were already renamed to `chat/*` and `interaction/*`.

After this milestone, a contributor should be able to run the documented validation commands and see failures only from real code regressions, not from stale migration names in test harnesses or command examples. Behavior remains equivalent except for correcting Datax-owned names that block or mislead validation.

## Progress

- [x] (2026-07-08 00:00Z) Created branch `datax/migration-phase1-6-test-isolation` from `main`.
- [x] (2026-07-08 00:00Z) Read `PLANS.md`, `docs/plans/Recommended-Datax-Migration-Execution-Model.md`, and `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`.
- [x] (2026-07-08 00:00Z) Searched for stale test, CI, and public app-server method references before implementation edits.
- [x] (2026-07-08 00:00Z) Created the initial file inventory and dependency order for this milestone.
- [x] (2026-07-08 06:13Z) Created GitHub issue #11 for this milestone.
- [x] (2026-07-08 06:15Z) Updated stale test-isolation files identified in the inventory.
- [x] (2026-07-08 06:16Z) Ran allowed static validation only; deferred expensive build, format, lint, generation, and tests to the user.
- [x] (2026-07-08 06:20Z) Created draft PR #12 for this milestone.
- [x] (2026-07-08 06:20Z) Updated this ExecPlan with final PR link and closeout notes.
- [x] (2026-07-08 07:25Z) Fixed user-reported `just test -p datax-tui` snapshot fixture failures by renaming tracked TUI Insta snapshots from `codex_tui__*.snap` to `datax_tui__*.snap` and accepting generated `datax_tui__*.snap.new` content for the failing cases.
- [x] (2026-07-08 07:33Z) Trimmed hook-browser snapshot helper output to avoid newly modified snapshots carrying right-padding that fails `git diff --check`.

## Surprises & Discoveries

- Observation: Phase 1.5 user validation already passed the major targeted Rust tests, so Phase 1.6 should avoid another broad mechanical rename.
  Evidence: `main` contains merge commit `236b706022` for PR #10 and the Phase 1.5 ExecPlan records that the user ran and passed the targeted validation sequence.
- Observation: The GitHub nextest platform workflow still injects old Cargo binary environment variables for renamed helper binaries even though it copies `datax-*` helper executables.
  Evidence: `.github/workflows/rust-ci-full-nextest-platform.yml` copies `datax-linux-sandbox`, `datax-windows-sandbox-setup.exe`, and `datax-command-runner.exe`, but still sets `CARGO_BIN_EXE_codex_linux_sandbox`, `CARGO_BIN_EXE_codex_windows_sandbox_setup`, and `CARGO_BIN_EXE_codex_command_runner`.
- Observation: The MCP interface reference still documents old public v2 method strings.
  Evidence: `codex-rs/docs/codex_mcp_interface.md` lists `thread/start`, `thread/resume`, `turn/start`, and `turn/interrupt` even though Phase 1.4 changed public app-server methods to `chat/*` and `interaction/*`.
- Observation: Some remaining Codex strings are intentionally not Phase 1.6 work.
  Evidence: `CODEX_SANDBOX_*` identifiers are protected; `.codex-plugin` is a plugin manifest format exception; ChatGPT backend `/backend-api/codex/` paths and model slugs such as `gpt-5-codex` are external service/provenance names; core-internal `thread_id` and `turn_id` names still compile and are not app-server protocol field names.
- Observation: Renaming the TUI crate/package changes Insta's snapshot file identity from `codex_tui__...snap` to `datax_tui__...snap`.
  Evidence: The user-run `just test -p datax-tui` generated `datax_tui__*.snap.new` files and failed because tracked fixtures still existed under `codex_tui__*.snap`.

## Decision Log

- Decision: Treat Phase 1.6 as a test-isolation and validation-readiness pass, not as a new broad rename band.
  Rationale: Previous phases already moved crate/package/protocol/persistence surfaces. Broadly rewriting remaining internal names would risk breaking tests again and belongs only to a later explicitly scoped cleanup if needed.
  Date/Author: 2026-07-08 / Codex.
- Decision: Do not run `just fmt`, `cargo build`, `cargo check`, `just fix`, `just test`, or generated schema commands in this milestone unless the user explicitly authorizes that exact command.
  Rationale: The user stated that Codex should not run those commands because they are expensive and destabilized WSL. This plan records exact commands for the user to run instead.
  Date/Author: 2026-07-08 / Codex.

## Outcomes & Retrospective

Implementation aligned active validation-facing files with Datax naming: CI helper binary environment variables, Cargo package command examples, app-server-over-MCP documentation, test helper binary lookups, and snapshots or fixtures that render validation commands. No behavior changes or product features were added.

Codex ran static validation only. Expensive format, build, lint, generation, and test commands remain deferred to the user per migration instructions and are listed explicitly below.

The milestone issue is #11 and the draft pull request is #12.

## Baseline

The milestone starts from `main` after Phase 1.5 was merged. The most recent commits at planning time are `ba134e9d8b update doc` and `236b706022 Merge pull request #10 from mbellary/datax/migration-phase1-5-persistence-fixtures`.

The starting branch for this milestone is `datax/migration-phase1-6-test-isolation`.

Known constraint: Codex must not run expensive build, check, lint, format, generation, or test commands in this phase unless the user explicitly authorizes the exact command. Codex may run static searches and `git diff --check`.

## Context and Orientation

The repository root is `/home/mbellary/wsl/projects/datax`. Rust code lives under `codex-rs`; the directory name is retained as a documented migration exception, while crate packages have been renamed to `datax-*`.

Test isolation means the files that decide how validation is run: GitHub Actions workflows, scripts that invoke Cargo package names, developer test command examples, and API reference text used by testers or client authors. This milestone is allowed to correct stale migration names in those surfaces. It is not allowed to change runtime behavior, add Datax product features, or mechanically rename core-internal thread/turn/item concepts that previous phases intentionally left in place.

## Rename Exception Register

The following references may remain after this milestone:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` in Rust code and tests. These are protected sandbox identifiers.
- `.codex-plugin` manifest directory names. These are plugin format names and were documented as an exception in Phase 1.5.
- ChatGPT/OpenAI backend paths such as `/backend-api/codex/`, request headers such as `x-codex-turn-state`, and model slugs such as `gpt-5-codex`. These are external service contract names, not Datax-owned local test harness names.
- The `codex-rs` directory path and Bazel labels under `//codex-rs/...`. Earlier phases documented that filesystem path rename is deferred.
- Core-internal Rust concepts and variable names such as `thread_id`, `turn_id`, `item_id`, `codex_thread`, `codex_home`, `codex_self_exe`, and `CodexHomeUserInstructionsProvider` when they are implementation names rather than public app-server protocol fields.
- Historical phase ExecPlans that describe earlier decisions, failures, or command outputs using old names for context.

## Public Surface Checklist

This milestone touches:

- CI/test helper environment variables for renamed Cargo helper binaries.
- Developer validation command examples that still refer to old package names.
- App-server-over-MCP reference text that still describes old public method names.

This milestone does not touch:

- CLI arguments or command behavior.
- App-server Rust protocol source types.
- Config keys or persisted data formats.
- Generated schemas, TypeScript bindings, or snapshots unless user-run validation exposes a stale generated artifact.
- Runtime telemetry semantics or external service paths.

## Dependency Order

First, correct test harness names that can directly cause validation failure. The GitHub nextest workflow should set Cargo binary environment variables using the renamed `datax_*` names, matching the helper binary package names.

Second, correct scripts and developer command examples that invoke old package names. These are lower risk than source changes but important because Phase 1.6 is about making validation repeatable.

Third, correct app-server-over-MCP reference text that still advertises old public `thread/*`, `turn/*`, and `item/*` API names. This is documentation for an already-renamed protocol, not a protocol implementation change.

Finally, run static checks only. Expensive format, build, lint, generation, and tests remain deferred to the user.

## File Inventory

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `.github/dependabot.yaml` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `.github/scripts/verify_tui_core_boundary.py` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `.github/workflows/rust-ci-full-nextest-platform.yml` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/app-server-client/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/app-server-test-client/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/app-server/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/app-server/tests/common/test_app_server.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/app-server/tests/suite/v2/executor_mcp.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/arg0/src/lib.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/src/doctor.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/app_server.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/debug_clear_memories.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/debug_models.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/delete.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/exec_server.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/execpolicy.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/features.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/login.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/marketplace_add.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/marketplace_remove.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/marketplace_upgrade.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/mcp_add_remove.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/mcp_list.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/plugin_cli.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/sandbox_network_proxy.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cli/tests/update.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/cloud-config/src/lib.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/codex-api/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/codex-api/src/provider.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/codex-client/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/codex-mcp/src/mcp/mod.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/config/scripts/generate-proto.sh` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/config/src/loader/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/config/src/requirements_exec_policy.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/src/agent/role_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/src/command_canonicalization_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/src/config/mod.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/src/landlock.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/src/tools/handlers/unified_exec_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/common/lib.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/common/test_codex.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/common/test_codex_exec.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/common/zsh_fork.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/suite/apply_patch_cli.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/suite/cli_stream.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/suite/live_cli.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/core/tests/suite/mod.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/deny.toml` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/docs/codex_mcp_interface.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec-server/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec-server/src/noise_channel_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec-server/src/protocol.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec-server/testing/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec/src/main.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec/tests/suite/apply_patch.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec/tests/suite/originator.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/exec/tests/suite/server_error_exit.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/execpolicy-legacy/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/execpolicy/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/linux-sandbox/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/linux-sandbox/src/lib.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/linux-sandbox/src/linux_run_main_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/mcp-server/tests/common/mcp_process.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/network-proxy/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/otel/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/protocol/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/protocol/src/mcp.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/rmcp-client/src/bin/test_stdio_server.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/rmcp-client/tests/streamable_http_test_support.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/sandboxing/src/landlock.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/sandboxing/src/manager.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/sandboxing/src/manager_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/shell-escalation/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/shell-escalation/src/unix/escalate_server.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/stdio-to-uds/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/stdio-to-uds/src/main.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/stdio-to-uds/tests/stdio_to_uds.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/thread-manager-sample/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/app/agent_status_feed_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/bottom_pane/chat_composer_history.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/bottom_pane/hooks_browser_view.rs` | `Completed` | Test helper trims captured snapshot lines after accepting generated TUI snapshot output; runtime rendering is unchanged. |
| `codex-rs/tui/**/snapshots/datax_tui__*.snap` | `Completed` | Mechanical TUI Insta fixture rename from `codex_tui__*.snap`; generated `.snap.new` output from user-run `just test -p datax-tui` was accepted for failing snapshots. |
| `codex-rs/tui/src/chatwidget/snapshots/datax_tui__chatwidget__tests__binary_size_ideal_response.snap` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/chatwidget/snapshots/datax_tui__chatwidget__tests__unified_exec_wait_after_final_agent_message.snap` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/chatwidget/snapshots/datax_tui__chatwidget__tests__unified_exec_wait_before_streamed_agent_message.snap` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/chatwidget/snapshots/datax_tui__chatwidget__tests__unified_exec_wait_status_renders_command_in_single_details_row.snap` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/chatwidget/tests.rs` | `Completed` | Test-only explicit snapshot helper names now point to `datax_tui__*.snap` fixtures. |
| `codex-rs/tui/src/chatwidget/tests/exec_flow.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/history_cell/snapshots/datax_tui__history_cell__tests__multiline_command_wraps_with_extra_indent_on_subsequent_lines.snap` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/history_cell/tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/markdown_render_tests.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/status_indicator_widget.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/src/streaming/controller.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/tui/tests/suite/resize_reflow.rs` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/utils/pty/README.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `codex-rs/windows-sandbox-rs/sandbox_smoketests.py` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `docs/contributing.md` | `Completed` | Phase 1.6 test-isolation rename/update; active validation surface aligned to Datax naming. |
| `docs/plans/datax_migration_phase1_6_test_isolation/github_issue.md` | `Completed` | Records milestone issue #11. |
| `docs/plans/datax_migration_phase1_6_test_isolation/pull_request.md` | `Completed` | Records draft PR #12. |
| `docs/plans/datax_migration_phase1_6_test_isolation/test_isolation_execplan.md` | `Completed` | Living ExecPlan for this milestone. |

## Plan of Work

Update the CI workflow and scripts first because they can block validation even when source code is correct. In `.github/workflows/rust-ci-full-nextest-platform.yml`, remove the stale `CARGO_BIN_EXE_codex_*` exports and provide only the renamed Datax variable names expected by Cargo for renamed binaries: `CARGO_BIN_EXE_datax-linux-sandbox`, `CARGO_BIN_EXE_datax-windows-sandbox-setup`, and `CARGO_BIN_EXE_datax-command-runner`.

Then update developer command examples that point at old Cargo package names. Use `datax-*` package names in command examples because Phase 1.3 renamed internal packages.

Then update `codex-rs/docs/codex_mcp_interface.md` so the public app-server-over-MCP documentation reflects the already-renamed API. Use `datax mcp-server`, `chat/*`, `interaction/*`, and `message/*` terminology for current public v2 surfaces. Keep v1 compatibility method names and external MCP tool names unchanged where the source still owns them.

If inspection shows that a candidate file only contains internal compatibility names, mark it `Not Required` with a reason instead of editing it.

## Validation Matrix

| Command | Working Directory | Required Before Merge | Status | Notes |
| --- | --- | --- | --- | --- |
| `git diff --check` | repository root | Yes | `Completed` | Static whitespace check returned no output. |
| `rg -n "CARGO_BIN_EXE_codex_(linux_sandbox|windows_sandbox_setup|command_runner)" .github codex-rs` | repository root | Yes | `Completed` | Returned no matches. |
| `rg -n "cargo_bin\\(\\\"codex|should find binary for codex|codex-(linux-sandbox|mcp-server|execve-wrapper|exec-server|exec\\b)|cargo test -p codex|cargo insta pending-snapshots -p codex|just test -p codex|cargo run -p codex|cargo build -p codex" codex-rs .github docs scripts --glob '!target/**' --glob '!Cargo.lock' --glob '!*.snap.new'` | repository root | Yes | `Completed` | Returned only historical ExecPlan references after active validation surfaces were updated. |
| `rg -n "thread/start|thread/read|thread/list|thread/resume|thread/fork|turn/start|turn/interrupt|item/" codex-rs/docs/codex_mcp_interface.md codex-rs/app-server/README.md codex-rs/app-server-protocol/src codex-rs/app-server/src` | repository root | Yes | `Completed` | Returned no matches. |
| `find codex-rs/tui -path '*/snapshots/codex_tui__*.snap' -print` | repository root | Yes | `Completed` | Returned no tracked fixture files after TUI snapshot rename. |
| `just fmt` | `codex-rs` | Yes | `Deferred` | User-run per instruction. |
| `cargo build` | `codex-rs` | Yes | `Deferred` | User-run full build because this phase targets validation readiness. |
| `just fix -p datax-config` | `codex-rs` | If script/doc comments touch config crate | `Deferred` | User-run if this milestone changes config generator or config crate code. |
| `just fix -p datax-linux-sandbox` | `codex-rs` | If linux sandbox files change | `Deferred` | User-run if this milestone changes linux sandbox source or tests. |
| `just fix -p datax-stdio-to-uds` | `codex-rs` | If stdio-to-uds files change | `Deferred` | User-run if this package is touched. |
| `just test -p datax-config` | `codex-rs` | If config generator changes | `Deferred` | User-run. |
| `just test -p datax-linux-sandbox` | `codex-rs` | If linux sandbox files change | `Deferred` | User-run. |
| `just test -p datax-stdio-to-uds` | `codex-rs` | If stdio-to-uds files change | `Deferred` | User-run. |
| `just test -p datax-tui` | `codex-rs` | If TUI boundary verifier or docs require it | `Deferred` | User-run only if needed. |
| `just test` | `codex-rs` | Final Phase 1 stabilization check | `Deferred` | User-run full suite. |

## Validation and Acceptance

From the repository root, run the whitespace check and expect no output:

    git diff --check

From the repository root, confirm stale nextest helper variables were removed and expect no output:

    rg -n "CARGO_BIN_EXE_codex_(linux_sandbox|windows_sandbox_setup|command_runner)" .github codex-rs

From the repository root, confirm active developer/test command examples do not point at old package names. Historical phase plans may appear and should be reviewed as documented history:

    rg -n "just test -p codex|just fix -p codex|cargo run -p codex|cargo build -p codex" docs codex-rs .github scripts --glob '!target/**' --glob '!Cargo.lock'

From the repository root, confirm active public app-server documentation does not advertise old method names:

    rg -n "thread/start|thread/read|thread/list|thread/resume|thread/fork|turn/start|turn/interrupt|item/" codex-rs/docs/codex_mcp_interface.md codex-rs/app-server/README.md codex-rs/app-server-protocol/src codex-rs/app-server/src

From the repository root, confirm no tracked TUI snapshot fixtures retain the old crate prefix:

    find codex-rs/tui -path '*/snapshots/codex_tui__*.snap' -print

From `codex-rs`, run the formatter and expect it to complete successfully:

    just fmt

From `codex-rs`, run a full build and expect it to complete successfully:

    cargo build

From `codex-rs`, run the targeted lint/fix commands for touched Rust packages and expect them to complete successfully:

    just fix -p datax-config
    just fix -p datax-linux-sandbox
    just fix -p datax-stdio-to-uds

From `codex-rs`, run the targeted tests for touched Rust packages and expect them to pass:

    just test -p datax-config
    just test -p datax-linux-sandbox
    just test -p datax-stdio-to-uds
    just test -p datax-tui

From `codex-rs`, run the final full suite only when the user is ready for Phase 1 stabilization validation and expect it to pass:

    just test

## Idempotence and Recovery

All planned edits are text changes to checked-in source, workflow, script, and documentation files. They can be reviewed with `git diff` and reverted file-by-file if a change proves too broad. No migration or destructive operation is required.

If a validation command fails because of a stale name in a file not listed above, first add that file to the inventory, record whether it is Datax-owned or an exception, then make the smallest fix. Do not run broad mechanical rewrites across the repository.

## Artifacts and Notes

GitHub issue #11 is recorded in `github_issue.md`. Draft PR #12 is recorded in `pull_request.md`.

## Interfaces and Dependencies

No new Rust interface is introduced. The expected end state is that existing validation interfaces use the renamed package and helper-binary names:

- Cargo package names are `datax-*`.
- Cargo binary environment variables for helper binaries use Cargo's hyphen-to-underscore conversion, for example `CARGO_BIN_EXE_datax_linux_sandbox` when Cargo emits an underscored binary variable and `CARGO_BIN_EXE_datax-linux-sandbox` where the existing test uses the hyphenated form.
- Public app-server-over-MCP v2 methods use `chat/*`, `interaction/*`, and `message/*` terminology.

Revision note: Initial Phase 1.6 ExecPlan created to scope test isolation work before implementation edits, per the recommended Datax migration execution model.
