# Phase 1.3 Rust Workspace Stabilization

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` and the migration execution model in `docs/plans/Recommended-Datax-Migration-Execution-Model.md`.

## Purpose / Big Picture

Phase 1.3 makes the Rust workspace internally consistent with the Datax product identity. After this milestone, Rust package names, dependency names, crate identifiers, Bazel crate labels, and the Cargo lockfile should agree on Datax naming. This does not add product behavior. A reviewer can see the work by inspecting Rust manifests and source imports, and later by running the explicit deferred validation commands recorded below.

## Progress

- [x] (2026-07-06T12:29:27Z) Confirmed `main` is clean after the Phase 1.2 merge and created branch `datax/migration-phase1-3-rust-workspace`.
- [x] (2026-07-06T12:29:27Z) Read the Phase 1 provisional plan, recommended execution model, and `PLANS.md`.
- [x] (2026-07-06T12:29:27Z) Identified Phase 1.3 as Rust workspace stabilization: crate packages, dependency names, crate identifiers, Bazel labels, lockfiles, and generated metadata.
- [x] (2026-07-06T12:29:27Z) Performed the initial dependency census with `rg`; the workspace contains many Rust source references to `codex_*`, so metadata and imports must be updated together.
- [ ] Create the GitHub issue and draft pull request for Phase 1.3.
- [ ] Rename Rust package names and workspace dependency keys from `codex-*` to `datax-*`.
- [ ] Rename Rust crate identifiers from `codex_*` to `datax_*` where they refer to renamed workspace crates.
- [ ] Update Bazel crate rule names and `crate_name` attributes to match Datax naming.
- [ ] Refresh `codex-rs/Cargo.lock` and any Bazel lock metadata required by the dependency rename.
- [ ] Run formatter only, and record all build/test commands as deferred for the post-implementation migration test pass.
- [ ] Update this ExecPlan with final inventory status, decisions, validation command status, and outcome notes.

## Surprises & Discoveries

- Observation: The Rust crate rename cannot be limited to `Cargo.toml` files because source imports use generated crate identifiers such as `codex_core`, `codex_protocol`, and `codex_utils_*`.
  Evidence: `rg -l 'codex[_-][A-Za-z0-9_-]+' codex-rs --glob '*.rs' --glob 'Cargo.toml' --glob 'Cargo.lock' --glob 'BUILD.bazel' --glob '*.bzl' | wc -l` returned `2008`.

- Observation: The repository instructions still describe existing upstream crate names as `codex-*`, but the approved migration plan explicitly requires `datax-*` crate package names in Phase 1.3.
  Evidence: `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` says “Rename crate packages from `codex-*` to `datax-*`.”

## Decision Log

- Decision: Treat Phase 1.3 as a mechanical Rust workspace rename, not a semantic cleanup.
  Rationale: The milestone is intended to remove rename drift after Phase 1.2 while preserving behavior. Semantic concepts such as Thread, Turn, and Item remain in later phases.
  Date/Author: 2026-07-06 / Codex

- Decision: Preserve the top-level `codex-rs` directory name in this milestone.
  Rationale: The current migration plan names Rust crate packages, imports, lockfiles, and Bazel metadata as Phase 1.3 scope, while filesystem/persistence paths are handled later. Renaming the root Rust directory would require broad script and documentation churn outside the compile stabilization surface.
  Date/Author: 2026-07-06 / Codex

- Decision: Do not rename or modify protected sandbox identifiers.
  Rationale: The repository instructions explicitly protect `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`; these are environment-contract names, not product identity.
  Date/Author: 2026-07-06 / Codex

## Outcomes & Retrospective

This section will be completed when the milestone branch is ready for review. Expected outcome: Rust workspace metadata and source-level crate references use Datax naming consistently, with tests and builds documented as deferred commands for the post-implementation migration validation pass.

## Context and Orientation

The Rust workspace lives under `codex-rs/`. Its root manifest, `codex-rs/Cargo.toml`, declares workspace members and central dependency keys. Each crate has a `Cargo.toml` package name, and many crates also define a Rust library or binary name with a `name = "codex_..."` or `name = "codex-..."` entry. Rust source files import crates by their library names, so changing `codex-core` to `datax-core` also requires source imports such as `codex_core::Config` to become `datax_core::Config` when the library crate is renamed.

Bazel metadata appears in `BUILD.bazel` files and uses the repository helper rule `codex_rust_crate`. The helper rule name itself is build infrastructure and can remain for now; the crate targets and `crate_name` values inside those rules must match the renamed Rust crates.

`codex-rs/Cargo.lock` records resolved package names. It must be refreshed or mechanically updated after manifest names change so Cargo no longer records internal packages as `codex-*`.

This phase does not rename app-server public concepts from Thread/Turn/Item to Chat/Interaction/Message. That is Phase 1.4. It also does not rename persisted local state, fixtures, or snapshots unless a crate rename directly requires an update; those are Phase 1.5.

## Baseline

Starting branch: `main`.

Milestone branch: `datax/migration-phase1-3-rust-workspace`.

Current repository state: clean at branch creation.

Known pre-existing validation policy: long builds and tests are staged. The commands below must be documented exactly, but execution is deferred until the user runs the migration validation pass unless the user explicitly asks to run a command during this milestone.

## File Inventory

The table below tracks files and file sets that belong to Phase 1.3. Rows marked `Pending` must become `Completed`, `Failed`, or `Not Required` before this milestone exits. For very large source sets, the row records the discovery command that identifies the exact file list; the command output is part of the inventory process and must be re-run after implementation to ensure there are no unexpected remaining references.

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/datax_migration_phase1_3_rust_workspace/rust_workspace_stabilization_execplan.md` | `In-Progress` | Living ExecPlan for Phase 1.3. |
| `codex-rs/Cargo.toml` | `Pending` | Root workspace package/dependency keys must rename internal `codex-*` dependencies to `datax-*`; workspace member paths stay stable unless a crate directory is intentionally renamed. |
| `codex-rs/Cargo.lock` | `Pending` | Lockfile must record internal package names as `datax-*` after manifest updates. |
| `codex-rs/**/Cargo.toml` | `Pending` | All Rust crate manifests with `name = "codex-*"`, dependency keys named `codex-*`, or `package = "codex-*"` belong to this phase. Discovery command: `rg -l '^name = "codex-|^codex-[a-z0-9_-]+ = \{|package = "codex-' codex-rs/Cargo.toml codex-rs/*/Cargo.toml codex-rs/*/*/Cargo.toml codex-rs/*/*/*/Cargo.toml`. |
| `codex-rs/**/BUILD.bazel` | `Pending` | Bazel crate target names and `crate_name` values must match the renamed package and Rust crate identifiers. The `codex_rust_crate` helper rule is infrastructure and remains an exception in this milestone. |
| `codex-rs/**/*.rs` | `Pending` | Rust source files importing or referring to renamed workspace crate identifiers such as `codex_core`, `codex_protocol`, and `codex_utils_*` must be updated to `datax_*`. Protected uppercase sandbox identifiers must not be changed. Discovery command: `rg -l 'codex_[A-Za-z0-9_]+' codex-rs --glob '*.rs'`. |
| `codex-rs/**/*.bzl` | `Pending` | Bazel/Starlark files are inspected for crate labels and package names. Infrastructure helper names can remain if they are not package identities. |
| `MODULE.bazel` | `Pending` | Root Bazel module metadata may contain crate package references generated from Rust metadata; inspect and update if required. |
| `MODULE.bazel.lock` | `Pending` | Bazel lock metadata may require refresh if Rust dependency metadata changes. |
| `defs.bzl` | `Pending` | Inspect helper-rule names and generated crate mapping. Rename only if it represents a public package or crate identity; otherwise document as an exception. |
| `datax-cli/package.json` | `Pending` | Inspect for Rust binary/package references introduced by Phase 1.2. Update if it still points at renamed Rust crate/package outputs. |
| `README.md` | `Pending` | Inspect only for Rust workspace command references affected by package renames. Do not expand product documentation. |
| `docs/install.md` | `Pending` | Inspect only for Rust workspace command references affected by package renames. Do not expand product documentation. |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` references | `Not Required` | Protected sandbox identifiers are intentionally excluded from all Phase 1 rename operations. |
| `Thread`, `Turn`, and `Item` protocol types | `Not Required` | App-server model rename is Phase 1.4 and must not be mixed into this branch. |
| Persistence directories and fixture snapshots | `Not Required` | Persistence, fixtures, and snapshots are Phase 1.5 unless a crate rename directly forces a checked-in generated artifact change. |

## Rename Exception Register

The following Codex/codex references may remain after Phase 1.3:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`, because they are protected sandbox contracts.
- The top-level `codex-rs/` directory path, because this milestone stabilizes Rust workspace metadata and imports without moving the Rust workspace root.
- The Bazel helper rule name `codex_rust_crate`, if changing it would be broad build-infrastructure churn not required for package or crate identity correctness.
- Upstream provenance, license, and historical references that do not represent active Datax product identity.
- App-server Thread/Turn/Item names, because Phase 1.4 owns the public protocol model rename.

