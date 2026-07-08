# Phase 1.7 Migration Freeze Checkpoint

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` from the repository root. It also follows `docs/plans/Recommended-Datax-Migration-Execution-Model.md`, which requires a milestone branch, GitHub issue, draft pull request, file inventory, explicit validation commands, staged implementation, and current rename exception tracking.

## Purpose / Big Picture

Phase 1.7 turns the completed Phase 1 rename work into a freeze-ready migration baseline. A freeze-ready baseline means the repository can be inspected, built, generated, tested, and smoke-tested as Datax without active tooling or GitHub metadata still directing users and automation toward Codex-owned names.

The user-visible outcome is a concrete freeze checklist plus a cleanup of active repository metadata that would otherwise make the final migration validation ambiguous. After this work, the user can run the freeze checklist commands and any remaining Codex-shaped names can be classified as protected, external, provenance, path-deferred, or explicitly deferred implementation names rather than accidental active product identity.

## Progress

- [x] (2026-07-08 00:00Z) Created branch `datax/migration-phase1-7-freeze-report` from updated `main`.
- [x] (2026-07-08 00:00Z) Created `docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md` with explicit freeze validation commands.
- [x] (2026-07-08 00:00Z) Linked the concrete freeze checklist from `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`.
- [x] (2026-07-08 00:00Z) Reviewed user-provided static-search output and identified active Datax-owned metadata still using Codex naming.
- [x] (2026-07-08 00:00Z) Fixed active root tooling, issue templates, CODEOWNERS, and CI labels/package arguments that still used Codex-owned names.
- [x] (2026-07-08 00:00Z) Fixed active PR template, CLA workflow document URL, and contributor guide wording that still pointed contributors at upstream Codex identity.
- [x] (2026-07-08 00:00Z) Ran lightweight static validation; `git diff --check` passed and focused stale package metadata search now returns only `codex-sdk`.
- [x] (2026-07-08 00:00Z) Inspected the `codex-sdk` match and found it is an active SDK/package surface, not a safe freeze exception.
- [x] (2026-07-08 00:00Z) Created GitHub issue #13 for Phase 1.7.
- [x] (2026-07-08 00:00Z) Created draft PR #14 for Phase 1.7.
- [x] Run allowed static validation only; build, format, generation, fix, and test commands remain user-run.
- [x] (2026-07-08 00:00Z) Took stock after expanding Phase 1.7 scope to include `codex-rs` path cleanup, internal-name inventory, and downstream Codex artifact notes.
- [x] (2026-07-08 00:00Z) Updated the Phase 1.7 file inventory with track-level stocktake rows. The `codex-sdk` surface is isolated and remains pending for a later track.
- [x] (2026-07-08 00:00Z) Added Track A runner commands for the `codex-rs` to `datax-rs` path rename inventory and validation.
- [x] (2026-07-08 00:00Z) Implemented Track A by moving `codex-rs/` to `datax-rs/` with `git mv`.
- [x] (2026-07-08 00:00Z) Updated active writable repository path references from `codex-rs` to `datax-rs`.
- [x] (2026-07-08 00:00Z) Confirmed no writable-tree `codex-rs` path references remain outside `.codex/` local metadata.
- [x] (2026-07-08 00:00Z) Inventoried internal Codex-era names and split them into public/wire/generated/persistence/snapshot identity debt versus internal-only classification entries.
- [x] (2026-07-08 00:00Z) Ran Track B post-change static status commands and confirmed the output matches the classified audit baseline.
- [ ] Inventory downstream Codex artifacts discovered during the Phase 1.7 scans for Phase 2 adapter/runtime planning.
- [ ] Record final validation evidence from the user and close out the freeze report.

## Surprises & Discoveries

- Observation: The broad remaining-Codex inventory found thousands of matches, but that count alone is not actionable because it includes deferred `codex-rs` paths, external service contracts, protected sandbox identifiers, and historical migration plans.
  Evidence: The user-run inventory command reported `9081 /tmp/datax_phase1_remaining_codex_refs.txt`.
- Observation: The broad inventory also exposed active metadata that is not safe to leave as a generic exception.
  Evidence: `package.json` still used `-p codex-hooks`; `.github/ISSUE_TEMPLATE/*` still presented Codex issue templates; `.github/CODEOWNERS` still assigned OpenAI Codex owners; and `.github/workflows/ci.yml` still referenced the removed `scripts/codex_package` path.
- Observation: Before Track A, the repository still used `codex-rs` as the Rust workspace directory.
  Evidence: root `justfile`, GitHub workflows, scripts, and generated paths referred to `codex-rs`; earlier milestones documented filesystem directory rename as deferred.
- Observation: The focused package metadata check is now narrowed to `codex-sdk`.
  Evidence: `rg -n "codex-hooks|codex_package|--package codex|CODEX_VERSION|Test Codex package builder|Codex package|codex-cli/README" package.json .gitignore .github scripts datax-cli justfile` returns only `.github/workflows/rust-release.yml:1200: --package codex-sdk`.
- Observation: `codex-sdk` is not only a release workflow spelling; it is backed by an active TypeScript SDK package.
  Evidence: `datax-cli/scripts/build_npm_package.py` has a `codex-sdk` package branch that stages `sdk/typescript`; `sdk/typescript/package.json` names the package `@openai/codex-sdk`; SDK samples import `Codex` from `@openai/codex-sdk`; and `sdk/typescript/src/exec.ts` still resolves `@openai/codex` platform packages.
- Observation: The top-level `codex-rs` rename has high blast radius and must be a dedicated implementation slice.
  Evidence: `rg -l --hidden "codex-rs" . --glob '!.git/**' --glob '!target/**' --glob '!codex-rs/target/**' --glob '!**/*.snap.new' | wc -l` reported 142 files. The largest groups are `codex-rs` itself, `.github`, `docs`, `scripts`, `tools`, and `sdk`.
