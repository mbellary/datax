# Phase 1.2 Product Identity Rename

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` from the repository root. It builds on `docs/plans/Recommended-Datax-Migration-Execution-Model.md` and the completed Phase 1.1 repository preparation plan at `docs/plans/datax_migration_phase1_1_repo_prep/repository_fork_preparation_execplan.md`.

## Purpose / Big Picture

Phase 1.2 changes the product identity surfaces that make the fork install, package, and launch as Datax instead of the upstream product name. After this milestone, the npm CLI shim, package builder, release package metadata, installer scripts, DotSlash manifest, root package metadata, and focused installation guidance should name Datax. A reviewer should be able to stage the npm package or run the local launcher and see `datax`-named package metadata and executable paths without any product feature behavior changes.

This milestone is not the full Rust crate graph migration. The `codex-rs` workspace directory, Rust crate package names, crate imports, app-server protocol concepts, generated protocol artifacts, Python SDK public module names, and persisted state paths remain for later Phase 1 milestones unless they directly block the package identity rename here.

## Progress

- [x] (2026-07-06T05:48:19Z) Confirmed Phase 1.1 PR #2 is merged and local `main` contains merge commit `0b8fbfedc9e342895c5f6c074cc667ac8cd910cb`.
- [x] (2026-07-06T05:48:19Z) Created branch `datax/migration-phase1-2-product-rename`.
- [x] (2026-07-06T05:48:19Z) Read `PLANS.md`, the recommended execution model, and the provisional Phase 1 migration plan.
- [x] (2026-07-06T05:48:19Z) Ran targeted discovery over package metadata, release scripts, installer scripts, CLI shim files, and build labels before implementation edits.
- [x] (2026-07-06T05:48:19Z) Established the Phase 1.2 boundary and dependency order before touching implementation files.
- [x] (2026-07-06T05:48:19Z) Created GitHub issue #3 for Phase 1.2 scope and acceptance.
- [ ] Create draft PR for this branch after the initial ExecPlan commit is pushed.
- [ ] Rename top-level npm CLI package directory and launcher surface.
- [ ] Rename canonical release package builder files, constants, and tests.
- [ ] Rename installer scripts and focused install documentation.
- [ ] Update root package metadata, selected release metadata, and lockfiles affected by path/package changes.
- [ ] Run incremental formatting and targeted validation commands.
- [ ] Update this ExecPlan with final inventory statuses, validation evidence, issue, PR, and outcome notes.

## Surprises & Discoveries

- Observation: Phase 1.2 and Phase 1.3 overlap in the provisional plan around Rust crate names.
  Evidence: Phase 1.2 says to rename Rust crate names, while Phase 1.3 is dedicated to Rust workspace stabilization. This plan resolves the overlap by limiting Phase 1.2 Rust edits to the CLI binary/package identity needed by the package launcher and release packaging, while deferring the full crate graph to Phase 1.3.

- Observation: SDK package metadata is product identity, but Python and TypeScript SDK source trees also encode protocol concepts such as thread and turn.
  Evidence: `sdk/python/pyproject.toml`, `sdk/typescript/package.json`, and SDK examples reference product names, while many SDK source files reference app-server thread/turn concepts planned for Phase 1.4. This milestone only changes SDK package metadata if it can be done without half-renaming public API concepts.

## Decision Log

- Decision: Keep the `codex-rs` directory name and most Rust crate package names unchanged in Phase 1.2.
  Rationale: Renaming the workspace directory and all `codex-*` crates requires coordinated Cargo, Bazel, imports, generated files, and lockfile updates. That is the explicit Phase 1.3 scope and would make Phase 1.2 too broad to validate incrementally.
  Date/Author: 2026-07-06 / Codex

- Decision: Rename package and release-builder path identities in Phase 1.2, including `codex-cli` and `scripts/codex_package`, because those are direct product packaging surfaces.
  Rationale: The npm package and standalone package builder are how the product is installed and launched. Leaving these names unchanged would fail the milestone purpose even if Rust internals still carry deferred names.
  Date/Author: 2026-07-06 / Codex

- Decision: Do not rename protected sandbox identifiers in any phase.
  Rationale: Repository instructions explicitly protect `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR` because they are tied to sandbox behavior.
  Date/Author: 2026-07-06 / Codex

## Outcomes & Retrospective

Not yet complete. This section will summarize the implemented product identity rename, validation results, remaining exceptions, and follow-up work before the milestone closes.

## Context and Orientation

This repository is a fork that still contains many upstream product identifiers. Phase 1.2 focuses on product identity surfaces: files that declare package names, executable names, release artifact names, install commands, or user-facing installer messages. A package builder is a script that assembles files into a release directory or npm tarball. A launcher is a small script that locates the native binary for the current platform and runs it.

The npm CLI package currently lives under `codex-cli/`. Its `package.json` exposes a `codex` command backed by `bin/codex.js`. That JavaScript launcher locates optional platform npm packages such as `@openai/codex-linux-x64` and then runs a native binary named `codex`.

The standalone release package builder currently lives under `scripts/codex_package/` and is launched through `scripts/build_codex_package.py`. It creates package layouts with files such as `codex-package.json`, `codex-resources`, `codex-path`, and `bin/codex`.

The root `justfile` wraps common developer commands and currently has helper recipes such as `codex` and `bazel-codex`. The root `package.json` and `pnpm-lock.yaml` name workspace paths and package metadata. GitHub release workflow and DotSlash config files encode release artifact names and binary paths.

## File Inventory

`Modified` values are `Pending`, `In-Progress`, `Completed`, `Failed`, or `Not Required`. `Not Required` means the file belongs to the discovery set or dependency chain but is intentionally deferred or only inspected for this milestone.

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/datax_migration_phase1_2_product_rename/product_identity_rename_execplan.md` | `In-Progress` | Living plan for Phase 1.2; update throughout the milestone. |
| `codex-cli/package.json` | `Pending` | Rename npm package metadata, command bin name, files entry, and repository URL/directory after directory rename. |
| `codex-cli/bin/codex.js` | `Pending` | Rename launcher file and product-specific package names, binary name, reinstall text, and managed environment variables where not protected. |
| `codex-cli/scripts/build_npm_package.py` | `Pending` | Rename npm staging constants, package choices, platform package names, staged bin path, temp prefixes, and SDK dependency wiring where in scope. |
| `codex-cli/scripts/README.md` | `Pending` | Update package staging instructions from old package names to Datax package names. |
| `codex-cli/scripts/init_firewall.sh` | `Pending` | Rename `/etc/codex` product path to `/etc/datax` if no dependency blocks it. |
| `codex-cli/scripts/run_in_container.sh` | `Pending` | Inspect and update product path/package references used by npm package staging. |
| `scripts/build_codex_package.py` | `Pending` | Rename launcher script to Datax equivalent and update import path after package-builder directory rename. |
| `scripts/codex_package/__init__.py` | `Pending` | Rename module docstring after package-builder directory rename. |
| `scripts/codex_package/archive.py` | `Pending` | Rename package archive temp prefixes and docstrings. |
| `scripts/codex_package/cargo.py` | `Pending` | Rename package builder constants and built binary names that directly define release packaging outputs; defer full Rust crate rename. |
| `scripts/codex_package/cli.py` | `Pending` | Rename CLI flags, defaults, temp prefixes, and user-visible package builder messages. |
| `scripts/codex_package/dotslash.py` | `Pending` | Rename package cache directory and package path metadata. |
| `scripts/codex_package/layout.py` | `Pending` | Rename canonical package layout metadata and executable/resource path names. |
| `scripts/codex_package/ripgrep.py` | `Pending` | Update manifest path after package-builder directory rename. |
| `scripts/codex_package/targets.py` | `Pending` | Rename package variant names, cargo binary mapping where in scope, and executable stems. |
| `scripts/codex_package/version.py` | `Pending` | Inspect for package builder path dependency; likely update docstring only. |
| `scripts/codex_package/zsh.py` | `Pending` | Rename DotSlash zsh manifest path and artifact label if release artifact is product-owned. |
| `scripts/codex_package/test_archive.py` | `Pending` | Update import path after package-builder directory rename if tests cover package module name. |
| `scripts/codex_package/test_cargo.py` | `Pending` | Update package variant names, expected binaries, and builder argument names that change in this phase. |
| `scripts/codex_package/README.md` | `Pending` | Update package builder documentation because it is build-critical documentation for release packaging. |
| `scripts/codex_package/codex-zsh` | `Pending` | Rename manifest output name/path only if release zsh artifact is product-owned; upstream release URLs may remain exception until artifact source exists. |
| `scripts/stage_npm_packages.py` | `Pending` | Update script import path, package artifact names, GitHub repo, native component names, and package expansion logic. |
| `scripts/install/install.sh` | `Pending` | Rename Unix installer environment variables, install paths, release URLs, package asset names, executable name, and user-facing messages. |
| `scripts/install/install.ps1` | `Pending` | Rename Windows installer environment variables, install paths, release URLs, package asset names, executable name, namespace, and user-facing messages. |
| `.github/dotslash-config.json` | `Pending` | Rename DotSlash output keys, artifact regexes, and binary paths for primary CLI/app-server release packages where in scope. |
| `.github/scripts/build-codex-package-archive.sh` | `Pending` | Rename script path/name and package archive references after package-builder rename. |
| `.github/workflows/rust-release.yml` | `Pending` | Rename release package build steps and artifact names that directly produce product-owned CLI package outputs; defer broad workflow cleanup unrelated to Phase 1.2. |
| `package.json` | `Pending` | Rename root monorepo package name and script paths affected by top-level directory rename. |
| `pnpm-lock.yaml` | `Pending` | Update workspace path entries after npm package directory/package metadata changes. |
| `README.md` | `Pending` | Update focused first-screen install and package identity text only; avoid broad product documentation expansion. |
| `docs/install.md` | `Pending` | Update build-critical install instructions and local CLI command examples. |
| `justfile` | `Pending` | Add/rename local helper recipes for `datax`; keep Rust crate package references deferred unless command behavior requires aliases. |
| `MODULE.bazel` | `Pending` | Rename Bazel module name and direct repository identity comments; defer `//codex-rs` labels to Phase 1.3. |
| `codex-rs/cli/Cargo.toml` | `Pending` | Rename CLI binary from `codex` to `datax` if package launcher and release packaging require it; defer dependency crate names. |
| `codex-rs/cli/BUILD.bazel` | `Pending` | Rename Bazel release binary target from product-owned CLI output to Datax if tied to package artifacts; defer crate macro names. |
| `codex-rs/Cargo.toml` | `Not Required` | Full workspace crate package rename belongs to Phase 1.3; inspect only for version and package builder dependency. |
| `codex-rs/Cargo.lock` | `Not Required` | Full crate lockfile rename belongs to Phase 1.3; do not churn before workspace rename. |
| `codex-rs/responses-api-proxy/npm/package.json` | `Not Required` | Separate npm package remains tied to response proxy packaging; inspect as dependency of staging script but do not rename in this milestone unless packaging tests require it. |
| `sdk/typescript/package.json` | `Not Required` | SDK product package metadata belongs to package rename discovery, but public TypeScript SDK API is tied to Phase 1.4 protocol rename; defer to avoid half-renaming SDK surface. |
| `sdk/python/pyproject.toml` | `Not Required` | Python package metadata is tied to module name, generated types, and runtime dependency package; defer to SDK/protocol milestone instead of breaking imports. |
| `sdk/python/uv.lock` | `Not Required` | Generated Python lockfile remains unchanged until Python SDK package/runtime dependency changes are performed together. |
| `sdk/python/src/openai_codex/**` | `Not Required` | Public module/package name and protocol methods are deferred to SDK/protocol milestones. |
| `sdk/typescript/src/**` | `Not Required` | TypeScript public API has thread/turn concepts and should change with Phase 1.4 protocol rename, not this package identity band. |
| `.github/CODEOWNERS` | `Not Required` | Existing owners and upstream team references are provenance/review metadata; no product package behavior change. |
| `.github/codex/**` | `Not Required` | Codex app automation metadata is not part of package identity and can remain as tooling provenance unless a later workflow milestone changes it. |