## Public Surface Checklist

This milestone touches package and crate names, Rust source imports, Bazel labels, lockfiles, and potentially generated build metadata. It does not intentionally touch CLI argument behavior, config key semantics, app-server protocol methods, persisted session formats, UI layouts, or snapshots. If implementation discovers a generated artifact that encodes renamed crate names, update it in this milestone and record the command used.

## Dependency Order

First, update package names and dependency keys in `Cargo.toml` files so Cargo has a single source of truth for internal package identity. Second, update Rust library and binary crate identifiers so source imports have valid targets. Third, mechanically update Rust source imports and path references from `codex_*` to `datax_*` where the referenced crate was renamed. Fourth, update Bazel `name` and `crate_name` values to match the Cargo metadata. Fifth, refresh `codex-rs/Cargo.lock` and any Bazel lock metadata. Last, run static searches to identify intentional exceptions and update this plan.

## Plan of Work

Create the GitHub issue and draft PR immediately after the initial ExecPlan commit so all implementation updates attach to the Phase 1.3 milestone.

Use mechanical transformations for package and crate identity strings. Review the resulting diff in bands: manifests first, source imports second, Bazel metadata third, lockfiles last. Do not manually rename the protected sandbox identifiers. Do not convert Thread/Turn/Item protocol types in this milestone.

