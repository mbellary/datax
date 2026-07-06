# Phase 1.1 Repository Fork Preparation

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document follows `PLANS.md` from the repository root. It implements Phase 1.1 from `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` using the execution model in `docs/plans/Recommended-Datax-Migration-Execution-Model.md`.

## Purpose / Big Picture

Phase 1.1 establishes a reviewable migration baseline before any broad rename work begins. After this milestone, a contributor can identify the fork repository, the exact starting commit, the dedicated branch, the GitHub issue, the draft pull request, and the validation commands used to prove that the baseline was prepared without changing product behavior. This matters because every later rename milestone depends on a trustworthy starting point and a small, auditable trail of decisions.

The observable result is a repository-preparation-only pull request that adds this ExecPlan, records fork provenance, fixes the branch naming guidance for this migration, and does not rename product code, crates, binaries, protocols, fixtures, or snapshots.

## Progress

- [x] (2026-07-06 05:07Z) Read `PLANS.md`, the recommended execution model, and the provisional Phase 1 migration plan.
- [x] (2026-07-06 05:07Z) Created dedicated branch `datax/migration-phase1-1-repo-prep`.
- [x] (2026-07-06 05:07Z) Identified the Phase 1.1 file set and dependency order before implementation edits.
- [x] (2026-07-06 05:07Z) Updated suggested branch names in the recommended execution model to use the migration repository identity.
- [x] (2026-07-06 05:07Z) Create the repository provenance note.
- [x] (2026-07-06 05:08Z) Ran targeted validation for a documentation-only repository-preparation milestone.
- [x] (2026-07-06 05:09Z) Created GitHub issue #1.
- [x] (2026-07-06 05:10Z) Created draft pull request #2.
- [x] (2026-07-06 05:10Z) Updated this ExecPlan with final validation, issue, pull request, and outcome evidence.

## Surprises & Discoveries

- Observation: The initial branch name from the execution model used the generic app branch prefix instead of the migration repository identity.
  Evidence: The branch guidance in `docs/plans/Recommended-Datax-Migration-Execution-Model.md` was corrected before continuing with Phase 1.1.
- Observation: The repository has one root `justfile`; there is no separate `codex-rs/justfile`.
  Evidence: `sed -n '1,220p' codex-rs/justfile` returned `No such file or directory`, while the root `justfile` sets `working-directory := "codex-rs"`.
- Observation: Repository issues were disabled when Phase 1.1 started.
  Evidence: `gh issue create --repo mbellary/datax ...` returned `the 'mbellary/datax' repository has disabled issues`; `gh repo edit mbellary/datax --enable-issues` succeeded and issue #1 was then created.
- Observation: GitHub reported existing dependency alerts on the default branch during the first Phase 1.1 push.
  Evidence: `git push -u origin datax/migration-phase1-1-repo-prep` printed `GitHub found 58 vulnerabilities on mbellary/datax's default branch`. This milestone does not change dependencies, so the alerts are recorded but not addressed here.

## Decision Log

- Decision: Use `datax/migration-phase1-1-repo-prep` as the Phase 1.1 branch name.
  Rationale: Phase 1 is specifically correcting repository identity, so the branch name should match the migration target rather than the default app convention.
  Date/Author: 2026-07-06 / Codex.
- Decision: Keep Phase 1.1 documentation-only except for migration planning and provenance artifacts.
  Rationale: The provisional Phase 1 plan says repository preparation should establish the fork, branch, and upstream commit hash before broad product renaming starts. Product rename, crate rename, protocol rename, fixtures, and snapshots belong to later milestones.
  Date/Author: 2026-07-06 / Codex.

## Outcomes & Retrospective

Phase 1.1 established the repository-preparation baseline without changing product behavior. The milestone added this ExecPlan, added a repository provenance note, corrected migration branch guidance to use the Datax repository identity, enabled GitHub issues for the fork, created issue #1, pushed the dedicated branch, and opened draft PR #2.

No Rust code, package metadata, CLI behavior, app-server protocol, generated artifacts, fixtures, snapshots, config behavior, or persisted state changed. Existing dependency alerts reported by GitHub are outside this milestone and should be triaged separately.

## Context and Orientation

The repository is a fork at `/home/mbellary/wsl/projects/datax` with remote `origin` set to `https://github.com/mbellary/datax.git`. The starting branch for this milestone was `main`, and the starting commit was `4adf3ae2d88aa34e0e1aee11edd898846513f249`.