- Observation: Internal Codex-era names are too broad for a blanket rewrite.
  Evidence: `rg -l --hidden "\b(thread_id|turn_id|ThreadId|TurnId|codex_thread|codex_turn|codex_turns)\b" . --glob '!.git/**' --glob '!target/**' --glob '!datax-rs/target/**' --glob '!**/*.snap.new' | wc -l` reported 557 files, mostly under `datax-rs` plus SDK and docs.
- Observation: Track B internal-name inventory remains broad after Track A and crosses source, generated artifacts, persisted schema, SDKs, and tests.
  Evidence: The Track B file inventory reported 527 files under `datax-rs`, 25 under `sdk`, and 6 under `docs`. The largest `datax-rs` groups were `core` (144 files), `tui` (87), `app-server` (39), `app-server-protocol` (34), `rollout-trace` (27), `hooks` (26), `state` (25), `ext` (25), and `thread-store` (19).
- Observation: The narrow `codex_thread` / `codex_turn` inventory is concentrated enough to classify, but not safe for a same-track mechanical rename.
  Evidence: Matches appear in `datax-rs/core/src/codex_thread.rs`, `datax-rs/core/src/lib.rs`, `datax-rs/core/src/thread_manager.rs`, core session/task/agent modules, rollout-trace reducers and model code, `datax-rs/app-server/src/request_processors/chat_processor.rs`, and historical migration plans.
- Observation: `ThreadId`, `thread_id`, and `turn_id` are not just local variable names.
  Evidence: Matches include `datax-rs/protocol/src/thread_id.rs`, app-server v1 protocol structs, generated TypeScript and JSON schemas, Python SDK generated types, SQLite migration files, rollout trace reducers, thread-store state, app-server test fixtures, and TUI state.
- Observation: Snapshot identity debt exists, but it is not a single category.
  Evidence: Existing snapshot files include generated identity names such as `codex_core__...` and `codex__doctor__...`; pending `.snap.new` files include many `codex_tui__...` paths. Separate snapshots named `datax_tui__model_migration__tests__model_migration_prompt_gpt5_codex*.snap` are model-slug references and are not Datax product identity debt.
- Observation: Track B post-change static status commands matched the recorded baseline.
  Evidence: The root grouping returned 527 `datax-rs`, 25 `sdk`, and 6 `docs` files. The `datax-rs` grouping returned the same largest groups recorded in this plan: `core` (144), `tui` (87), `app-server` (39), `app-server-protocol` (34), `rollout-trace` (27), `hooks` (26), `state` (25), `ext` (25), and `thread-store` (19). The `codex_thread` / `codex_turn` file list and snapshot file list also matched the classified Track B categories.
- Observation: Downstream Codex artifact discovery should be recorded for Phase 2 rather than implemented in Phase 1.
  Evidence: A focused artifact-candidate search across app-server, SDK, scripts, and GitHub metadata reported 184 files. These need classification before Phase 2, not adapter wiring during Phase 1.

## Decision Log

- Decision: Treat Phase 1.7 as a freeze checkpoint plus active identity cleanup.
  Rationale: Phase 1 must close with a coherent Datax baseline. Public and serialized Codex-era names are blockers; internal-only inherited names must be inventoried and either renamed or classified before freeze.
  Date/Author: 2026-07-08 / Codex.
- Decision: Do not run `just fmt`, `cargo build`, `cargo check`, `just fix`, `just test`, schema generation, or full smoke commands unless the user explicitly authorizes that exact command.
  Rationale: The user will run expensive commands and paste output. The freeze checklist records exact commands and evidence expectations.
  Date/Author: 2026-07-08 / Codex.
- Decision: Remove OpenAI Codex CODEOWNERS assignments rather than inventing replacement owners.
  Rationale: Datax repository-local owners are not known in this thread. An empty CODEOWNERS file with a Datax migration note is more accurate than retaining OpenAI Codex team ownership.
  Date/Author: 2026-07-08 / Codex.
- Decision: Treat `codex-rs` path references as Phase 1.7 cleanup scope, not accepted freeze exceptions.
  Rationale: `codex-rs` is a Datax-owned source path, not a downstream Codex runtime boundary. It must be inventoried first, then renamed to `datax-rs` with build, CI, package, schema, Bazel, and docs references updated in the same checkpoint.
  Date/Author: 2026-07-08 / Codex.
- Decision: Inventory and classify internal Codex-era identifiers in Phase 1.7.
  Rationale: Names such as `thread_id`, `turn_id`, `codex_thread`, and `codex_turns` may be internal implementation details, but Phase 1 should not close with unknown identity debt. Public, wire, generated, CLI-visible, fixture, or documentation occurrences must be renamed. Internal-only occurrences may remain only when explicitly classified.
  Date/Author: 2026-07-08 / Codex.
