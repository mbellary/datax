# Recommended Datax Migration Execution Model

## Purpose

This document records the agreed execution model for implementing
`docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`.

Phase 1 is a migration-only effort. The goal is to move the fork from Codex
identity to Datax identity while preserving behavior, buildability, and test
coverage. Product evolution, data engineering features, and Codex downstream
adapter work remain out of scope until the migration baseline is stable.

## Naming Convention

Use `Datax` as the standard product spelling in Rust types, crate names,
package metadata, UI strings, and generated artifacts unless a later explicit
branding decision changes it.

Plan filenames, implementation names, UI text, generated artifacts, and other
checked-in references should use `Datax`.

## Phase 1 Structure

The provisional migration plan describes seven implementation phases. For
reviewability, treat these as Phase 1 milestones:

1. Phase 1.1: Repository fork preparation.
2. Phase 1.2: Mechanical product rename.
3. Phase 1.3: Rust workspace stabilization.
4. Phase 1.4: App-server model and protocol rename.
5. Phase 1.5: Persistence, fixtures, and snapshots.
6. Phase 1.6: Test isolation pass.
7. Phase 1.7: Migration freeze checkpoint.

Each milestone should be independently reviewable and testable. Avoid combining
large rename bands into one pull request unless a later discovery shows that a
split would create more risk than it removes.

## Per-Milestone Requirements

For each Phase 1 milestone:

- Create a PLANS.md-compliant ExecPlan under `docs/plans/`.
- Before editing implementation files, identify the complete set of files that
  belong to the milestone, resolve ordering and dependency constraints, and
  then identify which files actually require modification or updates.
- Include a file inventory table in the ExecPlan with these columns:
  `Filename`, `Modified`, and `Remarks Notes`. The `Filename` value must
  include the path to the file.
- Create a dedicated git branch using the repository branch convention.
- Create a GitHub issue that describes the milestone scope and acceptance
  criteria.
- Create a draft pull request early so progress, CI, and review notes are
  attached to the milestone.
- Implement incrementally and update the ExecPlan as a living document.
- Document targeted build and test commands for the crates or packages changed.
- Defer test and build execution until the post-implementation migration test
  pass unless the user explicitly asks to run a command during the milestone.
- Run `just fmt` from `codex-rs` after code changes.
- Run `just fix -p <project>` before finalizing substantial Rust changes.
- Ask before running the complete `just test` suite.

The file inventory table should be updated as discovery continues. Use
`Modified` values of `Pending`, `In-Progress`, `Completed`, `Failed`, or
`Not Required`. Use `Not Required` for files that belong to the milestone or
were inspected for dependency reasons but do not require modification.

Example file inventory table:

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `path/to/file.rs` | `Pending` | Belongs to this milestone; dependency impact still being evaluated. |
| `path/to/generated.json` | `Pending` | Regenerate only after the source schema change is complete. |
| `path/to/inspected.md` | `Not Required` | Inspected because it references the renamed surface; no edit required. |

## Per-Milestone Exit Criteria

A Phase 1 milestone is complete only when:

- The ExecPlan is current, including progress, decisions, discoveries, and
  outcome notes.
- The file inventory table has no unresolved `Pending`, `In-Progress`, or
  `Failed` rows.
- All files marked `Completed` have corresponding implementation, generated
  artifact, fixture, snapshot, or metadata updates in the milestone branch.
- All files marked `Not Required` include a clear reason in `Remarks Notes`.
- The Rename Exception Register is current and every retained Codex or codex
  reference is intentional.
- The Public Surface Checklist has been reviewed and any touched surfaces have
  matching tests, generated artifacts, compatibility notes, or deferrals.
- The Validation Matrix commands required for the milestone are recorded
  exactly, including deferred commands that the user will run during the
  post-implementation migration test pass.
- The dedicated branch, GitHub issue, and draft pull request exist and point to
  the same milestone scope.
- The pull request is reviewable as a migration-only change and does not
  include unrelated product features or broad refactors.

## Required Tracking Sections

