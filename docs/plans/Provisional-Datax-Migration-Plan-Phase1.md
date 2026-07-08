# Provisional Datax Migration Plan: Fork-First Rename Migration

## Summary
Datax will start as a dedicated fork of the Codex codebase. Phase 1 is a strict migration-only phase: rename the product and core public concepts, prove the fork builds and tests in isolation, and avoid all product enhancements or Codex downstream integration work.

The strategy is:

- Fork Codex into a dedicated `datax` repository.
- Rename `codex` / `Codex` / `CODEX` references to `datax` / `Datax` / `DATAX` where they represent product, crate, package, binary, protocol, config, docs, or UI identity.
- Rename conceptual app-server entities:
  - `Thread` → `Chat`
  - `Turn` → `Interaction`
  - `Item` → `Message`
- Keep behavior equivalent during migration.
- Do not add data engineering product features in Phase 1.
- Do not add Codex-as-downstream integration in Phase 1.
- After Phase 1 is stable, start Phase 2 product evolution using the earlier Datax platform plan.

Discovery gate for this migration scope:

- Architecture discovery: complete.
- Implementation discovery: complete enough for a migration execution plan.
- Class A unresolved blockers: 0.
- Class B unresolved choices: 0, with assumptions listed below.
- Allowed output type: provisional implementation-ready migration plan.

## Migration Principles
Phase 1 success is measured by isolation and parity, not new capability.

Required invariants:

- The new repo is named `datax`.
- The build outputs are named Datax, not Codex.
- Rust crate names use `datax-*`, not `codex-*`.
- CLI commands, package names, generated schemas, TypeScript bindings, docs, test fixtures, snapshots, and visible UI strings use Datax naming.
- App-server public API uses `chat`, `interaction`, and `message` terminology.
- The migrated Datax repo compiles and tests without depending on the original Codex checkout.
- No deliberate feature behavior changes are introduced except naming and source-of-truth rename changes required by the migration.

Do not implement in Phase 1:

- No data engineering planning/build/deploy/schedule/monitor features.
- No new Codex adapter.
- No attempt to track upstream Codex changes.
- No compatibility layer for existing Codex persisted state unless required to keep tests passing.
- No broad refactors unrelated to the rename.

## Rename Scope
Product identity rename:

- `codex` → `datax`
- `Codex` → `Datax`
- `CODEX` → `DATAX`
- `openai/codex` references should become the Datax repository identity where they represent the forked product.
- Keep third-party or historical references unchanged only when the text is explicitly describing upstream provenance, license history, or an external package name that cannot be renamed.

Rust workspace rename:

- Rename crate packages from `codex-*` to `datax-*`.
- Rename internal Rust module references and imports accordingly.
- Rename binaries from `codex` to `datax`.
- Rename CLI-visible app names and help text.
- Update `Cargo.toml`, `Cargo.lock`, Bazel metadata, build scripts, schema generation paths, and package metadata.
- Keep module boundaries intact unless a rename makes a path invalid.

App-server concept rename:

- `Thread` → `Chat`
- `Turn` → `Interaction`
- `Item` → `Message`

Wire/API rename:

- `thread/start` → `chat/start`
- `thread/read` → `chat/read`
- `thread/list` → `chat/list`
- `thread/resume` → `chat/resume`
- `thread/fork` → `chat/fork`
- `turn/start` → `interaction/start`
- `turn/interrupt` → `interaction/interrupt`
- `item/*` → `message/*`

Type rename examples:

- `Thread` → `Chat`
- `ThreadId` → `ChatId`
- `ThreadItem` → `ChatMessage` or `Message`, preferring `Message` at the public API boundary.
- `Turn` → `Interaction`
- `TurnId` → `InteractionId`
- `TurnStatus` → `InteractionStatus`
- `ThreadStartParams` → `ChatStartParams`
- `TurnStartParams` → `InteractionStartParams`

Filesystem/config rename:

- `.codex` → `.datax`
- `codex.toml` or Codex-specific config names → `datax.toml`.
- `CODEX_HOME` → `DATAX_HOME`
- Other `CODEX_*` environment variables → `DATAX_*`, except sandbox variables explicitly protected by repo instructions.

Protected exception:

- Do not rename or modify code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.

## Implementation Phases
1. Repository fork preparation
   - Create the dedicated `datax` repository from the current Codex codebase.
   - Preserve commit history if possible.
   - Establish the initial branch as a migration branch.
   - Record upstream Codex commit hash in a migration note or ADR.

2. Mechanical product rename
   - Rename repository metadata, package metadata, CLI binary names, npm/package names if present, Rust crate names, build labels, install scripts, and generated artifact names.
   - Update user-visible strings from Codex to Datax.
   - Update docs that are part of the repo build or app-server API contract.
   - Avoid product documentation expansion.

3. Rust workspace stabilization
   - Rename `codex-*` crates to `datax-*`.
   - Update imports, crate paths, workspace membership, Bazel labels, lockfiles, and generated files.
   - Keep code movement minimal.
   - Fix compile errors caused by rename drift before doing any semantic cleanup.

4. App-server model rename
   - Rename public protocol resources from thread/turn/item to chat/interaction/message.
   - Rename request, response, notification, and schema types.
   - Regenerate app-server schemas and TypeScript bindings.
   - Update app-server README/API examples to match Datax names.