- Decision: Treat Track B as a freeze audit/classification gate, not as permission for a source-wide internal rename.
  Rationale: The inventory crosses public protocol compatibility, generated schemas, SDKs, persisted SQLite migrations, rollout traces, and TUI/app-server internals. A broad rename would risk breaking compatibility and would obscure which names are owned by downstream Codex compatibility versus Datax-owned implementation.
  Date/Author: 2026-07-08 / Codex.
- Decision: Defer actual `ThreadId` / `thread_id` / `turn_id` / `codex_thread` / `codex_turn` source renames into dependency-specific follow-up slices.
  Rationale: These names represent different concepts at different layers: app-server chat/interaction wire vocabulary, persisted thread-store schema, rollout trace model identity, TUI local state, core session orchestration, and future downstream Codex compatibility. Each slice needs its own owner, schema plan, and test plan.
  Date/Author: 2026-07-08 / Codex.
- Decision: Record downstream Codex artifacts discovered during Phase 1 scans for Phase 2.
  Rationale: Phase 1 scanning will expose artifacts that may belong to the future downstream Codex app-server adapter/runtime boundary. Capturing them now helps Phase 2 without adding adapter implementation or runtime coupling during Phase 1.
  Date/Author: 2026-07-08 / Codex.
- Decision: Do not classify `codex-sdk` as an accepted freeze exception without a dedicated decision.
  Rationale: It is an active public package and SDK surface, not merely historical text. Renaming it touches TypeScript SDK package metadata, imports, tests, release packaging, and possibly Python SDK references. That is too broad to silently include in metadata cleanup, but it is also too active to accept as a harmless exception.
  Date/Author: 2026-07-08 / Codex.

## Outcomes & Retrospective

In progress. The active metadata cleanup has been implemented, Phase 1.7 issue #13 has been created, and draft PR #14 has been opened. The focused package metadata gate now exposes `codex-sdk` as a remaining active SDK/package rename question, so Phase 1 freeze is not accepted yet. Final outcomes will be recorded after that question is resolved and user-run freeze checklist evidence is complete.

## Context and Orientation

The repository root is `/home/mbellary/wsl/projects/datax`. Rust code now lives under `datax-rs`; before Track A it lived under `codex-rs`. The product identity is Datax, and active user-facing repository surfaces should not instruct users to file Codex issues, run Codex packages, or rely on OpenAI Codex ownership metadata.

Phase 1.7 starts after Phase 1.6 has been merged into `main`. The user confirmed Phase 1.6 tests passed and updated local `main`. The work in this plan happens on branch `datax/migration-phase1-7-freeze-report`.

The concrete freeze checklist lives at `docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md`. That checklist is runner-facing: it lists exact commands for the user to execute and output to paste back into the thread.

The milestone GitHub issue is #13: `https://github.com/mbellary/datax/issues/13`.

The milestone draft pull request is #14: `https://github.com/mbellary/datax/pull/14`.

## Rename Exception Register

The following Codex-shaped references may remain after this milestone if they are recorded in the final freeze report:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`, `CODEX_SANDBOX_ENV_VAR`, `CODEX_SANDBOX_NETWORK_DISABLED`, and `CODEX_SANDBOX`. These are protected sandbox identifiers.
- External service paths such as `https://chatgpt.com/backend-api/codex`.
- External model slugs such as `gpt-5-codex` when used as model examples.
- Upstream artifact sources such as OpenAI Codex release URLs when still required to fetch externally built artifacts.
- Historical migration plans that discuss Codex-to-Datax migration decisions.
- `codex-sdk` package names are not accepted as a freeze exception yet. They remain an unresolved active SDK/package surface until a dedicated rename or explicit deferral decision is recorded.

The following references are Phase 1.7 cleanup or classification scope, not accepted exceptions:

- The old `codex-rs` directory path and Bazel labels under `//codex-rs/...`.
- Internal Codex-era identifiers such as `thread_id`, `turn_id`, `ThreadId`, `TurnId`, `codex_thread`, `codex_turn`, and `codex_turns`.

The following artifacts should be recorded for Phase 2 if discovered during Phase 1 scans, but should not be implemented in Phase 1:

- App-server process launch points.
- JSON-RPC transport code and protocol client/server types.
- Schema generation outputs that define downstream Codex compatibility surfaces.
- SDK/package surfaces that still refer to Codex runtime packages.
- External service URLs and upstream release artifact URLs.
- Runtime process-management scripts and compatibility shims.

## Public Surface Checklist

This milestone touches active GitHub issue templates, root package scripts, active CI labels and package staging arguments, CODEOWNERS metadata, and Phase 1 test-plan documentation.

This milestone does not touch Rust runtime behavior, app-server protocol source, generated schemas, persistence formats, CLI arguments, or npm launcher runtime code. Track A updates path strings in tests and snapshots when those strings point at the Rust workspace directory.

## Dependency Order

First, create the Phase 1.7 branch and preserve the uncommitted freeze checklist work there.

Second, fix active metadata and tooling references that are unambiguously Datax-owned: root `package.json`, `.gitignore`, issue templates, CODEOWNERS, and CI labels/package names.

Third, update this ExecPlan and the freeze checklist so the user can rerun focused static searches before expensive validation.

