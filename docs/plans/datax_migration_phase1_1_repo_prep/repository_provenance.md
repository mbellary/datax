# Phase 1.1 Repository Provenance

This note records the repository baseline for Phase 1.1 of the Datax migration. It is intentionally limited to fork preparation and does not rename product code, crates, binaries, protocols, fixtures, snapshots, generated artifacts, or persisted state.

## Baseline

- Repository path: `/home/mbellary/wsl/projects/datax`
- Remote: `origin` -> `https://github.com/mbellary/datax.git`
- Starting branch: `main`
- Milestone branch: `datax/migration-phase1-1-repo-prep`
- Baseline commit: `4adf3ae2d88aa34e0e1aee11edd898846513f249`
- Baseline commit summary: `update migration plan`
- Current migration plan: `docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`
- Execution model: `docs/plans/Recommended-Datax-Migration-Execution-Model.md`
- Phase 1.1 ExecPlan: `docs/plans/datax_migration_phase1_1_repo_prep/repository_fork_preparation_execplan.md`

## Scope Boundary

Phase 1.1 prepares the fork for staged migration work. The only checked-in changes expected in this milestone are migration planning, branch guidance, and this provenance note.

The following surfaces are intentionally unchanged until later milestones:

- Rust crate names and imports.
- CLI binary names and visible help text.
- JavaScript package names.
- Bazel labels and lockfiles.
- App-server protocol names.
- Config keys and default paths.
- Fixtures, snapshots, and generated schemas.
- User-visible product strings outside migration planning documents.

## Validation Summary

Local validation completed for the documentation-only Phase 1.1 scope:

- `git diff --check` passed with no output.
- The checked-in plan files were searched for the forbidden mixed-case spelling, and no matches were found.
- `just fmt-check` passed after rerunning outside the sandbox cache restriction. The first sandboxed attempt failed because `uv` could not create files under the home cache path.

GitHub issue: https://github.com/mbellary/datax/issues/1

Draft pull request: Pending.

Revision note, 2026-07-06: Created as the Phase 1.1 migration note required by the provisional Phase 1 plan.