After edits, run `just fmt` from `codex-rs`. Per the staged validation policy, do not run long build or test commands unless the user explicitly requests them. Record all commands in the Validation Matrix and Validation and Acceptance sections with status `Deferred`.

## Concrete Steps

From the repository root, confirm the branch and clean start:

    git status --short --branch

Create the GitHub issue and draft PR after committing this ExecPlan:

    gh issue create --title "Phase 1.3: Rust workspace stabilization" --body-file docs/plans/datax_migration_phase1_3_rust_workspace/github_issue.md
    gh pr create --draft --title "Phase 1.3: Rust workspace stabilization" --body-file docs/plans/datax_migration_phase1_3_rust_workspace/pull_request.md

Use `rg` to inventory all files that still contain crate-style Codex names:

    rg -l 'codex[_-][A-Za-z0-9_-]+' codex-rs --glob '*.rs' --glob 'Cargo.toml' --glob 'Cargo.lock' --glob 'BUILD.bazel' --glob '*.bzl'

Use `rg` to inventory manifests that have package or dependency names to update:

    rg -l '^name = "codex-|^codex-[a-z0-9_-]+ = \{|package = "codex-' codex-rs/Cargo.toml codex-rs/*/Cargo.toml codex-rs/*/*/Cargo.toml codex-rs/*/*/*/Cargo.toml

After implementation, run formatter from `codex-rs`:

    just fmt

## Validation Matrix

| Command | Working Directory | Status | Expected Result |
| --- | --- | --- | --- |
| `git diff --check` | repository root | `Deferred` | No whitespace errors. |
| `rg -n 'Data[X]' .` | repository root | `Deferred` | No forbidden mixed-case spelling. |
| `rg -n 'name = "codex-|package = "codex-|^codex-[a-z0-9_-]+ = \{' codex-rs --glob 'Cargo.toml'` | repository root | `Deferred` | No internal Rust package/dependency names remain except documented external/provenance exceptions. |
| `rg -n 'crate_name = "codex_|use codex_|::codex_|codex_[A-Za-z0-9_]*::' codex-rs --glob '*.rs' --glob 'BUILD.bazel'` | repository root | `Deferred` | No renamed Rust crate identifiers remain except protected or documented exceptions. |
| `just fmt` | `codex-rs` | `Deferred` | Rust formatting completes successfully. |
| `cargo generate-lockfile` | `codex-rs` | `Deferred` | `codex-rs/Cargo.lock` refreshes with renamed internal package names. |
| `just bazel-lock-update` | repository root | `Deferred` | Bazel lock metadata refreshes if Cargo/Bazel dependency metadata changed. |
| `just bazel-lock-check` | repository root | `Deferred` | Bazel lock metadata has no drift. |
| `just test -p datax-cli` | `codex-rs` | `Deferred` | CLI crate tests pass after rename. |
| `just test -p datax-core` | `codex-rs` | `Deferred` | Core crate tests pass after rename. |
| `just test -p datax-protocol` | `codex-rs` | `Deferred` | Protocol crate tests pass after rename. |
| `just test -p datax-app-server-protocol` | `codex-rs` | `Deferred` | App-server protocol crate tests pass after rename; protocol concept rename remains Phase 1.4. |