Fourth, create the GitHub issue and draft pull request.

Finally, run only lightweight static checks. The user runs the build, generation, formatting, fix, test, and smoke commands from the freeze checklist.

## File Inventory

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `.gitignore` | `Completed` | Active ignore entry now points at `datax-cli/README.md` instead of `codex-cli/README.md`. |
| `.github/CODEOWNERS` | `Completed` | Removed OpenAI Codex team ownership and recorded that Datax ownership is unset during Phase 1 freeze. |
| `.github/ISSUE_TEMPLATE/1-codex-app.yml` | `Completed` | Renamed to `.github/ISSUE_TEMPLATE/1-datax-app.yml`. |
| `.github/ISSUE_TEMPLATE/1-datax-app.yml` | `Completed` | User-facing app issue template now uses Datax naming. |
| `.github/ISSUE_TEMPLATE/3-cli.yml` | `Not Required` | Inspected because it contains a `gpt-5.2-codex` model example; that is an external model slug, not product identity. |
| `.github/ISSUE_TEMPLATE/4-bug-report.yml` | `Completed` | User-facing bug template now uses Datax wording and no longer links users to upstream Codex discussions. |
| `.github/ISSUE_TEMPLATE/5-feature-request.yml` | `Completed` | User-facing feature template now uses Datax wording and no longer links to upstream Codex contributing policy. |
| `.github/ISSUE_TEMPLATE/6-docs-issue.yml` | `Completed` | Documentation request copy now says Datax. |
| `.github/pull_request_template.md` | `Completed` | Active PR template now links to the Datax contributing document instead of upstream Codex. |
| `.github/workflows/cla.yml` | `Completed` | CLA workflow now points to the Datax repository CLA document path. |
| `.github/workflows/ci.yml` | `Completed` | Active CI labels, package-builder path, variable name, and package staging argument now use Datax names. |
| `.github/workflows/rust-release-windows.yml` | `Completed` | Active release step label now says Datax package archives. |
| `.github/workflows/rust-release.yml` | `Pending` | Isolated `codex-sdk` track. Active release workflow still stages `--package codex-sdk`; fix later after user approval for the SDK/package track. |
| `datax-cli/scripts/build_npm_package.py` | `Pending` | Isolated `codex-sdk` track. Active package builder still has a `codex-sdk` package branch; fix later after user approval for the SDK/package track. |
| `sdk/typescript/package.json` | `Pending` | Isolated `codex-sdk` track. Active TypeScript SDK package name remains `@openai/codex-sdk`; fix later after user approval for the SDK/package track. |
| `sdk/typescript/src/exec.ts` | `Pending` | Isolated `codex-sdk` track. Active TypeScript SDK runtime still resolves `@openai/codex` platform packages; fix later after user approval for the SDK/package track. |
| `sdk/typescript/samples/*.ts` | `Pending` | Isolated `codex-sdk` track. SDK samples still import `Codex` from `@openai/codex-sdk`; fix later after user approval for the SDK/package track. |
| `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` | `Completed` | Updated high-level Phase 1 plan with Phase 1.7 identity cleanup, `datax-rs`, internal-name audit, and downstream artifact note. |
| `docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md` | `Completed` | Concrete Phase 1.7 runner-facing freeze checklist. |
| `docs/plans/datax_migration_phase1_7_freeze_report/github_issue.md` | `Completed` | Records the GitHub issue body for issue #13. |
| `docs/plans/datax_migration_phase1_7_freeze_report/migration_freeze_checkpoint_execplan.md` | `Completed` | This living ExecPlan. |
| `docs/CLA.md` | `Completed` | Project name in the CLA text now says Datax CLI. Legal grant terms were not otherwise rewritten. |
| `docs/contributing.md` | `Completed` | Contributor guidance now uses Datax maintainer and Datax CLI wording. |
| `package.json` | `Completed` | Root hook-schema script now targets `datax-hooks`. |

### Phase 1.7 Track Inventory

