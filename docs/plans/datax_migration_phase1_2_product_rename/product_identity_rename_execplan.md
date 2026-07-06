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
- [x] (2026-07-06T05:48:19Z) Pushed branch and created draft PR #4 for Phase 1.2.
- [x] (2026-07-06T05:48:19Z) Renamed top-level npm CLI package directory and launcher surface.
- [x] (2026-07-06T05:48:19Z) Renamed canonical release package builder files, constants, and tests.
- [x] (2026-07-06T05:48:19Z) Renamed installer scripts and focused install documentation.
- [x] (2026-07-06T05:48:19Z) Updated root package metadata, selected release metadata, and lockfiles affected by path/package changes.
- [x] (2026-07-06T05:48:19Z) Ran lightweight validation that does not require long Rust builds: Python syntax compile, package-builder unit tests, npm staging smoke, and `just fmt`.
- [x] (2026-07-06T05:48:19Z) Deferred long-running targeted Rust tests at user request; exact commands are retained in the Validation Matrix for the post-Phase-1 test pass.
- [x] (2026-07-06T05:48:19Z) Updated this ExecPlan with final inventory statuses, validation evidence, issue, PR, and outcome notes.

## Surprises & Discoveries

- Observation: Phase 1.2 and Phase 1.3 overlap in the provisional plan around Rust crate names.
  Evidence: Phase 1.2 says to rename Rust crate names, while Phase 1.3 is dedicated to Rust workspace stabilization. This plan resolves the overlap by limiting Phase 1.2 Rust edits to the CLI binary/package identity needed by the package launcher and release packaging, while deferring the full crate graph to Phase 1.3.

- Observation: SDK package metadata is product identity, but Python and TypeScript SDK source trees also encode protocol concepts such as thread and turn.
  Evidence: `sdk/python/pyproject.toml`, `sdk/typescript/package.json`, and SDK examples reference product names, while many SDK source files reference app-server thread/turn concepts planned for Phase 1.4. This milestone only changes SDK package metadata if it can be done without half-renaming public API concepts.

- Observation: Long Rust tests are too expensive to run after every phase during the migration.
  Evidence: `just test -p codex-cli` began compiling dependent crates and was interrupted at the user's request before completion. The migration process now records exact test commands in each phase ExecPlan and defers expensive execution until the user runs the per-phase test list after all phases are implemented.

- Observation: Running `node datax-cli/bin/datax.js --version` directly from the source checkout is not a valid launcher smoke test unless a native optional dependency or vendor directory has first been staged.
  Evidence: The source-tree launcher reported a missing `datax-linux-x64` optional dependency. The validation command was corrected to stage the meta package and a temporary native vendor executable before invoking `node`.

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

- Decision: Defer long-running test execution during each phase and document the exact commands instead.
  Rationale: The user will run the accumulated per-phase test commands after all phases are implemented and provide results. During implementation, lightweight checks may still run when they provide quick feedback, but expensive Rust/build validation should be recorded as deferred unless the user explicitly asks to run it.
  Date/Author: 2026-07-06 / Codex

## Outcomes & Retrospective

Phase 1.2 completed the package and executable identity rename needed for Datax install, packaging, and local launch surfaces. The npm CLI package now lives under `datax-cli`, exposes a `datax` bin, stages Datax platform optional dependencies, and points package metadata at `mbellary/datax`. The standalone release package builder now lives under `scripts/datax_package`, produces a Datax primary package layout, and is launched by `scripts/build_datax_package.py`.

The Unix and Windows installers now use `DATAX_*` variables, `.datax` defaults, Datax package assets, and `datax` executable paths. Focused release metadata, DotSlash config, root package metadata, `pnpm` workspace/lockfile metadata, selected Bazel binary labels, build-critical documentation, and update command text were updated to match the renamed executable and package surfaces.

Long-running Rust/build validation is intentionally deferred for the user's post-implementation migration test pass. Lightweight checks already completed are recorded in the Validation Matrix. Remaining upstream-name references are limited to protected sandbox identifiers, Rust workspace/crate names, app-server/protocol surfaces, SDK surfaces, helper binaries, historical/provenance paths, and other entries documented in the Rename Exception Register for later milestones.

