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
- [x] (2026-07-08 00:00Z) Ran lightweight static validation; `git diff --check` passed and focused stale package metadata search now returns only the pending `codex-sdk` classification.
- [x] (2026-07-08 00:00Z) Created GitHub issue #13 for Phase 1.7.
- [x] (2026-07-08 00:00Z) Created draft PR #14 for Phase 1.7.
- [x] Run allowed static validation only; build, format, generation, fix, and test commands remain user-run.
- [ ] Record final validation evidence from the user and close out the freeze report.

## Surprises & Discoveries

- Observation: The broad remaining-Codex inventory found thousands of matches, but that count alone is not actionable because it includes deferred `codex-rs` paths, external service contracts, protected sandbox identifiers, and historical migration plans.
  Evidence: The user-run inventory command reported `9081 /tmp/datax_phase1_remaining_codex_refs.txt`.
- Observation: The broad inventory also exposed active metadata that is not safe to leave as a generic exception.
  Evidence: `package.json` still used `-p codex-hooks`; `.github/ISSUE_TEMPLATE/*` still presented Codex issue templates; `.github/CODEOWNERS` still assigned OpenAI Codex owners; and `.github/workflows/ci.yml` still referenced the removed `scripts/codex_package` path.
- Observation: The repository still uses `codex-rs` as the Rust workspace directory.
  Evidence: root `justfile`, GitHub workflows, scripts, and generated paths refer to `codex-rs`; earlier milestones documented filesystem directory rename as deferred.
- Observation: The focused package metadata check is now narrowed to `codex-sdk`.
  Evidence: `rg -n "codex-hooks|codex_package|--package codex|CODEX_VERSION|Test Codex package builder|Codex package|codex-cli/README" package.json .gitignore .github scripts datax-cli justfile` returns only `.github/workflows/rust-release.yml:1200: --package codex-sdk`.

## Decision Log

- Decision: Treat Phase 1.7 as a freeze checkpoint plus active metadata cleanup, not as a broad Rust or filesystem rename.
  Rationale: Previous phases intentionally deferred internal implementation names and the `codex-rs` directory. Reopening those changes during freeze would increase risk and invalidate already-passed tests.
  Date/Author: 2026-07-08 / Codex.
- Decision: Do not run `just fmt`, `cargo build`, `cargo check`, `just fix`, `just test`, schema generation, or full smoke commands unless the user explicitly authorizes that exact command.
  Rationale: The user will run expensive commands and paste output. The freeze checklist records exact commands and evidence expectations.
  Date/Author: 2026-07-08 / Codex.
- Decision: Remove OpenAI Codex CODEOWNERS assignments rather than inventing replacement owners.
  Rationale: Datax repository-local owners are not known in this thread. An empty CODEOWNERS file with a Datax migration note is more accurate than retaining OpenAI Codex team ownership.
  Date/Author: 2026-07-08 / Codex.
- Decision: Keep `codex-rs` path references as explicit migration exceptions for Phase 1.7.
  Rationale: The directory rename is deferred, and changing it would affect Cargo, Bazel, scripts, workflows, and many validation commands beyond freeze metadata cleanup.
  Date/Author: 2026-07-08 / Codex.

## Outcomes & Retrospective

In progress. The active metadata cleanup has been implemented, Phase 1.7 issue #13 has been created, and draft PR #14 has been opened. Final outcomes will be recorded after user-run freeze checklist evidence is complete.

## Context and Orientation

The repository root is `/home/mbellary/wsl/projects/datax`. Rust code still lives under `codex-rs`; that path is a documented migration exception. The product identity is Datax, and active user-facing repository surfaces should not instruct users to file Codex issues, run Codex packages, or rely on OpenAI Codex ownership metadata.

Phase 1.7 starts after Phase 1.6 has been merged into `main`. The user confirmed Phase 1.6 tests passed and updated local `main`. The work in this plan happens on branch `datax/migration-phase1-7-freeze-report`.

The concrete freeze checklist lives at `docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md`. That checklist is runner-facing: it lists exact commands for the user to execute and output to paste back into the thread.

The milestone GitHub issue is #13: `https://github.com/mbellary/datax/issues/13`.

The milestone draft pull request is #14: `https://github.com/mbellary/datax/pull/14`.

## Rename Exception Register