These rows describe the implementation tracks now in Phase 1.7 scope. Each track must be expanded into a per-file implementation inventory before files in that track are modified. The `codex-sdk` track is intentionally isolated and will be fixed later after user approval.

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `datax-rs/` | `Completed` | Track A: moved from `codex-rs/` to `datax-rs/` with `git mv`; in-tree path references now point at `datax-rs`. |
| `.github/` | `Completed` | Track A: CI/workflow/action references to the Rust workspace path now point at `datax-rs`. |
| `docs/` | `Completed` | Track A: active and historical plan path references were updated to the new source path where they describe repository layout. |
| `scripts/` | `Completed` | Track A: helper and packaging script references to the Rust workspace path now point at `datax-rs`. |
| `tools/` | `Completed` | Track A: tooling references to the Rust workspace path now point at `datax-rs`. |
| `sdk/` | `Completed` | Track A: SDK test/helper source-path references now point at `datax-rs`; `codex-sdk` package names remain untouched for the later isolated track. |
| `.codex/` | `No Change Required` | Local Codex app skill/environment metadata remains read-only in this sandbox. It is excluded from Track A implementation and is not active repository source. |
| `.devcontainer/` | `Completed` | Track A: development container references to the Rust workspace path now point at `datax-rs`. |
| `MODULE.bazel` | `Completed` | Track A: Bazel workspace path reference now points at `//datax-rs`. |
| `defs.bzl` | `Completed` | Track A: Bazel helper path references now point at `datax-rs` / `//datax-rs`. |
| `.bazelrc` | `Completed` | Track A: Bazel configuration path references now point at `datax-rs` / `//datax-rs`. |
| `.bazelignore` | `Completed` | Track A: Bazel ignore path now points at `datax-rs/target`. |
| `.gitattributes` | `Completed` | Track A: repository path metadata now points at `datax-rs`. |
| `AGENTS.md` | `Completed` | Track A path references now point at `datax-rs`. Stale non-path Codex-era instruction wording remains a separate migration-policy concern. |
| `justfile` | `Completed` | Track A: root task working directory and Bazel labels now point at `datax-rs`. |
| `package.json` | `Completed` | Track A: root script manifest path now points at `datax-rs/Cargo.toml`. |
| `datax-cli/` | `Completed` | Track A: package-building path reference now points at `datax-rs`. |
| `pnpm-workspace.yaml` | `Completed` | Track A: workspace path reference now points at `datax-rs/responses-api-proxy/npm`. |
| `pnpm-lock.yaml` | `Completed` | Track A: checked-in pnpm workspace path metadata now points at `datax-rs/responses-api-proxy/npm`. |
| `flake.nix` | `Completed` | Track A: Nix path references now point at `datax-rs`. |
| `bazel/` | `Completed` | Track A: Bazel support path reference now points at `//datax-rs`. |
| `third_party/` | `Completed` | Track A: V8 documentation source-path references now point at `datax-rs/Cargo.lock`. |
| `datax-rs/**/*.rs` | `Completed` | Track B audit: 527 files contain internal Codex-era names. Classified as dependency-specific migration debt, not a mechanical rename. Major groups are `core`, `tui`, `app-server`, `app-server-protocol`, `rollout-trace`, `hooks`, `state`, `ext`, and `thread-store`. |
| `sdk/**/*` | `Completed` | Track B audit: 25 files contain internal-name matches. Classified as SDK/API and generated-client debt; do not edit until the SDK/package track is approved. |
| `docs/**/*` | `Completed` | Track B audit: 6 files contain matches. Current matches are planning/history records and the Phase 1 policy itself. No product documentation rewrite in this track. |
| `datax-rs/protocol/src/thread_id.rs` | `Deferred` | Track B classification: source owner for `ThreadId`. This is a core protocol type and must be renamed only with downstream protocol, app-server, SDK, persistence, and generated schema review. |
| `datax-rs/app-server-protocol/src/protocol/v1.rs` | `Deferred` | Track B classification: v1 compatibility surface still exposes `ThreadId` / `conversation_id`. Do not alter in Track B; v1 compatibility requires a separate decision. |
| `datax-rs/app-server-protocol/src/protocol/thread_history.rs` | `Deferred` | Track B classification: maps raw response and history events that still carry `turn_id` internally. Requires app-server protocol and replay compatibility review before renaming. |
| `datax-rs/app-server-protocol/schema/**/*` | `Deferred` | Track B classification: generated schema/type artifacts still include `ThreadId` and historical compatibility names. Must be regenerated from source only after an approved protocol rename slice. |
| `datax-rs/state/migrations/**/*` | `Deferred` | Track B classification: persisted SQLite schema and migration names still use `thread_id`. Do not rewrite historical migrations for cosmetic identity cleanup. |
| `datax-rs/rollout-trace/**/*` | `Deferred` | Track B classification: rollout trace model/reducer names include `codex_turn` / `codex_turns`; defer to a trace-schema migration slice. |
| `datax-rs/core/src/codex_thread.rs` and related `datax-rs/core/src/session/**/*` / `datax-rs/core/src/tasks/**/*` files | `Deferred` | Track B classification: internal core execution/session owner names. Rename only after app-server, TUI, rollout, and downstream Codex compatibility boundaries are mapped. |
| `datax-rs/tui/**/*` | `Deferred` | Track B classification: TUI local state still uses `thread_id` / `turn_id` because it bridges app-server chat/interaction payloads to inherited local session structures. Rename only with focused TUI state tests and snapshot review. |
| `datax-rs/**/snapshots/*codex*.snap` and `datax-rs/**/snapshots/*codex*.snap.new` | `Completed` | Track B audit: existing generated snapshot identity debt found in `core`, `cli`, and pending `tui` snapshots. `datax_tui__...gpt5_codex...` snapshots are model-slug references and are excluded from product identity cleanup. |
| `.github/workflows/rust-release.yml` | `Pending` | Isolated `codex-sdk` track. Do not fix until user approves the SDK/package track. |
| `datax-cli/scripts/build_npm_package.py` | `Pending` | Isolated `codex-sdk` track. Do not fix until user approves the SDK/package track. |
| `sdk/typescript/package.json` | `Pending` | Isolated `codex-sdk` track. Do not fix until user approves the SDK/package track. |
| `sdk/typescript/src/exec.ts` | `Pending` | Isolated `codex-sdk` track. Do not fix until user approves the SDK/package track. |
| `sdk/typescript/samples/*.ts` | `Pending` | Isolated `codex-sdk` track. Do not fix until user approves the SDK/package track. |
| `datax-rs/app-server*/**/*` | `Pending` | Track C: downstream Codex artifact note. Candidate app-server launch, protocol, transport, schema, and test-client artifacts to record for Phase 2; do not implement adapter/runtime wiring in Phase 1. |
| `datax-rs/rmcp-client/**/*` | `Pending` | Track C: downstream/runtime transport candidate. Record for Phase 2 if relevant; no Phase 1 adapter implementation. |
| `datax-rs/backend-client/**/*` | `Pending` | Track C: downstream/client candidate. Record for Phase 2 if relevant; no Phase 1 adapter implementation. |
| `scripts/**/*` | `Pending` | Track C: process-management and helper script candidates. Record for Phase 2 when related to downstream Codex runtime; otherwise handle under Track A. |