Each milestone ExecPlan should include these tracking sections before
implementation begins:

- Baseline: record the starting branch, upstream or fork commit, current build
  state, and any known pre-existing failures.
- Rename Exception Register: list identifiers and files that must retain Codex
  or codex naming because they are protected sandbox identifiers, upstream
  provenance, license text, external package names, compatibility aliases, or
  public surfaces intentionally deferred to a later milestone.
- Public Surface Checklist: mark whether the milestone touches CLI
  names or arguments, config keys, app-server protocol, persisted data or
  session formats, package or crate names, generated schemas or types, UI
  strings, or snapshots.
- Dependency Order: record the safe implementation order, including source
  changes before generated artifacts and dependency metadata before import
  updates when applicable.
- Validation Matrix: list the exact build, format, generation, and test
  commands planned for the milestone, including which checks are targeted and
  which require approval because they run the complete suite.
- Rollback or Recovery Note: describe the expected rollback path for the
  milestone, including any generated artifacts, lockfiles, or renamed files
  that need special attention.
- Open Questions and Decision Log: record naming decisions, compatibility
  choices, deferred work, and discoveries that affect implementation scope.

## Suggested Branches

Use these branch names unless a later milestone needs a more precise name:

- `datax/migration-phase1-1-repo-prep`
- `datax/migration-phase1-2-product-rename`
- `datax/migration-phase1-3-rust-workspace`
- `datax/migration-phase1-4-app-server-protocol`
- `datax/migration-phase1-5-persistence-fixtures`
- `datax/migration-phase1-6-test-isolation`
- `datax/migration-phase1-7-freeze-report`

## Incremental Validation Strategy

Build and test costs are expected to be high, so validation should be staged.

Record narrow checks after each meaningful rename band. Prefer documenting
crate-specific commands such as `just test -p <crate>` over workspace-wide
commands while the change is still in progress. The user will run the recorded
commands after the implementation phases are complete and report results. Run
commands during a milestone only when the user explicitly approves or requests
that specific validation.

When generated artifacts are affected, regenerate them in the same milestone as
the source shape change. For app-server protocol changes, run the app-server
schema generation and protocol tests in the same pull request.

## Phase 1 Migration Exit Criteria

Phase 1 migration is complete only when:

- All Phase 1 milestones are complete and their pull requests are merged or
  otherwise explicitly closed with replacement work recorded.
- The final codebase, build metadata, generated artifacts, fixtures, snapshots,
  and user-visible strings use Datax naming except for entries documented in
  the final Rename Exception Register.
- No forbidden mixed-case spelling with a capital trailing X remains in
  checked-in files.
- Protected sandbox identifiers remain unchanged.
- Rust workspace metadata, crate imports, Bazel labels, package metadata,
  lockfiles, generated schemas, and TypeScript bindings are internally
  consistent.
- Persistence, session, fixture, and snapshot changes are covered by the
  milestone validation evidence or documented compatibility decisions.
- Targeted milestone checks have passed, and the final approved stabilization
  checks have been run or explicitly deferred with a recorded reason.
- The migration freeze checkpoint records the final baseline, remaining
  exceptions, known risks, and follow-up work for post-migration product
  development.

## Protected Exceptions

Do not rename or modify code related to:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`
- `CODEX_SANDBOX_ENV_VAR`

These identifiers are intentionally tied to sandbox behavior and must remain
unchanged throughout the migration.

## Review Boundaries

Phase 1 pull requests should contain migration work only. Defer the following:

- Data engineering domain features.
- Workflow, deployment, schedule, monitor, approval, or artifact features.
- Codex adapter or upstream tracking work.
- Broad refactors that are not required to make the renamed code compile and
  pass tests.
- Product documentation expansion outside build-critical or API-contract docs.

## First Milestone Recommendation

Begin with Phase 1.1, repository fork preparation.

The first ExecPlan should record repository provenance, identify the upstream
Codex commit hash, confirm the current build baseline, and create the branch,
issue, and draft pull request that will carry the first migration milestone.
It should not start broad product renaming.
