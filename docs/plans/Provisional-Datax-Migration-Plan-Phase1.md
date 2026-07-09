# Provisional Datax Migration Plan: Fork-First Rename Migration

## Summary
Datax will start as a dedicated fork of the Codex codebase. Phase 1 is a strict migration-only phase: rename the product and core public concepts, prove the fork builds and tests in isolation, and avoid all product enhancements or Codex downstream integration work.

Phase 1 must leave the repository in a coherent Datax baseline before Phase 2 begins. Remaining Codex references are acceptable only when they are intentional, documented, and tied to upstream provenance, protected sandbox behavior, compatibility boundaries, or the future downstream Codex app-server integration.

The strategy is:

- Fork Codex into a dedicated `datax` repository.
- Rename `codex` / `Codex` / `CODEX` references to `datax` / `Datax` / `DATAX` where they represent product, crate, package, binary, protocol, config, docs, or UI identity.
- Rename Datax-owned source path references such as `codex-rs` to `datax-rs` before Phase 1 closes.
- Rename conceptual app-server entities:
  - `Thread` → `Chat`
  - `Turn` → `Interaction`
  - `Item` → `Message`
- Keep behavior equivalent during migration.
- Do not add data engineering product features in Phase 1.
- Do not add Codex-as-downstream integration in Phase 1.
- Keep the current Datax folder layout intact during Phase 1. Do not restructure the repository just to make the layout look cleaner.
- After Phase 1 is stable, start Phase 2 product evolution from the migrated Datax baseline.

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
- Datax-owned source paths use Datax naming, including `datax-rs` instead of `codex-rs`.
- App-server public API uses `chat`, `interaction`, and `message` terminology.
- The migrated Datax repo compiles and tests without depending on the original Codex checkout.
- No deliberate feature behavior changes are introduced except naming and source-of-truth rename changes required by the migration.
- The repository remains fork-first: migrate Codex to Datax, then evolve Datax into the data-engineering product.
- Migration cleanup belongs in Phase 1. Phase 2 should not start with unresolved product identity ambiguity.
- Public, wire, schema, generated, CLI-visible, and documentation names must use Datax concepts before Phase 1 closes.
- Internal implementation names inherited from Codex must be inventoried and classified before Phase 1 closes, even when they are not automatically renamed.

Do not implement in Phase 1:

- No data engineering planning/build/deploy/schedule/monitor features.
- No new Codex adapter.
- No attempt to track upstream Codex changes.
- No compatibility layer for existing Codex persisted state unless required to keep tests passing.
- No broad refactors unrelated to the rename.
- No broad folder restructuring of Datax-owned crates, packages, or tools.

Phase 1 completion standard:

- This repository is Datax.
- Remaining Codex references are intentional and documented.
- Datax-owned build, package, test, schema, CLI, and app-server surfaces no longer accidentally point at Codex.
- Public app-server and generated API surfaces do not expose Codex-era `thread`, `turn`, or `item` naming.
- Internal Codex-era identifiers such as `thread_id`, `turn_id`, `codex_thread`, or `codex_turns` are either renamed or explicitly classified.
- Any unresolved Codex naming is either a protected exception, an upstream provenance reference, or a tracked follow-up with an owner and reason.

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

Top-level source path rename:

- Rename the Datax-owned Rust source directory from `codex-rs` to `datax-rs`.
- Treat `codex-rs` as migration debt, not as an intentional downstream Codex boundary.
- Inventory all `codex-rs` path references before moving the directory.
- Update references in `justfile`, `package.json`, `.github`, build scripts, release scripts, docs, schema generation commands, Bazel metadata, and helper scripts.
- Use `git mv` for the directory move so history is easier to follow.
- Keep this as a dedicated Phase 1 cleanup milestone because it has high build, CI, packaging, and documentation blast radius.

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

Internal name audit:

- Inventory internal identifiers that still use Codex-era concepts, including `thread_id`, `turn_id`, `ThreadId`, `TurnId`, `codex_thread`, `codex_turn`, and `codex_turns`.
- Public API, wire payload, generated schema, TypeScript binding, CLI-visible, fixture, and documentation occurrences must be renamed to Datax concepts.
- Internal-only occurrences may remain temporarily only when they are not serialized, not visible to clients, and not part of a public crate API.
- Each remaining internal-only occurrence must be classified as one of: rename now, protected exception, downstream Codex boundary, compatibility shim, historical/provenance, or deferred with explicit reason.
- Do not perform a blanket mechanical rewrite of internal names. Rename dependency-aware after identifying the owner type, serialization boundary, public exposure, tests, generated files, and call graph.
- Do not rename protected sandbox identifiers as part of this audit.

Filesystem/config rename:

- `.codex` → `.datax`
- `codex.toml` or Codex-specific config names → `datax.toml`.
- `CODEX_HOME` → `DATAX_HOME`
- Other `CODEX_*` environment variables → `DATAX_*`, except sandbox variables explicitly protected by repo instructions.

Protected exception:

- Do not rename or modify code related to `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` or `CODEX_SANDBOX_ENV_VAR`.

Intentional Codex boundary exception:

- Phase 1 may document future downstream Codex app-server integration boundaries, but it must not implement the adapter.
- While scanning the codebase in Phase 1, record downstream Codex artifacts that Phase 2 should evaluate for adapter/runtime work.
- Future downstream Codex integration should be isolated behind a Datax-owned agent adapter boundary.
- If a dedicated implementation area is needed later, prefer names such as `codex-runtime` for downstream Codex app-server process/client/protocol integration and `codex-compat` for compatibility shims.
- Do not use `codex-core` as the new boundary name. It conflicts with inherited crate meaning and incorrectly implies that Codex remains the core of the Datax product.
- Do not move existing Datax-owned folders as part of this boundary decision. Only Codex downstream or compatibility code should move into the dedicated boundary when Phase 2 implementation starts.

Downstream Codex artifact note:

- Phase 1 should record, not implement, artifacts that look relevant to future downstream Codex app-server integration.
- Examples include app-server process launch points, JSON-RPC transport code, protocol client/server types, schema generation outputs, SDK/package surfaces, external service URLs, upstream release artifact URLs, compatibility shims, and runtime process-management scripts.
- Each discovered artifact should be classified as candidate `codex-runtime`, candidate `codex-compat`, upstream provenance, external dependency, protected exception, or unrelated.
- Phase 2 owns adapter/runtime implementation decisions for these artifacts.

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

7. Phase 1.7: Migration freeze checkpoint and remaining identity cleanup
   - Produce a migration report listing renamed surfaces, skipped exceptions, failing or skipped tests, and known follow-up risks.
   - Tag or branch the first compiling/tested Datax baseline.
   - Run the focused remaining-Codex-reference audit.
   - Classify every remaining Codex reference as fixed, protected, provenance, downstream-boundary, compatibility-boundary, or deferred.
   - Resolve any remaining Datax-owned `codex`, `Codex`, or `CODEX` references discovered by the freeze checkpoint.
   - Run a focused internal-name audit for `thread_id`, `turn_id`, `ThreadId`, `TurnId`, `codex_thread`, `codex_turn`, and `codex_turns`.
   - Rename any public, wire, generated, CLI-visible, fixture, or documentation occurrences found by that audit.
   - Classify any remaining internal-only occurrences before closing the checkpoint.
   - Inventory every `codex-rs` path reference before modifying files.
   - Rename `codex-rs` to `datax-rs`.
   - Update all path references required by build, test, schema generation, packaging, release, CI, and docs.
   - Keep protected sandbox identifiers unchanged.
   - Keep upstream provenance references only when they clearly refer to original source, license history, or external upstream project identity.
   - Keep downstream Codex references only when they refer to the future Codex app-server integration boundary.
   - Record downstream Codex artifacts discovered during scanning for Phase 2 adapter/runtime planning.
   - Record every intentional exception in the Phase 1 freeze checklist or a dedicated exceptions table.
   - Do not use this stage to introduce Datax product features or to restructure existing Datax folders.
   - Do not use the `codex-rs` rename as permission to move unrelated Datax-owned crates or redesign the repository layout.
   - Only after this checkpoint, begin Phase 2 product evolution.