## Track A: `codex-rs` to `datax-rs` Path Rename

Track A is limited to the Rust workspace directory path and references that point at that path. It does not rename internal Rust modules, user-facing protocol fields, SDK package names, downstream Codex runtime artifacts, or protected sandbox identifiers. The goal is to remove the Datax-owned `codex-rs` source path from active repository tooling while preserving behavior.

Track A proceeded in this order:

1. Inventory `codex-rs` path references and classify the affected groups.
2. Update this file inventory with the Track A classification.
3. Move the Rust workspace path with `git mv`.
4. Update writable repository references from `codex-rs` to `datax-rs`.
5. Leave `.codex/` local metadata unmodified because it is read-only in this workspace and not active project source.
6. User runs the validation commands below and provides the output.

### Track A Inventory Commands

These commands were used to define the Track A scope before implementation. They are retained as provenance for the implementation; do not rerun them as the post-change validation.

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch

    test -d codex-rs && echo "codex-rs exists" || echo "codex-rs missing"
    test -d datax-rs && echo "datax-rs exists" || echo "datax-rs missing"

    rg -l --hidden "codex-rs" . \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!codex-rs/target/**' \
      --glob '!**/*.snap.new' \
      | sort > /tmp/datax_phase1_7_track_a_codex_rs_files.txt

    wc -l /tmp/datax_phase1_7_track_a_codex_rs_files.txt
    sed -n '1,260p' /tmp/datax_phase1_7_track_a_codex_rs_files.txt

    awk -F/ '
      {
        if ($1 == ".") {
          key = $2
        } else {
          key = $1
        }
        counts[key]++
      }
      END {
        for (key in counts) {
          print counts[key], key
        }
      }
    ' /tmp/datax_phase1_7_track_a_codex_rs_files.txt | sort -nr

    rg -n --hidden "codex-rs|//codex-rs|\\./codex-rs|/codex-rs" \
      AGENTS.md \
      justfile \
      package.json \
      pnpm-workspace.yaml \
      pnpm-lock.yaml \
      MODULE.bazel \
      defs.bzl \
      flake.nix \
      .bazelrc \
      .bazelignore \
      .gitattributes \
      .devcontainer \
      .github \
      bazel \
      datax-cli \
      docs \
      scripts \
      sdk \
      third_party \
      tools \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!codex-rs/target/**' \
      --glob '!**/*.snap.new'

### Track A Implementation Commands

These implementation commands have been executed in this milestone.

The directory move must use Git so history follows the source tree:

    cd /home/mbellary/wsl/projects/datax
    git mv codex-rs datax-rs

After the directory move, Codex updated active path references from `codex-rs` to `datax-rs` in writable repository files. This update did not touch `codex-sdk` package names, downstream Codex app-server compatibility names, or protected sandbox identifiers.

### Track A Post-Change Validation Commands

After the implementation edit is complete, the user runs these commands and provides the output. These commands are intended to catch path drift before any expensive build or test command is attempted.

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch

    test -d codex-rs && echo "codex-rs exists" || echo "codex-rs missing"
    test -d datax-rs && echo "datax-rs exists" || echo "datax-rs missing"

    rg -n --hidden "codex-rs|//codex-rs|\\./codex-rs|/codex-rs" . \
      --glob '!.git/**' \
      --glob '!.codex/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      > /tmp/datax_phase1_7_track_a_remaining_codex_rs_refs.txt

    wc -l /tmp/datax_phase1_7_track_a_remaining_codex_rs_refs.txt
    sed -n '1,260p' /tmp/datax_phase1_7_track_a_remaining_codex_rs_refs.txt

    rg -n --hidden "datax-rs" \
      AGENTS.md \
      justfile \
      package.json \
      pnpm-workspace.yaml \
      MODULE.bazel \
      defs.bzl \
      flake.nix \
      .bazelrc \
      .bazelignore \
      .gitattributes \
      .devcontainer \
      .github \
      bazel \
      datax-cli \
      scripts \
      sdk \
      third_party \
      tools \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new'

    git diff --check -- . ':(exclude)**/*.snap'

## Track B: Internal-Name and Snapshot Identity Audit

Track B is an audit and classification track. It does not rename source symbols, generated schema names, SDK names, persisted database columns, or snapshot filenames. The purpose is to prevent Phase 1 from closing with unknown Codex-era internal-name debt while also avoiding a risky mechanical rewrite across protocol, persistence, SDK, TUI, app-server, rollout, and core session boundaries.

The Track B audit concluded:

- `ThreadId`, `thread_id`, and `turn_id` are cross-cutting compatibility and persistence terms, not just local implementation names.
- `codex_thread`, `codex_turn`, and `codex_turns` are concentrated in core execution/session and rollout trace code, but they still cross module and schema boundaries.
- Existing and pending snapshot filenames with `codex_core__...`, `codex_tui__...`, or `codex__doctor__...` are generated from crate/module/test identity and should be handled with the specific crate/module rename that produces them.
- Snapshot names containing `gpt5_codex` or `gpt5_codex_mini` are model-slug references and are accepted exceptions, not Datax product identity debt.

### Track B Inventory Commands

These commands were used to define the Track B scope. They are retained as provenance and can be rerun by the user after this commit to confirm the audit baseline.

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch

    rg -l --hidden "\b(thread_id|turn_id|ThreadId|TurnId|codex_thread|codex_turn|codex_turns)\b" . \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      | awk -F/ '{ if ($1 == ".") print $2; else print $1 }' \
      | sort \
      | uniq -c \
      | sort -nr

    rg -l --hidden "\b(thread_id|turn_id|ThreadId|TurnId|codex_thread|codex_turn|codex_turns)\b" datax-rs \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      | awk -F/ '{ print $2 }' \
      | sort \
      | uniq -c \
      | sort -nr

    rg -l --hidden "\b(codex_thread|codex_turn|codex_turns)\b" datax-rs sdk docs \
      --glob '!**/*.snap.new'

    rg -l --hidden "\bThreadId\b" \
      datax-rs/app-server-protocol/schema \
      datax-rs/app-server-protocol/src \
      datax-rs/protocol/src \
      sdk \
      --glob '!**/*.snap.new'

    find datax-rs -path '*/snapshots/*codex*.snap' -print
    find datax-rs -path '*/snapshots/*codex*.snap.new' -print

### Track B Post-Change Status Commands

After this Track B audit commit is pulled, the user runs these commands and provides the output. These are static status commands only; they do not build, format, fix, generate schemas, or run tests.

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch

    rg -l --hidden "\b(thread_id|turn_id|ThreadId|TurnId|codex_thread|codex_turn|codex_turns)\b" . \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      | awk -F/ '{ if ($1 == ".") print $2; else print $1 }' \
      | sort \
      | uniq -c \
      | sort -nr

    rg -l --hidden "\b(thread_id|turn_id|ThreadId|TurnId|codex_thread|codex_turn|codex_turns)\b" datax-rs \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      | awk -F/ '{ print $2 }' \
      | sort \
      | uniq -c \
      | sort -nr

    rg -l --hidden "\b(codex_thread|codex_turn|codex_turns)\b" datax-rs sdk docs \
      --glob '!**/*.snap.new'

    find datax-rs -path '*/snapshots/*codex*.snap' -print
    find datax-rs -path '*/snapshots/*codex*.snap.new' -print

Expected result: output is allowed. Any output should match the classified Track B categories above. New public, wire, generated, CLI-visible, fixture, documentation, or snapshot identity matches outside these categories must be added to this ExecPlan before Phase 1 freeze is accepted.

## Plan of Work

The implementation started as metadata-only, but Phase 1.7 now also owns remaining Datax-owned identity cleanup. Inventory every `codex-rs` path reference before modifying files, then rename the top-level Rust source directory to `datax-rs` and update build, CI, package, schema, Bazel, docs, and helper-script references in the same checkpoint. Inventory internal Codex-era identifiers before modifying files; rename public, wire, generated, CLI-visible, fixture, snapshot identity, and documentation occurrences, and classify any internal-only names that remain. Snapshot IDs and pending snapshot filenames such as `codex_tui__...` and `codex_core__...` belong to Track B because they are generated from crate/module/test identity, not from the Track A Rust workspace path. Record downstream Codex artifacts discovered during these scans for Phase 2 adapter/runtime planning, but do not implement the adapter or wire Datax to a downstream Codex app-server in Phase 1. Do not use this as permission to move unrelated Datax-owned crates or redesign the repository layout. Do not run format, build, generation, fix, or test commands unless the user asks for that exact command.

The final freeze report will use the concrete checklist as the command source. This ExecPlan tracks implementation of the checklist and the pre-freeze metadata cleanup.

## Validation Matrix

