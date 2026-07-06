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
- [x] (2026-07-06T12:32:24Z) Created GitHub issue #5 and draft PR #6 for Phase 1.3.
- [x] (2026-07-06T12:40:22Z) Renamed Rust package names and workspace dependency keys from `codex-*` to `datax-*`.
- [x] (2026-07-06T12:40:22Z) Renamed Rust crate identifiers from `codex_*` to `datax_*` where they refer to renamed workspace crates.
- [x] (2026-07-06T12:40:22Z) Updated Bazel crate target names and `crate_name` attributes to match Datax naming while preserving `codex-rs` filesystem paths.
- [x] (2026-07-06T12:40:22Z) Refreshed `codex-rs/Cargo.lock` with `cargo generate-lockfile`; first sandboxed attempt failed on network DNS, escalated retry succeeded.
- [x] (2026-07-06T12:40:22Z) Ran `just fmt` from `codex-rs` successfully after fixing manifest path drift.
- [x] (2026-07-06T12:40:22Z) Ran lightweight static checks and Python syntax checks; build/test commands remain deferred.
- [x] (2026-07-06T12:40:22Z) Updated this ExecPlan with final inventory status, decisions, validation command status, and outcome notes.
- [x] (2026-07-06T13:20:00Z) Fixed `codex-rs/.config/nextest.toml` package filters after user validation showed `just test -p datax-cli` failed during nextest config parsing.
- [x] (2026-07-06T13:35:00Z) Corrected `codex-rs/Cargo.lock` Rama helper packages from `0.3.0-rc1` to `0.3.0-alpha.4` after user validation showed Rust 1.95 rejected the rc packages.
- [x] (2026-07-06T13:45:00Z) Fixed `datax-rmcp-client` initialization compile error by cloning the `InitializeResult` inside RMCP's `Arc`.
- [x] (2026-07-06T13:55:00Z) Fixed `datax-mcp` compile error caused by a mechanical rewrite from local module `codex_apps` to nonexistent crate `datax_apps`.

## Surprises & Discoveries

- Observation: The Rust crate rename cannot be limited to `Cargo.toml` files because source imports use generated crate identifiers such as `codex_core`, `codex_protocol`, and `codex_utils_*`.
  Evidence: `rg -l 'codex[_-][A-Za-z0-9_-]+' codex-rs --glob '*.rs' --glob 'Cargo.toml' --glob 'Cargo.lock' --glob 'BUILD.bazel' --glob '*.bzl' | wc -l` returned `2008`.

- Observation: The repository instructions still describe existing upstream crate names as `codex-*`, but the approved migration plan explicitly requires `datax-*` crate package names in Phase 1.3.
  Evidence: `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` says “Rename crate packages from `codex-*` to `datax-*`.”

- Observation: Mechanical package-name replacement can corrupt filesystem paths for crates whose directories still begin with `codex-`.
  Evidence: `just fmt` initially failed because Cargo looked for `codex-rs/datax-experimental-api-macros/Cargo.toml`; fixing the path fields restored manifest loading.

- Observation: Bazel was not installed in the environment, but a temporary Bazelisk install in `/tmp` was sufficient to run the required lock commands without modifying the repository.
  Evidence: The first `just bazel-lock-update` failed with `bazel: not found`; after downloading Bazelisk to `/tmp/datax-bazel-bin/bazel`, `PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-update` and `PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-check` both completed.

- Observation: `codex-rs/.config/nextest.toml` is part of the Rust package rename dependency surface because nextest validates package filters before running even a targeted crate test.
  Evidence: User-run `just test -p datax-cli` failed with `operator didn't match any packages` for filters referencing `codex-app-server-protocol`, `codex-app-server`, `codex-core`, and `codex-windows-sandbox`.

- Observation: Refreshing `codex-rs/Cargo.lock` selected `rama-error`, `rama-macros`, and `rama-utils` `0.3.0-rc1`, but the repository toolchain is Rust 1.95 and those rc packages require Rust 1.96.
  Evidence: User-run validation failed with `rustc 1.95.0 is not supported by the following packages`; the direct Rama dependencies in `codex-rs/network-proxy/Cargo.toml` are pinned to `=0.3.0-alpha.4`, so the helper packages were locked back to `0.3.0-alpha.4` with `cargo update -p ... --precise 0.3.0-alpha.4`.