The term "provenance" means a short, checked-in record of where this fork started: repository URL, baseline commit, remote URL, and the reason no product behavior was changed in this milestone. The term "baseline" means the known repository state before broad rename work. For Phase 1.1, the baseline is captured through Git metadata, planning documents, and lightweight validation commands.

This milestone intentionally does not change Rust source files, package metadata, crate names, CLI names, app-server protocol names, persisted state, fixtures, snapshots, generated schemas, or TypeScript bindings. Those surfaces are listed here to make the boundary clear for future contributors.

## File Inventory

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `docs/plans/Recommended-Datax-Migration-Execution-Model.md` | `Completed` | Belongs to Phase 1.1 because it prescribed milestone branch names; updated suggested branches to use the migration repository identity. |
| `docs/plans/datax_migration_phase1_1_repo_prep/repository_fork_preparation_execplan.md` | `Completed` | This living ExecPlan tracks Phase 1.1 scope, progress, validation, issue, draft PR, and outcome evidence. |
| `docs/plans/datax_migration_phase1_1_repo_prep/repository_provenance.md` | `Completed` | Records fork baseline, remote, branch, starting commit, scope boundary, validation summary, and issue link. |
| `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` | `Not Required` | Inspected as the source of Phase 1.1 scope; no edit required for this milestone. |
| `docs/plans/Provision-Datax-Migration-Plan-Phase2.md` | `Not Required` | Inspected because the Phase 1 plan references Phase 2; no edit required before Phase 1.1 completes. |
| `PLANS.md` | `Not Required` | Inspected for ExecPlan requirements; no edit required. |
| `AGENTS.md` | `Not Required` | Governs repository workflow and validation; no edit required. |
| `README.md` | `Not Required` | Belongs to later product rename work; inspected only to confirm Phase 1.1 should not edit product-facing documentation. |
| `docs/install.md` | `Not Required` | Belongs to later install and product rename work; no edit required during repository preparation. |
| `justfile` | `Not Required` | Inspected to determine validation commands; no edit required. |
| `package.json` | `Not Required` | Belongs to later package rename work; no edit required during repository preparation. |
| `codex-cli/package.json` | `Not Required` | Belongs to later package rename work; no edit required during repository preparation. |
| `codex-rs/Cargo.toml` | `Not Required` | Belongs to later Rust workspace rename work; no edit required during repository preparation. |
| `MODULE.bazel` | `Not Required` | Belongs to later build metadata rename work; no edit required during repository preparation. |
| `BUILD.bazel` | `Not Required` | Belongs to later build metadata rename work; no edit required during repository preparation. |

## Baseline

The baseline branch before Phase 1.1 was `main`. The Phase 1.1 branch is `datax/migration-phase1-1-repo-prep`. The baseline commit is `4adf3ae2d88aa34e0e1aee11edd898846513f249`, with recent history showing migration planning commits followed by upstream fork history.

The remote named `origin` points to `https://github.com/mbellary/datax.git` for fetch and push. The working tree was clean before Phase 1.1 edits began. The user reported that the repository had already been installed and built from `docs/install.md` before this milestone started; this milestone will run documentation-appropriate validation locally and record any skipped heavy checks rather than repeating the full build without approval.

## Rename Exception Register

Phase 1.1 does not rename implementation identifiers, so retained `Codex`, `codex`, and `CODEX` references are expected throughout the source tree. They remain intentionally deferred to later milestones.