| Command | Working Directory | Required Before Merge | Status | Notes |
| --- | --- | --- | --- | --- |
| `git diff --check -- . ':(exclude)**/*.snap'` | repository root | Yes | `Completed` | Static whitespace check returned no output when terminal snapshot fixtures are excluded. Plain `git diff --check` flags intentional trailing spaces on changed terminal snapshot lines. |
| `rg -n "codex-hooks|codex_package|--package codex|CODEX_VERSION|Test Codex package builder|Codex package|codex-cli/README" package.json .gitignore .github scripts datax-cli justfile` | repository root | Yes | `Failed` | Returned `.github/workflows/rust-release.yml:1200: --package codex-sdk`; inspection showed this is an active SDK/package rename gap, not an accepted exception. |
| `rg -n "Codex App|Codex Web|Codex team|openai/codex|codex/discussions|codex-action|codex-label|codex-deduplicate" .github/ISSUE_TEMPLATE .github/workflows .github/actions .github/pull_request_template.md .github/CODEOWNERS` | repository root | Yes | `Completed` | Active issue template and CODEOWNERS matches are removed; remaining matches are upstream-only workflow guards, external action references, or artifact sources requiring freeze exception classification. |
| `rg -n "\b(Codex|codex|CODEX)\b" README.md docs datax-cli package.json .github --glob '!docs/plans/datax_migration_phase1_*_*/**'` | repository root | Yes | `Completed` | Broad inventory still returns documented/deferred categories, including `datax-rs`, external docs links, upstream-only workflows, and `codex-sdk`. |
| `rg -l --hidden "\b(thread_id\|turn_id\|ThreadId\|TurnId\|codex_thread\|codex_turn\|codex_turns)\b" . ...` | repository root | Yes | `Completed` | Track B post-change root grouping returned 527 `datax-rs`, 25 `sdk`, and 6 `docs` files, matching the classified baseline. |
| `rg -l --hidden "\b(thread_id\|turn_id\|ThreadId\|TurnId\|codex_thread\|codex_turn\|codex_turns)\b" datax-rs ...` | repository root | Yes | `Completed` | Track B post-change `datax-rs` grouping matched the classified crate/module baseline. |
| `rg -l --hidden "\b(codex_thread\|codex_turn\|codex_turns)\b" datax-rs sdk docs ...` | repository root | Yes | `Completed` | Track B post-change narrow Codex-era internal-name list matched the deferred core, rollout-trace, app-server, and historical-plan categories. |
| `find datax-rs -path '*/snapshots/*codex*.snap' -print` | repository root | Yes | `Completed` | Existing snapshot identity output matched the classified baseline. Model-slug snapshots containing `gpt5_codex` are accepted exceptions. |
| `find datax-rs -path '*/snapshots/*codex*.snap.new' -print` | repository root | Yes | `Completed` | Pending snapshot identity output matched the classified baseline. These files remain unaccepted pending a crate/module identity rename track. |
| `just fmt` | repository root | Yes | `Deferred` | User-run only. |
| `just write-config-schema` | repository root | Yes | `Deferred` | User-run only. |
| `just write-app-server-schema` | repository root | Yes | `Deferred` | User-run only. |
| `just write-app-server-schema --experimental` | repository root | Yes | `Deferred` | User-run only. |
| `just write-hooks-schema` | repository root | Yes | `Deferred` | User-run only. |
| `cargo build` | repository root | Yes | `Deferred` | User-run only. |
| `just test -p datax-cli` | repository root | Yes | `Deferred` | User-run only. |
| `just test` | repository root | Yes | `Deferred` | User-run full suite only. |

## Validation and Acceptance

From the repository root, run the whitespace check and expect no output:

    cd /home/mbellary/wsl/projects/datax
    git diff --check -- . ':(exclude)**/*.snap'

From the repository root, rerun the focused package metadata search:

    cd /home/mbellary/wsl/projects/datax
    rg -n "codex-hooks|codex_package|--package codex|CODEX_VERSION|Test Codex package builder|Codex package|codex-cli/README" package.json .gitignore .github scripts datax-cli justfile

Expected result: no active stale Datax-owned metadata remains. Any matches must be classified in this ExecPlan before freeze.

From the repository root, rerun the focused GitHub metadata search:

    cd /home/mbellary/wsl/projects/datax
    rg -n "Codex App|Codex Web|Codex team|openai/codex|codex/discussions|codex-action|codex-label|codex-deduplicate" .github/ISSUE_TEMPLATE .github/workflows .github/actions .github/pull_request_template.md .github/CODEOWNERS

Expected result: no active issue template or CODEOWNERS matches remain. Upstream-only workflow guards and external action names may remain only if documented in the final exception register.

From the repository root, run the broad final inventory:

    cd /home/mbellary/wsl/projects/datax
    rg -n "\b(Codex|codex|CODEX)\b" README.md docs datax-cli package.json .github --glob '!docs/plans/datax_migration_phase1_*_*/**'

Expected result: output may remain, but each match must be categorized before Phase 1 freeze is accepted.

The full freeze command list is in `docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md`. The user runs those commands and provides output; Codex records results in this plan and the final freeze report.

## Idempotence and Recovery

The metadata edits are safe to reapply because they are deterministic text changes. If a search finds a new active stale reference, update the relevant file and rerun only the focused static search. If a generated or test command later changes files, stop freeze validation, review the diff, commit the accepted change, and restart the affected checklist section.

Do not delete historical plans or protected identifiers to make a search count smaller. The goal is accurate classification, not artificial zero matches.

## Artifacts and Notes

The user-provided focused search showed these active stale matches before cleanup:

    package.json:8: "write-hooks-schema": "cargo run --manifest-path ./datax-rs/Cargo.toml -p codex-hooks --bin write_hooks_schema_fixtures"
    .github/ISSUE_TEMPLATE/1-codex-app.yml:1:name: Codex App Bug
    .github/ISSUE_TEMPLATE/4-bug-report.yml:2:description: Report an issue in Codex Web, integrations, or other Codex components
    .github/ISSUE_TEMPLATE/5-feature-request.yml:2:description: Propose a new feature for Codex
    .github/ISSUE_TEMPLATE/6-docs-issue.yml:8:Thank you for submitting a documentation request. It helps make Codex better.
    .github/CODEOWNERS:2:/datax-rs/core/ @openai/codex-core-agent-team
    .github/workflows/ci.yml:29:Test Codex package builder

These are Datax-owned active metadata references and were fixed in this milestone.