- Observation: RMCP's `peer_info()` returns an `Arc<InitializeResult>` with the locked dependency graph, while `McpClient::initialize` returns an owned `InitializeResult`.
  Evidence: User-run validation failed compiling `datax-rmcp-client` with `expected InitializeResult, found Arc<InitializeResult>` at `rmcp-client/src/rmcp_client.rs:485`.

- Observation: The `codex_apps` module in `datax-mcp` is an internal module, not an external crate import, so it must remain referenced as `codex_apps`.
  Evidence: User-run validation failed compiling `datax-mcp` with unresolved import `datax_apps` in `codex-mcp/src/lib.rs`; the symbols are defined in `codex-rs/codex-mcp/src/codex_apps.rs`.

## Decision Log

- Decision: Treat Phase 1.3 as a mechanical Rust workspace rename, not a semantic cleanup.
  Rationale: The milestone is intended to remove rename drift after Phase 1.2 while preserving behavior. Semantic concepts such as Thread, Turn, and Item remain in later phases.
  Date/Author: 2026-07-06 / Codex

- Decision: Preserve the top-level `codex-rs` directory name in this milestone.
  Rationale: The current migration plan names Rust crate packages, imports, lockfiles, and Bazel metadata as Phase 1.3 scope, while filesystem/persistence paths are handled later. Renaming the root Rust directory would require broad script and documentation churn outside the compile stabilization surface.
  Date/Author: 2026-07-06 / Codex

- Decision: Keep crate directory names such as `codex-api`, `codex-client`, `codex-home`, and `codex-mcp` unchanged while renaming their Cargo package names.
  Rationale: Renaming the directories would create broad path and workspace churn beyond the package/crate identity stabilization goal. Cargo supports `datax-*` package names with existing path values.
  Date/Author: 2026-07-06 / Codex

- Decision: Rename Windows helper binaries to `datax-command-runner` and `datax-windows-sandbox-setup` in this milestone.
  Rationale: They are Rust binary outputs and release/package surfaces tied to the crate rename, so leaving them as `codex-*` would keep build-output identity drift.
  Date/Author: 2026-07-06 / Codex

- Decision: Do not rename or modify protected sandbox identifiers.
  Rationale: The repository instructions explicitly protect `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`; these are environment-contract names, not product identity.
  Date/Author: 2026-07-06 / Codex

## Outcomes & Retrospective

