# Phase 1.6 GitHub Issue

Created: https://github.com/mbellary/datax/issues/11

## Proposed Title

Phase 1.6: Test isolation pass for Datax migration

## Proposed Body

Phase 1.6 is a migration-only stabilization pass. It aligns validation-facing files after the Datax rename so tests, CI helpers, and developer command examples no longer point at stale Codex package names or pre-Phase-1.4 public app-server method names.

Scope:

- Update CI/test helper references to renamed Datax binaries and Cargo packages.
- Update active test fixtures, snapshots, and developer validation command examples that still use old package or binary names.
- Update app-server-over-MCP reference text to current `chat/*`, `interaction/*`, and `message/*` terminology.
- Keep protected sandbox identifiers, external service names, `.codex-plugin`, historical ExecPlan evidence, and internal implementation identifiers as documented exceptions.

Acceptance criteria:

- The Phase 1.6 ExecPlan is current and contains the file inventory, dependency order, rename exceptions, and explicit validation commands.
- Active test harnesses no longer spawn or require the old `codex` binary where the renamed `datax` binary is expected.
- Active command examples use `datax-*` Cargo package names.
- Static migration searches return only documented historical or external exceptions.
- Expensive validation commands are documented for user execution and are not run by Codex unless explicitly requested.

Validation commands for the user are listed in:

`docs/plans/datax_migration_phase1_6_test_isolation/test_isolation_execplan.md`