## Validation and Acceptance

Validation execution is staged. The following commands are intentionally documented but deferred until the user runs the post-implementation migration validation pass, unless the user explicitly asks to run one during this milestone.

From the repository root, check whitespace and expect no output:

    git diff --check

From the repository root, check the forbidden mixed-case spelling and expect no output:

    rg -n 'Data[X]' .

From the repository root, check Rust manifests and expect no internal package/dependency names with the old prefix except documented external or provenance exceptions:

    rg -n 'name = "codex-|package = "codex-|^codex-[a-z0-9_-]+ = \{' codex-rs --glob 'Cargo.toml'

From the repository root, check Rust crate identifiers and Bazel crate names and expect no renamed crate identifiers with the old prefix except documented protected or infrastructure exceptions:

    rg -n 'crate_name = "codex_|use codex_|::codex_|codex_[A-Za-z0-9_]*::' codex-rs --glob '*.rs' --glob 'BUILD.bazel'

From `codex-rs`, run formatting and expect it to complete successfully:

    just fmt

From `codex-rs`, refresh the Cargo lockfile and expect `codex-rs/Cargo.lock` to contain renamed internal package names:

    cargo generate-lockfile

From the repository root, refresh and verify Bazel lock metadata if it changes:

    just bazel-lock-update
    just bazel-lock-check

From `codex-rs`, run targeted crate tests and expect them to pass:

    just test -p datax-cli
    just test -p datax-core
    just test -p datax-protocol
    just test -p datax-app-server-protocol

Acceptance for this milestone is met when Rust package names, crate identifiers, dependency keys, Bazel labels, and lockfiles consistently use Datax naming, the exception register explains any retained Codex/codex references, and the deferred validation command list is complete.

## Idempotence and Recovery

The rename operations are safe to repeat if they are implemented as deterministic mechanical replacements and followed by `rg` searches. If a mechanical rewrite touches protected sandbox identifiers or Phase 1.4 protocol concepts, recover by reverting only those hunks and recording the reason in this plan. If lockfile generation produces unrelated dependency churn, inspect `git diff codex-rs/Cargo.lock` and keep only changes caused by renamed internal package names.

Rollback for the milestone branch is a normal git branch rollback. No database migrations or destructive local-state changes are part of this phase.

## Artifacts and Notes

GitHub issue and pull request links will be added after creation.

Initial discovery transcript:

    rg -l 'codex[_-][A-Za-z0-9_-]+' codex-rs --glob '*.rs' --glob 'Cargo.toml' --glob 'Cargo.lock' --glob 'BUILD.bazel' --glob '*.bzl' | wc -l
    2008

    rg -l '^name = "codex-|^codex-[a-z0-9_-]+ = \{|package = "codex-' codex-rs/Cargo.toml codex-rs/*/Cargo.toml codex-rs/*/*/Cargo.toml codex-rs/*/*/*/Cargo.toml | wc -l
    132

## Interfaces and Dependencies

At the end of this milestone, internal Rust packages should use package names such as `datax-core`, `datax-protocol`, and `datax-cli`. Their Rust library crate identifiers should use names such as `datax_core`, `datax_protocol`, and `datax_tui`. Source imports should refer to those renamed crate identifiers.

The Bazel helper rule may still be named `codex_rust_crate` as a documented build-infrastructure exception, but crate targets and `crate_name` values should agree with the new Datax crate identifiers.

## Change Notes

2026-07-06: Created the Phase 1.3 ExecPlan before implementation so the milestone has a branch, inventory, dependency order, exception register, and explicit deferred validation commands.