Implemented outcome: Rust workspace metadata, source-level external crate references, package scripts, and release/package binary names now use Datax naming consistently for Phase 1.3 scope. Build and test execution remains staged; lightweight checks were run locally. Cargo and Bazel lock metadata were refreshed.

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
| `docs/plans/datax_migration_phase1_3_rust_workspace/rust_workspace_stabilization_execplan.md` | `Completed` | Living ExecPlan updated with implementation decisions, validation status, and outcome notes. |
| `codex-rs/Cargo.toml` | `Completed` | Root workspace package/dependency keys renamed to `datax-*`; existing `codex-*` directory paths preserved where those directories still exist. |
| `codex-rs/Cargo.lock` | `Completed` | Refreshed with `cargo generate-lockfile`; internal package names now use `datax-*`; Rama helper packages are locked to Rust 1.95-compatible `0.3.0-alpha.4`. |
| `codex-rs/.config/nextest.toml` | `Completed` | Updated nextest package filters from old package names to `datax-*` so targeted tests can parse the config. |
| `codex-rs/**/Cargo.toml` | `Completed` | Rust crate package names, dependency keys, library names, and binary names renamed where they represent internal Datax crates and binaries. |
| `codex-rs/**/BUILD.bazel` | `Completed` | Bazel crate target names and `crate_name` values renamed; `codex_rust_crate` and `codex-rs` path references remain documented exceptions. |
| `codex-rs/**/*.rs` | `Completed` | External workspace crate paths changed from `codex_*::` to `datax_*::`; internal modules such as `crate::codex_thread` are not crate imports and are deferred exceptions. |
| `codex-rs/codex-mcp/src/lib.rs` | `Completed` | Restored public re-exports for `CodexAppsToolsCacheKey` and `codex_apps_tools_cache_key` to the local `codex_apps` module. |
| `codex-rs/rmcp-client/src/rmcp_client.rs` | `Completed` | Adjusted RMCP initialize result handling to return an owned `InitializeResult` after `peer_info()` returns an `Arc`. |
| `codex-rs/**/*.bzl` | `Not Required` | Inspected for Phase 1.3. Existing `codex-rs` path handling and `codex_rust_crate` helper infrastructure are retained exceptions. |
| `MODULE.bazel` | `Not Required` | Inspected; it references the `codex-rs` workspace path, not internal Rust package names requiring Phase 1.3 modification. |
| `MODULE.bazel.lock` | `Completed` | Refreshed with `PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-update` after installing temporary Bazelisk outside the repo. |
| `defs.bzl` | `Not Required` | Inspected; helper-rule names and `codex-rs` path handling are retained build-infrastructure exceptions. |
| `datax-cli/package.json` | `Not Required` | Inspected; no Rust package rename change required in this file. |
| `README.md` | `Not Required` | Inspected for Rust workspace command references; no Phase 1.3 edit required. |
| `docs/install.md` | `Completed` | Updated staged test command examples from old Rust package names to Datax package names. |
| `justfile` | `Completed` | Updated Cargo package/bin command references to Datax names. |
| `scripts/**` | `Completed` | Updated Rust package/binary references in packaging and helper scripts; preserved `codex-rs` workspace path references. |
| `.github/**` | `Completed` | Updated Rust release, DotSlash, code-signing, and Cargo workspace helper references tied to renamed Rust package/binary outputs. |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` references | `Not Required` | Protected sandbox identifiers are intentionally excluded from all Phase 1 rename operations. |
| `Thread`, `Turn`, and `Item` protocol types | `Not Required` | App-server model rename is Phase 1.4 and must not be mixed into this branch. |
| Persistence directories and fixture snapshots | `Not Required` | Persistence, fixtures, and snapshots are Phase 1.5 unless a crate rename directly forces a checked-in generated artifact change. |

## Rename Exception Register

The following Codex/codex references may remain after Phase 1.3:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`, because they are protected sandbox contracts.
- The top-level `codex-rs/` directory path, because this milestone stabilizes Rust workspace metadata and imports without moving the Rust workspace root.
- Crate directories still named `codex-api`, `codex-client`, `codex-home`, `codex-mcp`, `codex-backend-openapi-models`, and `codex-experimental-api-macros`; their package names are Datax, but paths stay unchanged in this milestone.
- The Bazel helper rule name `codex_rust_crate`, if changing it would be broad build-infrastructure churn not required for package or crate identity correctness.
- Internal Rust modules such as `crate::codex_thread`, `crate::codex_delegate`, and MCP tool modules; these are not external crate identifiers and will be handled with conceptual/persistence renames in later milestones if still product-owned.
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
| `git diff --check` | repository root | `Completed` | No whitespace errors. |
| `rg -n 'Data[X]' .` | repository root | `Completed` | No forbidden mixed-case spelling. |
| `rg -n 'name = "codex-|package = "codex-|^codex-[a-z0-9_-]+ = \{' codex-rs --glob 'Cargo.toml'` | repository root | `Completed` | No internal Rust package/dependency names remain except documented external/provenance exceptions. |
| `rg --pcre2 -n '(?<!crate::)(?<!super::)\bcodex_[A-Za-z0-9_]+::' codex-rs --glob '*.rs'` | repository root | `Completed` | No old external crate-path references remain. |
| `rg -n 'crate_name = "codex_' codex-rs --glob 'BUILD.bazel'` | repository root | `Completed` | No old Bazel `crate_name` values remain. |
| `rg -n 'package\(codex-' codex-rs/.config/nextest.toml` | repository root | `Completed` | No old nextest package filters remain. |
| `python3 -m py_compile scripts/datax_package/*.py datax-cli/scripts/build_npm_package.py scripts/stage_npm_packages.py` | repository root | `Completed` | Touched Python package scripts compile. |
| `just fmt` | `codex-rs` | `Completed` | Rust formatting completed successfully. |
| `cargo generate-lockfile` | `codex-rs` | `Completed` | `codex-rs/Cargo.lock` refreshed with renamed internal package names after escalated network retry. |
| `cargo update -p rama-error --precise 0.3.0-alpha.4` | `codex-rs` | `Completed` | `rama-error` lock entry is compatible with Rust 1.95. |
| `cargo update -p rama-utils --precise 0.3.0-alpha.4` | `codex-rs` | `Completed` | `rama-utils` lock entry is compatible with Rust 1.95. |
| `cargo update -p rama-macros --precise 0.3.0-alpha.4` | `codex-rs` | `Completed` | `rama-macros` lock entry is compatible with Rust 1.95. |
| `rg -n '0\.3\.0-rc1' codex-rs/Cargo.lock` | repository root | `Completed` | No Rama rc package versions remain in the lockfile. |
| `PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-update` | repository root | `Completed` | Bazel lock metadata refreshed using temporary Bazelisk. |
| `PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-check` | repository root | `Completed` | Bazel lock metadata has no drift. |
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

    rg --pcre2 -n '(?<!crate::)(?<!super::)\bcodex_[A-Za-z0-9_]+::' codex-rs --glob '*.rs'
    rg -n 'crate_name = "codex_' codex-rs --glob 'BUILD.bazel'