## Rename Exception Register

The following names may remain after Phase 1.2 and must be revisited later:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`: protected by repository instructions and never renamed.
- `codex-rs/` directory and `codex-*` Rust crate packages: deferred to Phase 1.3 Rust workspace stabilization.
- App-server thread/turn/item public protocol names: deferred to Phase 1.4.
- Python package/module names such as `openai_codex`: deferred until SDK and generated protocol changes can be done together.
- Historical upstream release URLs for already-published artifacts: retain only where the artifact cannot exist under Datax yet, and document each retained URL after implementation.
- `.github/codex/**` automation metadata: repository tooling provenance, not user-facing product package identity in this milestone.

## Public Surface Checklist

This milestone touches CLI executable naming, npm package names, install scripts, release artifact names, DotSlash manifests, package-builder output layout, selected docs that explain installation/build commands, and Bazel release output labels. It does not intentionally touch configuration file formats, app-server protocol methods, persisted session formats, generated app-server schemas, TypeScript bindings, Python SDK generated bindings, snapshots, or data engineering product behavior.

If `codex-rs/cli/Cargo.toml` changes the binary name from `codex` to `datax`, targeted CLI validation must prove `cargo run --bin datax -- --version` and the npm launcher path both work. If that binary rename reveals broad import or test coupling, record the dependency and either complete only the required direct references or defer with an explicit exception.

## Dependency Order

First, create and commit this ExecPlan so all implementation edits have a reviewed boundary. Second, create the GitHub issue and draft PR after the plan exists. Third, rename the npm CLI directory from `codex-cli` to `datax-cli`, rename the launcher from `bin/codex.js` to `bin/datax.js`, and update `package.json` plus package staging script references. Fourth, rename the standalone package builder directory and launcher from `scripts/codex_package` and `scripts/build_codex_package.py` to Datax equivalents, then update imports and package-builder tests. Fifth, rename installer scripts and release metadata that depend on the new package layout. Sixth, update root metadata, `pnpm-lock.yaml`, `README.md`, `docs/install.md`, `justfile`, `MODULE.bazel`, and focused Bazel release labels. Seventh, run formatters and targeted tests, then update the inventory statuses and exception register.

Source scripts should be changed before generated lockfiles. Directory renames must be done before import-path rewrites. Release package layout changes must be validated before installer scripts are considered complete. No app-server protocol or SDK public API files should be edited until their dependency milestone.

## Plan of Work

Rename package identity in the npm CLI package. Change the package name from the upstream package to a Datax package, expose a `datax` bin entry, rename the launcher file, update optional platform package names, update the native binary lookup to expect `datax` or `datax.exe`, and update managed package environment variables to `DATAX_*` unless they are protected sandbox variables.

Rename the standalone package builder. Move the helper package directory and script names to Datax equivalents. Update constants, docstrings, package variants, canonical layout names, artifact names, and tests so the builder creates a `datax` executable layout with Datax package metadata.

Rename installer and release metadata. Update Unix and Windows install scripts so they use `DATAX_*` environment variables, `.datax` home paths, `datax` executable names, Datax package assets, and the `mbellary/datax` repository for release lookup. Update the release workflow and DotSlash config only where they directly name product-owned package artifacts.

Update local developer metadata. Adjust `package.json`, `pnpm-lock.yaml`, `README.md`, `docs/install.md`, `justfile`, `MODULE.bazel`, and focused CLI Bazel/Cargo output names so local build and launch instructions align with Datax naming. Keep deferred Rust crate names documented rather than partially renamed.

## Concrete Steps

From the repository root, run:

    git status --short --branch
    rg -l "codex|Codex|CODEX" codex-cli scripts/codex_package scripts/stage_npm_packages.py scripts/build_codex_package.py scripts/install .github/dotslash-config.json .github/workflows/rust-release.yml .github/scripts/build-codex-package-archive.sh package.json README.md docs/install.md justfile MODULE.bazel codex-rs/cli/Cargo.toml codex-rs/cli/BUILD.bazel | sort

Create the GitHub issue:

    gh issue create --repo mbellary/datax --title "Phase 1.2: product identity rename" --body-file <temp-body>

After the first commit is pushed, create a draft PR:

    gh pr create --repo mbellary/datax --draft --base main --head datax/migration-phase1-2-product-rename --title "Phase 1.2: product identity rename" --body-file <temp-body>

After implementation, run the validation commands listed below and paste concise evidence into this ExecPlan.

## Validation and Acceptance

Validation will be incremental because builds are slow:

- Run `git diff --check` from the repository root and expect no whitespace errors.
- Run `just fmt` from `codex-rs` after code changes and expect formatting to complete successfully.
- Run `just fmt-check` from the repository root and expect no formatting diffs.
- Run package-builder unit tests for the renamed script package with Python unittest and expect all tests in that helper package to pass.
- If the Rust CLI binary name changes, run `just test -p codex-cli` from `codex-rs` and expect the CLI crate tests to pass.
- Run a package staging smoke test with a temporary staging directory for the Datax npm meta package and expect staged `package.json` to expose a `datax` bin and Datax optional dependency names.
- Run static searches for this milestone’s package surfaces and expect no old product identity references except documented exceptions.
- Do not run the complete `just test` suite without user approval.

Acceptance for this milestone is that a reviewer can inspect staged package metadata and installer/release scripts and see Datax package names, `datax` executable paths, and Datax repository identity. Any remaining upstream product references must either be protected sandbox identifiers, upstream provenance, or explicitly deferred in this plan.

## Validation Matrix

| Command | Working Directory | Required | Status | Remarks Notes |
| --- | --- | --- | --- | --- |
| `git diff --check` | repository root | Yes | Pending | Whitespace validation after edits. |
| `just fmt` | `codex-rs` | Yes, after code changes | Pending | Repository instruction requires this after code changes. |
| `just fmt-check` | repository root | Yes | Pending | Formatting check; may need unsandboxed run if tool cache writes outside workspace. |
| `python -m unittest discover -s scripts/datax_package -p 'test_*.py'` | repository root | Yes if package builder is renamed | Pending | Validates package-builder helper tests. |
| `just test -p codex-cli` | `codex-rs` | Yes if CLI binary/package changes | Pending | Targeted Rust CLI tests. |
| `node datax-cli/bin/datax.js --version` | repository root | Best effort | Pending | May fail before native binary staging; record result. |
| `python datax-cli/scripts/build_npm_package.py --version 0.0.0-dev --staging-dir /tmp/datax-npm-stage` | repository root | Yes if npm builder remains executable without native binaries | Pending | Stages npm meta package for metadata inspection. |
| Search for the forbidden mixed-case spelling in `docs/plans` | repository root | Yes | Pending | Must return no matches without writing the forbidden token into this plan. |

## Idempotence and Recovery

Directory renames should be performed with `git mv` so Git preserves history. If a validation command fails, keep the changed files in place, update the `Surprises & Discoveries` section with the failure, and fix the dependency that caused it before moving to the next rename band. If a generated lockfile changes unexpectedly, inspect the diff before accepting it; do not keep unrelated dependency updates. Rollback for this milestone is the branch itself: abandon the branch or revert the Phase 1.2 commits, with no database or external state migration required.

## Artifacts and Notes

GitHub issue:

    https://github.com/mbellary/datax/issues/3

GitHub draft PR is not created yet. Add it here immediately after creation.

Baseline:

    branch: datax/migration-phase1-2-product-rename
    base commit: 0b8fbfedc9e342895c5f6c074cc667ac8cd910cb
    Phase 1.1 PR: https://github.com/mbellary/datax/pull/2

## Interfaces and Dependencies

The Datax npm CLI package should expose a `datax` executable through `datax-cli/package.json` and `datax-cli/bin/datax.js`. The launcher should locate platform packages with Datax package names and execute `datax` or `datax.exe` from the native package vendor directory.

The standalone package builder should be importable as `scripts/datax_package` and launched through `scripts/build_datax_package.py`. Its canonical package layout should contain Datax-named metadata, resources, path directory, and executable entries. Tests should import the renamed helper package and assert Datax package variants.

The installer scripts should use `DATAX_RELEASE`, `DATAX_NON_INTERACTIVE`, `DATAX_INSTALL_DIR`, and `DATAX_HOME`, with `.datax` as the default product home. They should install or launch `datax`, not the upstream executable name. Protected sandbox identifiers are not part of these installer variables and must not be changed if encountered elsewhere.

Revision note, 2026-07-06: Initial Phase 1.2 ExecPlan added after branch creation and before implementation edits. This records the milestone boundary, inventory, dependency order, validation matrix, and deferrals so the product identity rename can proceed incrementally.