8. Phase 1.8: Mechanical protocol/domain migration
   - Execute the mechanical protocol/domain migration plan maintained in
     `docs/plans/datax_mechanical_protocol_migration/mechanical_protocol_migration_execplan.md`.
   - Treat the following mapping as the source-of-truth invariant:
     `Codex` → `Datax`, `Thread` → `Chat`, `Turn` → `Interaction`, and
     `Item` → `Message`.
   - Apply the mapping compositionally to Datax-facing names. Examples:
     `ThreadManager` → `ChatManager`, `ThreadId` → `ChatId`,
     `TurnItem` → `InteractionMessage`, and `RolloutItem` →
     `RolloutMessage`.
   - Preserve existing runtime capabilities. This phase renames and aligns
     Datax-facing contracts; it does not remove managers, live chat/session
     handles, persistence, event streams, resumability, or history machinery.
   - Keep compatibility aliases, provenance references, protected sandbox
     identifiers, and future downstream Codex bridge terms explicit and
     isolated.
   - Do not add data engineering product features or downstream Codex app-server
     integration in this phase.
   - Only after Phase 1.8 is complete, begin Phase 2 product evolution and
     downstream adapter/runtime implementation.

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

Downstream Codex policy for Phase 1:

- Codex is not a runtime dependency during Phase 1.
- Datax app-server must not call a downstream Codex app-server during Phase 1.
- `codex-rs` is not a downstream Codex boundary. It is a Datax-owned source path and should become `datax-rs`.
- Downstream Codex artifacts discovered during Phase 1 scanning should be noted for Phase 2; they should not be wired into Datax runtime behavior in Phase 1.
- Phase 1 may reserve terminology for future boundaries:
  - `AgentAdapter` or `AgentRuntime` for the Datax-owned abstraction.
  - `codex-runtime` for downstream Codex app-server client/process/protocol integration.
  - `codex-compat` for legacy compatibility shims, if required.
- Datax public APIs must not expose downstream Codex `Thread`, `Turn`, or `Item` types.

## Test Plan
Required validation sequence:

The concrete Phase 1.7 freeze checklist, including exact commands and expected
evidence, is maintained in
`docs/plans/datax_migration_phase1_7_freeze_report/phase1_migration_freeze_checklist.md`.
The final executable Phase 1 verification plan, including build, test,
formatting, schema generation, smoke, snapshot, and install-from-source
commands, is maintained in
`docs/plans/datax_migration_phase1_test_plan/phase1_test_plan.md`.

1. Static search checks
   - No product-owned `codex`, `Codex`, or `CODEX` references remain except documented exceptions.
   - No Datax-owned `codex-rs` path references remain.
   - No public protocol `Thread`, `Turn`, or `Item` names remain.
   - Phase 1.8 mechanical protocol/domain migration checks from
     `docs/plans/datax_mechanical_protocol_migration/mechanical_protocol_migration_execplan.md`
     pass or have documented compatibility exceptions.
   - No public, wire, generated, CLI-visible, fixture, or documentation references expose `thread_id`, `turn_id`, `ThreadId`, `TurnId`, `codex_thread`, `codex_turn`, or `codex_turns`.
   - Any internal-only Codex-era names that remain are classified with an explicit reason.
   - Protected sandbox identifiers remain unchanged.
   - Downstream Codex references are limited to documented future adapter/runtime boundaries.
   - Candidate downstream Codex artifacts discovered during scans are recorded for Phase 2 follow-up.
   - Any remaining SDK, package, workflow, CI, issue template, or release artifact using Codex naming is either renamed or recorded as a Phase 1 blocker.

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
- The Datax-owned Rust source directory is `datax-rs`, not `codex-rs`.
- No Datax product features were added during the migration.
- No unresolved Datax-owned Codex identity references remain.
- No unclassified internal Codex-era implementation names remain.
- Datax-facing runtime, persistence, and history names follow the Phase 1.8
  mechanical mapping from `Codex`/`Thread`/`Turn`/`Item` to
  `Datax`/`Chat`/`Interaction`/`Message`.