5. Persistence and fixture rename
   - Rename persisted paths and config defaults from Codex to Datax.
   - Update test fixtures, snapshots, golden files, and schema fixtures.
   - Decide fixture-by-fixture whether upstream historical text should remain as provenance or become Datax product text.

6. Test isolation pass
   - Run formatting.
   - Run targeted crate tests after each major rename band.
   - Run full test suite once common/core/protocol rename work is stable.
   - Fix only migration-caused failures.

7. Migration freeze checkpoint
   - Produce a migration report listing renamed surfaces, skipped exceptions, failing or skipped tests, and known follow-up risks.
   - Tag or branch the first compiling/tested Datax baseline.
   - Only after this checkpoint, begin Phase 2 product evolution.

## Public API And Interface Changes
Phase 1 intentionally changes naming but not behavior.

Expected public API changes:

- Datax app-server exposes `chat/*`, `interaction/*`, and `message/*` methods instead of `thread/*`, `turn/*`, and `item/*`.
- Generated TypeScript types use Datax names.
- CLI uses `datax`, not `codex`.
- Config and state directories use Datax names.
- Environment variables use `DATAX_*`, except protected sandbox internals.
- Logs, telemetry labels, app titles, and service names use Datax naming.


Compatibility policy for Phase 1:

- No backwards compatibility with existing Codex clients is required.
- No migration of existing Codex local state is required.
- No aliases such as `codex`, `thread/start`, or `CODEX_HOME` are required unless needed temporarily inside tests.
- Any temporary alias must be marked as migration-only and removed before Phase 1 completion.

## Test Plan
Required validation sequence:

The concrete Phase 1.7 freeze checklist, including exact commands and expected
evidence, is maintained in
`docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md`.

1. Static search checks
   - No product-owned `codex`, `Codex`, or `CODEX` references remain except documented exceptions.
   - No public protocol `Thread`, `Turn`, or `Item` names remain.
   - Protected sandbox identifiers remain unchanged.

2. Formatting and generation
   - Run repository formatter.
   - Regenerate app-server schema fixtures.
   - Regenerate TypeScript bindings if the repo requires checked-in generated output.
   - Regenerate config schema if config types changed.

3. Rust tests
   - Run targeted tests for renamed crates as each subsystem stabilizes.
   - Run app-server protocol tests after API rename.
   - Run app-server integration tests through public JSON-RPC API.
   - Run full suite before declaring Phase 1 complete.

4. CLI smoke tests
   - `datax --help` works.
   - `datax app-server --stdio` starts and initializes.
   - App-server accepts `chat/start` and `interaction/start`.
   - Notifications use `message/*` naming.

5. Snapshot and fixture tests
   - Update intentional UI/text snapshots.
   - Review all snapshot changes before accepting.
   - Keep snapshot changes limited to rename effects.

Phase 1 acceptance criteria:

- Fresh clone of `datax` builds without the original Codex repo.
- All required generated artifacts are updated.
- Tests pass or have documented, migration-specific skips.
- The main executable is `datax`.
- Public app-server protocol uses Chat/Interaction/Message.
- No Datax product features were added during the migration.

## Risks And Controls
Rename blast radius:

- Use staged rename bands instead of one huge semantic rewrite.
- Keep each stage reviewable and testable.
- Prefer mechanical commits for mechanical changes.

Protocol breakage:

- Treat app-server rename as a deliberate breaking change.
- Regenerate schema and TypeScript artifacts in the same stage.
- Test via JSON-RPC public API, not only Rust internals.

Hidden Codex references:

- Add automated `rg` checks for forbidden public names.
- Maintain an exception list for protected sandbox identifiers, license/provenance text, and unavoidable external names.

Generated artifact drift:

- Regenerate schemas and bindings after protocol/type rename.
- Include generated changes in the same migration stage as their source changes.

Behavior drift:

- No feature work during Phase 1.
- Fix only rename-induced compile, test, path, fixture, and packaging issues.
- Defer cleanup refactors unless they are necessary to make the renamed code compile.

## Phase 2 Starts After Migration
Once Phase 1 is complete, Datax product evolution resumes from the clean Datax baseline.
Phase 2 Plan : Provision-Datax-Migration-Plan-Phase2.md

Phase 2 should introduce:

- Data engineering domain model: Plan, Workflow, Deployment, Schedule, Execution, Monitor, Artifact, Approval.
- Datax-specific UI and app-server workflows.
- Decisions about whether Codex remains internal forked runtime code, external downstream app-server, or both during transition.
- Optional Codex adapter work only after the migration baseline is stable.

The earlier external Codex adapter plan remains strategically useful, but it is explicitly deferred until after the fork-and-rename migration is complete.

## Assumptions And Defaults
Chosen defaults for this provisional migration plan:

- Datax starts as a fork of the current Codex codebase.
- Phase 1 is migration-only.
- The target repo is dedicated to Datax and does not need Codex client compatibility.
- Datax can make breaking API changes from Codex naming to Datax naming.
- SQLite/local state compatibility with existing Codex installs is not required.
- The migration preserves behavior unless a naming change forces a visible protocol or path change.
- Protected Codex sandbox environment identifiers remain unchanged.
- Product enhancements begin only after the migrated Datax repo builds and tests in isolation.