Protected sandbox identifiers that must remain unchanged throughout the migration:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`
- `CODEX_SANDBOX_ENV_VAR`

Additional retained references in Phase 1.1:

- Upstream provenance and fork history references may retain original names when they describe source history rather than the target product identity.
- Existing code, tests, packages, crate names, binary names, generated artifacts, fixtures, and snapshots retain current naming until the milestone assigned to that surface changes them.

## Public Surface Checklist

Phase 1.1 does not touch CLI names or arguments, config keys, app-server protocol methods or types, persisted data or session formats, package or crate names, generated schemas or types, UI strings, or snapshots.

The only public project-management surface touched in this milestone is branch naming guidance and repository provenance documentation. Later milestones must update this checklist for the surfaces they actually change.

## Dependency Order

The safe order for Phase 1.1 is:

1. Read `PLANS.md`, the recommended execution model, and the provisional Phase 1 plan.
2. Create the dedicated branch using the migration repository identity.
3. Identify the files that belong to repository preparation and decide which require modification.
4. Correct the execution model branch names so future milestones use the same branch convention.
5. Create this ExecPlan with a file inventory, baseline, exception register, public surface checklist, dependency order, validation matrix, and recovery note.
6. Add a repository provenance note recording remote, branch, and commit baseline.
7. Run documentation-appropriate validation.
8. Create the GitHub issue and draft pull request.
9. Update this ExecPlan and the provenance note with final evidence.

This order avoids editing implementation files before the milestone boundary is explicit.

## Plan of Work

Update `docs/plans/Recommended-Datax-Migration-Execution-Model.md` so suggested branch names start with `datax/`. Create `docs/plans/datax_migration_phase1_1_repo_prep/repository_fork_preparation_execplan.md` as the living plan for this milestone. Create `docs/plans/datax_migration_phase1_1_repo_prep/repository_provenance.md` as the migration note that records where the fork baseline started.

Do not edit Rust code, JavaScript package files, Bazel files, generated artifacts, fixtures, snapshots, app-server protocol files, CLI behavior, or config behavior in this milestone.

## Concrete Steps

Run commands from `/home/mbellary/wsl/projects/datax` unless another working directory is stated.

Create the dedicated branch:

    git switch -c datax/migration-phase1-1-repo-prep

Record baseline metadata:

    git rev-parse HEAD
    git remote -v
    git status --short --branch

Validate the documentation-only milestone:

    git diff --check
    rg -n "forbidden spelling pattern" docs/plans

The second validation command in this section is intentionally described generically because the forbidden mixed-case spelling should not be written into checked-in plan files. The actual command run during implementation must search for that exact spelling in checked-in files and report no matches.

Create GitHub artifacts after local validation:

    gh issue create --repo mbellary/datax --title "Phase 1.1: repository fork preparation" --body-file <temporary issue body>
    git push -u origin datax/migration-phase1-1-repo-prep
    gh pr create --repo mbellary/datax --draft --base main --head datax/migration-phase1-1-repo-prep --title "Phase 1.1: repository fork preparation" --body-file <temporary PR body>

## Validation Matrix

| Command | Required For Phase 1.1 | Status | Remarks Notes |
| --- | --- | --- | --- |
| `git status --short --branch` | Yes | Completed | Confirmed the milestone started on a clean branch before edits. |
| `git diff --check` | Yes | Completed | Passed with no output. |
| Search checked-in plan files for the forbidden mixed-case spelling | Yes | Completed | Passed with no matches. |
| `just fmt-check` | Yes | Completed | First sandboxed run failed because `uv` could not write to the home cache; rerun with approved permissions passed. |
| `just fmt` from `codex-rs` | No | Not Required | No code changes are planned in Phase 1.1. |
| `just test -p <crate>` | No | Not Required | No crate changes are planned in Phase 1.1. |
| Full `just test` | No | Not Required | Complete suite is intentionally deferred because Phase 1.1 is documentation-only and full suite requires approval. |

## Validation and Acceptance

Phase 1.1 is accepted when the branch exists, the file inventory has no unresolved rows, the provenance note records the starting commit and remote, the branch guidance is corrected, documentation validation passes, the GitHub issue exists, and the draft pull request exists.

The human-visible proof is:

- `git status --short --branch` shows `datax/migration-phase1-1-repo-prep`.
- `git diff --check` exits successfully.
- The exact search for the forbidden mixed-case spelling returns no matches in checked-in plan files.
- The GitHub issue and draft pull request URLs are recorded in this ExecPlan and the provenance note.

## Idempotence and Recovery

All Phase 1.1 edits are documentation or planning metadata. If a validation command fails, fix the relevant document and rerun the same command. If the branch or pull request must be recreated, keep the same milestone scope and update the artifact links in this ExecPlan.

Rollback is low risk: revert the documentation edits in this branch, close the draft pull request if it was opened, and keep `main` unchanged. No generated artifacts, lockfiles, crates, package metadata, or persisted data are changed in this milestone.

## Artifacts and Notes

Starting metadata:

    branch: datax/migration-phase1-1-repo-prep
    baseline commit: 4adf3ae2d88aa34e0e1aee11edd898846513f249
    origin: https://github.com/mbellary/datax.git

GitHub issue: https://github.com/mbellary/datax/issues/1.

Draft pull request: https://github.com/mbellary/datax/pull/2.

## Interfaces and Dependencies

No runtime interfaces, Rust traits, app-server methods, CLI commands, package dependencies, or generated schemas are added or changed in this milestone.

This milestone depends only on Git metadata, checked-in planning documents, and GitHub project-management artifacts. Later implementation milestones depend on this one for branch naming, provenance, and the rule that each migration band must be independently planned, inventoried, validated, and reviewed.

Revision note, 2026-07-06: Created the Phase 1.1 ExecPlan before implementation edits beyond branch guidance so the milestone can be resumed from this file alone.