- Any remaining Codex references have an explicit exception classification.
- Candidate downstream Codex artifacts have a Phase 2 follow-up note when discovered during Phase 1 scans.
- Existing Datax folder structure remains intact except for rename-required moves already completed during migration.

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
- Add focused `rg` checks for `codex-rs` path references before and after the top-level path rename.
- Add focused `rg` checks for internal Codex-era names such as `thread_id`, `turn_id`, `ThreadId`, `TurnId`, `codex_thread`, `codex_turn`, and `codex_turns`.
- Maintain an exception list for protected sandbox identifiers, license/provenance text, and unavoidable external names.

Internal name drift:

- Treat public or serialized Codex-era internal names as migration blockers.
- Treat internal-only Codex-era names as inventory items that must be renamed or classified before Phase 1 closes.
- Avoid mechanical rewrites that change local modules, owner types, or TUI-internal data structures without dependency review.
- Prefer small, dependency-aware renames with targeted validation evidence.

Top-level path rename risk:

- Treat `codex-rs` to `datax-rs` as a high-blast-radius mechanical migration.
- Inventory all references first.
- Update path references and directory move in the same milestone.
- Validate build scripts, package scripts, schema generation commands, Bazel metadata, and CI workflow references before closing Phase 1.

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

Phase 1 includes the Phase 1.8 mechanical protocol/domain migration plan:
`docs/plans/datax_mechanical_protocol_migration/mechanical_protocol_migration_execplan.md`.
Phase 2 must not begin downstream adapter/runtime implementation until that
plan has either completed or documented any remaining compatibility exceptions.

Phase 2 should introduce:

- Data engineering domain model: Plan, Workflow, Deployment, Schedule, Execution, Monitor, Artifact, Approval.
- Datax-specific UI and app-server workflows.
- Datax app-server as the product boundary for clients.
- A Datax-owned agent adapter abstraction.
- A downstream Codex app-server runtime integration behind that adapter.
- A dedicated Codex boundary area only for downstream app-server integration or compatibility shims, without restructuring unrelated Datax folders.

The earlier external Codex adapter plan remains strategically useful, but it must be rewritten for the fork-first strategy before implementation. Phase 2 should assume:

- Before: `codex TUI/CLI -> codex app-server`.
- After: `datax TUI/CLI -> datax app-server -> downstream Codex app-server`.
- Datax app-server owns Datax product state, public Datax protocol, and data-engineering workflows.
- Downstream Codex app-server is called only through the adapter/runtime boundary and only when needed.
- Datax public APIs do not expose Codex `Thread`, `Turn`, or `Item`.

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
- Remaining migration cleanup is completed in Phase 1, not Phase 2.
- Datax-owned `datax-rs` path references are completed in Phase 1, not Phase 2.
- Internal Codex-era names are inventoried and classified in Phase 1, not left as unknown Phase 2 debt.
- Phase 1.8 mechanically migrates Datax-facing protocol/domain names according
  to `Codex` → `Datax`, `Thread` → `Chat`, `Turn` → `Interaction`, and
  `Item` → `Message`.
- Phase 2 starts from the current Datax repo structure and does not begin with a broad folder restructure.
- Future downstream Codex app-server integration is isolated behind Datax-owned adapter/runtime boundaries.