## Context and Orientation

This repository is a fork that still contains many upstream product identifiers. Phase 1.2 focuses on product identity surfaces: files that declare package names, executable names, release artifact names, install commands, or user-facing installer messages. A package builder is a script that assembles files into a release directory or npm tarball. A launcher is a small script that locates the native binary for the current platform and runs it.

The npm CLI package currently lives under `codex-cli/`. Its `package.json` exposes a `codex` command backed by `bin/codex.js`. That JavaScript launcher locates optional platform npm packages such as `@openai/codex-linux-x64` and then runs a native binary named `codex`.

The standalone release package builder currently lives under `scripts/codex_package/` and is launched through `scripts/build_codex_package.py`. It creates package layouts with files such as `codex-package.json`, `codex-resources`, `codex-path`, and `bin/codex`.

The root `justfile` wraps common developer commands and currently has helper recipes such as `codex` and `bazel-codex`. The root `package.json` and `pnpm-lock.yaml` name workspace paths and package metadata. GitHub release workflow and DotSlash config files encode release artifact names and binary paths.

## File Inventory

`Modified` values are `Pending`, `In-Progress`, `Completed`, `Failed`, or `Not Required`. `Not Required` means the file belongs to the discovery set or dependency chain but is intentionally deferred or only inspected for this milestone.

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/datax_migration_phase1_2_product_rename/product_identity_rename_execplan.md` | `Completed` | Living plan for Phase 1.2; updated with inventory, validation staging, issue, PR, and outcome notes. |
| `docs/plans/Recommended-Datax-Migration-Execution-Model.md` | `Completed` | Updated migration-wide policy so each phase documents exact test/build commands and defers execution until the post-implementation test pass unless explicitly requested. |
| `datax-cli/package.json` | `Completed` | Renamed npm package metadata, `datax` bin entry, files entry, repository URL, and directory metadata. |
| `datax-cli/bin/datax.js` | `Completed` | Renamed launcher file, platform package names, native executable lookup, reinstall text, and managed package environment variables. |
| `datax-cli/scripts/build_npm_package.py` | `Completed` | Renamed CLI npm staging constants, package choices, platform package names, staged bin path, and temp prefixes; response proxy and SDK package choices remain deferred exceptions. |
| `datax-cli/scripts/README.md` | `Completed` | Updated package staging instructions for the Datax CLI package only. |
| `datax-cli/scripts/init_firewall.sh` | `Completed` | Renamed product-owned container firewall config path to `/etc/datax/allowed_domains.txt`. |
| `datax-cli/scripts/run_in_container.sh` | `Completed` | Renamed container image, container prefix, product config path, and CLI invocation to `datax`. |
| `scripts/build_datax_package.py` | `Completed` | Renamed package-builder launcher and import path. |
| `scripts/datax_package/__init__.py` | `Completed` | Renamed package-builder module docstring. |
| `scripts/datax_package/archive.py` | `Completed` | Renamed package archive docstring and temp prefixes. |
| `scripts/datax_package/cargo.py` | `Completed` | Renamed package-builder entrypoint behavior while keeping the current `codex-rs` workspace path and Windows helper binary names as documented deferrals. |
| `scripts/datax_package/cli.py` | `Completed` | Renamed CLI defaults, temp prefixes, and user-visible package builder messages; Windows helper flags remain tied to existing helper binaries. |
| `scripts/datax_package/dotslash.py` | `Completed` | Renamed package cache directory and package path metadata. |
| `scripts/datax_package/layout.py` | `Completed` | Renamed canonical package metadata, resources directory, path directory, and primary executable paths; Windows helper files remain deferred. |
| `scripts/datax_package/ripgrep.py` | `Completed` | Updated manifest path after package-builder directory rename. |
| `scripts/datax_package/targets.py` | `Completed` | Renamed primary package variant to `datax`; app-server variant remains deferred because its binary still exists under the current Rust package graph. |
| `scripts/datax_package/version.py` | `Completed` | Updated docstring/path context while keeping `codex-rs/Cargo.toml` as the current version source. |
| `scripts/datax_package/zsh.py` | `Completed` | Renamed zsh DotSlash manifest path and artifact label. |
| `scripts/datax_package/test_archive.py` | `Completed` | Updated imports after package-builder directory rename. |
| `scripts/datax_package/test_cargo.py` | `Completed` | Updated package variant expectations and primary binary expectations; Windows helper expectations remain deferred. |
| `scripts/datax_package/README.md` | `Completed` | Updated build-critical package builder documentation. |
| `scripts/datax_package/datax-zsh` | `Completed` | Renamed manifest output name/path and Datax release URLs for future artifacts. |
| `scripts/stage_npm_packages.py` | `Completed` | Updated script import path, primary package artifact names, GitHub repo, native component names, and package expansion logic. |
| `scripts/install/install.sh` | `Completed` | Renamed Unix installer environment variables, install paths, release URLs, package asset names, executable name, and user-facing messages. |
| `scripts/install/install.ps1` | `Completed` | Renamed Windows installer environment variables, install paths, release URLs, package asset names, executable name, namespace, and user-facing messages. |
| `.github/dotslash-config.json` | `Completed` | Renamed primary CLI DotSlash output key, artifact regexes, and binary paths; app-server/response-proxy/helper outputs remain deferred. |
| `.github/scripts/build-datax-package-archive.sh` | `Completed` | Renamed helper script path/name and primary package archive references. |
| `.github/workflows/rust-release.yml` | `Completed` | Renamed primary CLI release package build steps, artifacts, npm staging, package checksum manifest, and Datax binary references; unrelated release surfaces remain deferred. |
| `.github/workflows/sdk.yml` | `Completed` | Updated Bazel-built CLI target and staged binary path from the removed CLI binary target to `datax`. |
| `.github/scripts/test_run_bazel_with_buildbuddy.py` | `Completed` | Updated test fixture Bazel labels for the renamed CLI binary target. |
| `.github/ISSUE_TEMPLATE/3-cli.yml` | `Completed` | Updated CLI issue template to Datax user-facing package and command names. |
| `.github/blob-size-allowlist.txt` | `Completed` | Updated allowlist after splash image rename. |
| `.github/datax-cli-splash.png` | `Completed` | Renamed splash asset path used by `README.md`. |
| `package.json` | `Completed` | Renamed root monorepo package name. |
| `pnpm-lock.yaml` | `Completed` | Updated workspace importer path after npm package directory rename. |
| `pnpm-workspace.yaml` | `Completed` | Updated workspace package path to `datax-cli`. |
| `README.md` | `Completed` | Updated focused first-screen install and package identity text only; avoided broad product documentation expansion. |
| `docs/install.md` | `Completed` | Updated build-critical install instructions and local CLI command examples. |
| `justfile` | `Completed` | Added/renamed local helper recipes for `datax` and updated direct binary paths while leaving Rust crate package references deferred. |
| `MODULE.bazel` | `Completed` | Renamed Bazel module name to `datax`; `//codex-rs` path labels remain deferred to Phase 1.3. |
| `codex-rs/cli/Cargo.toml` | `Completed` | Renamed CLI binary target from the upstream executable name to `datax`; package/dependency crate names remain deferred to Phase 1.3. |
| `codex-rs/cli/BUILD.bazel` | `Completed` | Renamed Bazel release binary target to `datax`. |
| `codex-rs/app-server/BUILD.bazel` | `Completed` | Updated direct Bazel data dependency to the renamed CLI binary target. |
| `codex-rs/core/BUILD.bazel` | `Completed` | Updated direct Bazel data dependency to the renamed CLI binary target. |
| `codex-rs/rmcp-client/BUILD.bazel` | `Completed` | Updated direct Bazel data dependency to the renamed CLI binary target. |
| `codex-rs/tui/BUILD.bazel` | `Completed` | Updated direct Bazel data dependency to the renamed CLI binary target. |
| `codex-rs/docs/bazel.md` | `Completed` | Updated direct Bazel CLI target example to the renamed binary target. |
| `codex-rs/app-server-test-client/README.md` | `Completed` | Updated CLI binary build examples to `--bin datax`; app-server protocol examples remain deferred. |
| `codex-rs/cli/src/doctor.rs` | `Completed` | Updated npm package root diagnostics for the Datax npm package. |
| `codex-rs/cli/src/doctor/updates.rs` | `Completed` | Updated update command labels for Datax npm/bun/brew package names. |
| `codex-rs/tui/src/update_action.rs` | `Completed` | Updated update actions and standalone installer commands to Datax package and installer names. |
| `codex-rs/tui/src/npm_registry.rs` | `Completed` | Updated npm registry test fixture URL to the Datax package. |
| `codex-rs/tui/src/snapshots/codex_tui__update_prompt__tests__update_prompt_modal.snap` | `Completed` | Updated rename-only update prompt snapshot text. |
| `cliff.toml` | `Completed` | Updated changelog install command to the Datax npm package. |
| `scripts/debug-codex.sh` | `Completed` | Updated helper command to run the `datax` binary; script filename is deferred. |
| `scripts/run_tui_with_exec_server.sh` | `Completed` | Updated helper to start the renamed CLI binary. |
| `scripts/start-codex-exec.sh` | `Completed` | Updated helper to build the renamed CLI binary. |
| `scripts/test-remote-env.sh` | `Completed` | Updated helper to build and locate the renamed CLI binary. |
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
- `codex-rs/responses-api-proxy` package and npm artifact names: deferred because the response proxy remains a separate package whose full rename depends on Rust workspace stabilization and release workflow follow-up.
- `codex-app-server` binary/package names: deferred because app-server model and protocol rename is Phase 1.4.
- `codex-command-runner` and `codex-windows-sandbox-setup` helper binaries: deferred because they are owned by the Windows sandbox Rust crate and renaming them requires coordinated Rust source, tests, and packaging changes.
- App-server thread/turn/item public protocol names: deferred to Phase 1.4.
- Python package/module names such as `openai_codex`: deferred until SDK and generated protocol changes can be done together.
- TypeScript SDK package/API names: deferred until SDK and generated protocol changes can be done together.
- Historical upstream release URLs for already-published artifacts: retain only where the artifact cannot exist under Datax yet, and document each retained URL after implementation.
- `.github/codex/**` automation metadata: repository tooling provenance, not user-facing product package identity in this milestone.