From `codex-rs`, run formatting and expect it to complete successfully:

    just fmt

From `codex-rs`, refresh the Cargo lockfile and expect `codex-rs/Cargo.lock` to contain renamed internal package names:

    cargo generate-lockfile

From the repository root, refresh and verify Bazel lock metadata if it changes:

    PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-update
    PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-check

The commands above were run with a temporary Bazelisk binary installed at `/tmp/datax-bazel-bin/bazel` because `bazel` was not installed globally.

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

GitHub issue: https://github.com/mbellary/datax/issues/5

Draft pull request: https://github.com/mbellary/datax/pull/6

Initial discovery transcript:

    rg -l 'codex[_-][A-Za-z0-9_-]+' codex-rs --glob '*.rs' --glob 'Cargo.toml' --glob 'Cargo.lock' --glob 'BUILD.bazel' --glob '*.bzl' | wc -l
    2008

    rg -l '^name = "codex-|^codex-[a-z0-9_-]+ = \{|package = "codex-' codex-rs/Cargo.toml codex-rs/*/Cargo.toml codex-rs/*/*/Cargo.toml codex-rs/*/*/*/Cargo.toml | wc -l
    132

Formatter and generation transcript:

    just fmt
    # First run failed because manifest path values were rewritten from codex-* directories to datax-* directories.
    # After restoring path values, just fmt completed successfully.

    cargo generate-lockfile
    # First sandboxed run failed resolving index.crates.io.
    # Escalated retry completed and refreshed codex-rs/Cargo.lock.

    PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-update
    # Completed after temporary Bazelisk installation.

    PATH=/tmp/datax-bazel-bin:$PATH just bazel-lock-check
    # Completed with no lock drift.

## Interfaces and Dependencies

At the end of this milestone, internal Rust packages should use package names such as `datax-core`, `datax-protocol`, and `datax-cli`. Their Rust library crate identifiers should use names such as `datax_core`, `datax_protocol`, and `datax_tui`. Source imports should refer to those renamed crate identifiers.

The Bazel helper rule may still be named `codex_rust_crate` as a documented build-infrastructure exception, but crate targets and `crate_name` values should agree with the new Datax crate identifiers.

## Change Notes

2026-07-06: Created the Phase 1.3 ExecPlan before implementation so the milestone has a branch, inventory, dependency order, exception register, and explicit deferred validation commands.

2026-07-06: Added GitHub issue and draft pull request links after creating the milestone tracking artifacts.

2026-07-06: Implemented the Rust workspace package/crate rename, updated package and release helpers, refreshed the Cargo lockfile, and recorded the Bazel lock update blocker.