The following Codex-shaped references may remain after this milestone if they are recorded in the final freeze report:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`, `CODEX_SANDBOX_ENV_VAR`, `CODEX_SANDBOX_NETWORK_DISABLED`, and `CODEX_SANDBOX`. These are protected sandbox identifiers.
- The `codex-rs` directory path and Bazel labels under `//codex-rs/...`. Filesystem rename is deferred.
- External service paths such as `https://chatgpt.com/backend-api/codex`.
- External model slugs such as `gpt-5-codex` when used as model examples.
- Upstream artifact sources such as OpenAI Codex release URLs when still required to fetch externally built artifacts.
- Historical migration plans that discuss Codex-to-Datax migration decisions.
- `codex-sdk` package names if classified as an external/upstream SDK artifact in the final freeze report.

## Public Surface Checklist

This milestone touches active GitHub issue templates, root package scripts, active CI labels and package staging arguments, CODEOWNERS metadata, and Phase 1 test-plan documentation.

This milestone does not touch Rust runtime behavior, app-server protocol source, generated schemas, persistence formats, CLI arguments, npm launcher runtime code, or snapshots.

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
| `.github/workflows/rust-release.yml` | `Not Required` | Inspected because `codex-sdk` remains. This needs final exception classification as external/upstream SDK or a later dedicated package rename. |
| `datax-cli/scripts/build_npm_package.py` | `Not Required` | Inspected because `codex-sdk` remains. This needs final exception classification as external/upstream SDK or a later dedicated package rename. |
| `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` | `Completed` | Links the high-level Test Plan to the concrete freeze checklist. |
| `docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md` | `Completed` | Concrete Phase 1.7 runner-facing freeze checklist. |
| `docs/plans/datax_migration_phase1_7_freeze_report/github_issue.md` | `Completed` | Records the GitHub issue body for issue #13. |
| `docs/plans/datax_migration_phase1_7_freeze_report/migration_freeze_checkpoint_execplan.md` | `Completed` | This living ExecPlan. |
| `docs/CLA.md` | `Completed` | Project name in the CLA text now says Datax CLI. Legal grant terms were not otherwise rewritten. |
| `docs/contributing.md` | `Completed` | Contributor guidance now uses Datax maintainer and Datax CLI wording. |
| `package.json` | `Completed` | Root hook-schema script now targets `datax-hooks`. |

## Plan of Work

The implementation is intentionally metadata-only. Update the files listed as `Completed` in the inventory and leave runtime Rust code untouched. After edits, rerun focused static searches to confirm the active metadata cleanup removed the previously reported stale names. Do not run format, build, generation, fix, or test commands unless the user asks for that exact command.

The final freeze report will use the concrete checklist as the command source. This ExecPlan tracks implementation of the checklist and the pre-freeze metadata cleanup.

## Validation Matrix

| Command | Working Directory | Required Before Merge | Status | Notes |
| --- | --- | --- | --- | --- |
| `git diff --check` | repository root | Yes | `Completed` | Static whitespace check returned no output. |
| `rg -n "codex-hooks|codex_package|--package codex|CODEX_VERSION|Test Codex package builder|Codex package|codex-cli/README" package.json .gitignore .github scripts datax-cli justfile` | repository root | Yes | `Completed` | Returned only `--package codex-sdk`, which remains pending final exception classification. |
| `rg -n "Codex App|Codex Web|Codex team|openai/codex|codex/discussions|codex-action|codex-label|codex-deduplicate" .github/ISSUE_TEMPLATE .github/workflows .github/actions .github/pull_request_template.md .github/CODEOWNERS` | repository root | Yes | `Completed` | Active issue template and CODEOWNERS matches are removed; remaining matches are upstream-only workflow guards, external action references, or artifact sources requiring freeze exception classification. |
| `rg -n "\b(Codex|codex|CODEX)\b" README.md docs datax-cli package.json .github --glob '!docs/plans/datax_migration_phase1_*_*/**'` | repository root | Yes | `Completed` | Broad inventory still returns documented/deferred categories, including `codex-rs`, external docs links, upstream-only workflows, and `codex-sdk`. |
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
    git diff --check

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

    package.json:8: "write-hooks-schema": "cargo run --manifest-path ./codex-rs/Cargo.toml -p codex-hooks --bin write_hooks_schema_fixtures"
    .github/ISSUE_TEMPLATE/1-codex-app.yml:1:name: Codex App Bug
    .github/ISSUE_TEMPLATE/4-bug-report.yml:2:description: Report an issue in Codex Web, integrations, or other Codex components
    .github/ISSUE_TEMPLATE/5-feature-request.yml:2:description: Propose a new feature for Codex
    .github/ISSUE_TEMPLATE/6-docs-issue.yml:8:Thank you for submitting a documentation request. It helps make Codex better.
    .github/CODEOWNERS:2:/codex-rs/core/ @openai/codex-core-agent-team
    .github/workflows/ci.yml:29:Test Codex package builder

These are Datax-owned active metadata references and were fixed in this milestone.