## Public Surface Checklist

This milestone touches CLI executable naming, npm package names, install scripts, release artifact names, DotSlash manifests, package-builder output layout, selected docs that explain installation/build commands, and Bazel release output labels. It does not intentionally touch configuration file formats, app-server protocol methods, persisted session formats, generated app-server schemas, TypeScript bindings, Python SDK generated bindings, snapshots, or data engineering product behavior.

If `codex-rs/cli/Cargo.toml` changes the binary name from `codex` to `datax`, targeted CLI validation must prove `cargo run --bin datax -- --version` and the npm launcher path both work. If that binary rename reveals broad import or test coupling, record the dependency and either complete only the required direct references or defer with an explicit exception.

## Dependency Order

First, create and commit this ExecPlan so all implementation edits have a reviewed boundary. Second, create the GitHub issue and draft PR after the plan exists. Third, rename the npm CLI directory from `codex-cli` to `datax-cli`, rename the launcher from `bin/codex.js` to `bin/datax.js`, and update `package.json` plus package staging script references. Fourth, rename the standalone package builder directory and launcher from `scripts/codex_package` and `scripts/build_codex_package.py` to Datax equivalents, then update imports and package-builder tests. Fifth, rename installer scripts and release metadata that depend on the new package layout. Sixth, update root metadata, `pnpm-lock.yaml`, `README.md`, `docs/install.md`, `justfile`, `MODULE.bazel`, and focused Bazel release labels. Seventh, record formatter, build, and test commands in the Validation Matrix for the user's post-implementation test pass, then update the inventory statuses and exception register.

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

After implementation, record the validation commands listed below and paste concise evidence into this ExecPlan for checks that are intentionally run during the milestone. Commands marked `Deferred` are retained for the user's post-implementation migration test pass.

## Validation and Acceptance

Validation commands are documented per phase because builds are slow. For the remainder of Phase 1, test and build commands should be staged in each ExecPlan and deferred for the user's post-implementation batch run unless the user explicitly asks to run them during the phase. This Phase 1.2 plan keeps already-run lightweight checks as evidence, but future phase work should default to documenting commands rather than executing tests.

From the repository root, run the whitespace check and expect no output:

    git diff --check

From `codex-rs`, run the formatter and expect it to complete successfully:

    just fmt

From the repository root, run the formatter check and expect no formatting diffs:

    just fmt-check

From the repository root, run Python syntax validation for renamed packaging scripts and expect it to complete successfully:

    python3 -m py_compile datax-cli/scripts/build_npm_package.py scripts/stage_npm_packages.py scripts/build_datax_package.py scripts/datax_package/*.py

From the repository root, run package-builder unit tests and expect all tests in the renamed helper package to pass:

    python3 -m unittest discover -s scripts/datax_package -p 'test_*.py'

From `codex-rs`, run the targeted CLI crate tests and expect them to pass:

    just test -p codex-cli

From the repository root on Linux x64, run the staged local launcher smoke test and expect it to print `datax 0.0.0-dev`. This command stages the npm meta package and a temporary native vendor executable before invoking the launcher:

    launcher_stage=$(mktemp -d /tmp/datax-launcher-smoke.XXXXXX)
    python3 datax-cli/scripts/build_npm_package.py --version 0.0.0-dev --staging-dir "$launcher_stage"
    mkdir -p "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin"
    printf '#!/usr/bin/env sh\nprintf "datax 0.0.0-dev\\n"\n' > "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin/datax"
    chmod +x "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin/datax"
    node "$launcher_stage/bin/datax.js" --version

From the repository root, run the npm package staging smoke test and expect the staged `package.json` to expose `name: datax`, `bin.datax`, and Datax platform optional dependencies:

    tmpdir=$(mktemp -d /tmp/datax-npm-stage.XXXXXX)
    python3 datax-cli/scripts/build_npm_package.py --version 0.0.0-dev --staging-dir "$tmpdir"
    sed -n '1,120p' "$tmpdir/package.json"

From the repository root, run the forbidden mixed-case spelling search and expect no matches. The search term is split here only so the forbidden spelling is not checked into the plan:

    rg -n "Data""X" docs/plans .github datax-cli scripts codex-rs/cli codex-rs/tui README.md package.json pnpm-workspace.yaml MODULE.bazel

From the repository root, run the unresolved inventory-status search and expect no matches:

    rg -n '\| `[^`]+` \| `(Pending|In-Progress|Failed)`' docs/plans/datax_migration_phase1_2_product_rename/product_identity_rename_execplan.md

From the repository root, run the malformed rename-fragment search and expect no matches:

    rg -n "RUNNER_TEMP\}datax|unsigned-dmgdatax|signed-dmgdatax|releasedatax|npmdatax|openaidatax|datax-rs|datax-command-runner|datax-windows-sandbox" .github datax-cli scripts codex-rs docs README.md package.json pnpm-workspace.yaml MODULE.bazel justfile

Do not run the complete `just test` suite without user approval.

Acceptance for this milestone is that a reviewer can inspect staged package metadata and installer/release scripts and see Datax package names, `datax` executable paths, and Datax repository identity. Any remaining upstream product references must either be protected sandbox identifiers, upstream provenance, or explicitly deferred in this plan.

## Validation Matrix

| Command | Working Directory | Required | Status | Remarks Notes |
| --- | --- | --- | --- | --- |
| `git diff --check` | repository root | Yes | Deferred | Whitespace validation after edits; user will run in the post-implementation test pass unless requested earlier. |
| `just fmt` | `codex-rs` | Yes, after code changes | Completed | Initial sandboxed run failed because `uv` could not write its home cache; rerun with cache access passed. |
| `just fmt-check` | repository root | Yes | Deferred | Formatting check; user will run in the post-implementation test pass unless requested earlier. |
| `python3 -m py_compile datax-cli/scripts/build_npm_package.py scripts/stage_npm_packages.py scripts/build_datax_package.py scripts/datax_package/*.py` | repository root | Yes for renamed Python scripts | Completed | Passed. Generated `__pycache__` directories were removed after the check. |
| `python3 -m unittest discover -s scripts/datax_package -p 'test_*.py'` | repository root | Yes if package builder is renamed | Completed | Passed: 8 tests. |
| `just test -p codex-cli` | `codex-rs` | Yes if CLI binary/package changes | Deferred | Started, then interrupted at user request to defer long-running tests. Exact command retained for the post-implementation test pass. |
| Staged launcher smoke command from `Validation and Acceptance` | repository root | Best effort | Deferred | Direct source-tree launcher execution fails unless a native optional dependency or vendor directory is present; use the staged command above. |
| `python3 datax-cli/scripts/build_npm_package.py --version 0.0.0-dev --staging-dir $(mktemp -d /tmp/datax-npm-stage.XXXXXX)` | repository root | Yes if npm builder remains executable without native binaries | Completed | Passed and staged package metadata with `name: datax`, `bin.datax`, and Datax platform optional dependencies. |
| Search for the forbidden mixed-case spelling in `docs/plans`, package, release, CLI, TUI, and root metadata surfaces | repository root | Yes | Completed | Returned no matches after the initial plan wording was corrected. |
| Search for malformed rename fragments such as `datax-rs`, `npmdatax`, and `datax-command-runner` in package/release surfaces | repository root | Yes | Completed | Caught one workflow path typo and passed after correction. |

## Idempotence and Recovery

Directory renames should be performed with `git mv` so Git preserves history. If a validation command fails, keep the changed files in place, update the `Surprises & Discoveries` section with the failure, and fix the dependency that caused it before moving to the next rename band. If a generated lockfile changes unexpectedly, inspect the diff before accepting it; do not keep unrelated dependency updates. Rollback for this milestone is the branch itself: abandon the branch or revert the Phase 1.2 commits, with no database or external state migration required.

## Artifacts and Notes

GitHub issue:

    https://github.com/mbellary/datax/issues/3

GitHub draft PR:

    https://github.com/mbellary/datax/pull/4

Baseline:

    branch: datax/migration-phase1-2-product-rename
    base commit: 0b8fbfedc9e342895c5f6c074cc667ac8cd910cb
    Phase 1.1 PR: https://github.com/mbellary/datax/pull/2

## Interfaces and Dependencies

The Datax npm CLI package should expose a `datax` executable through `datax-cli/package.json` and `datax-cli/bin/datax.js`. The launcher should locate platform packages with Datax package names and execute `datax` or `datax.exe` from the native package vendor directory.

The standalone package builder should be importable as `scripts/datax_package` and launched through `scripts/build_datax_package.py`. Its canonical package layout should contain Datax-named metadata, resources, path directory, and executable entries. Tests should import the renamed helper package and assert Datax package variants.

The installer scripts should use `DATAX_RELEASE`, `DATAX_NON_INTERACTIVE`, `DATAX_INSTALL_DIR`, and `DATAX_HOME`, with `.datax` as the default product home. They should install or launch `datax`, not the upstream executable name. Protected sandbox identifiers are not part of these installer variables and must not be changed if encountered elsewhere.

Revision note, 2026-07-06: Initial Phase 1.2 ExecPlan added after branch creation and before implementation edits. This records the milestone boundary, inventory, dependency order, validation matrix, and deferrals so the product identity rename can proceed incrementally.

Revision note, 2026-07-06: Phase 1.2 implementation completed and plan updated for the staged-test process. Long-running Rust/build checks remain documented for the post-implementation migration test pass.
