## Scope

Implement Phase 1.4 of the Datax migration: app-server model and protocol rename.

This milestone is migration-only. It renames the public app-server v2 protocol from Thread, Turn, and Item terminology to Chat, Interaction, and Message terminology while preserving behavior.

## Out of Scope

- Data engineering product features.
- Broad internal runtime concept cleanup outside the app-server public API boundary.
- Persistence directory, fixture, and snapshot rename except generated app-server protocol schema artifacts.
- Backwards-compatible aliases for old `thread/*`, `turn/*`, or `item/*` methods.
- Protected sandbox identifiers: `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`.

## Acceptance Criteria

- Phase 1.4 ExecPlan is current and self-contained.
- App-server v2 method strings use `chat/*`, `interaction/*`, and `message/*` naming where applicable.
- Public app-server v2 request, response, notification, and schema type names use Chat, Interaction, and Message terminology.
- Generated JSON schemas and TypeScript bindings are regenerated with the renamed public protocol names.
- App-server README/API examples and v2 integration tests use the renamed public methods and types.
- Remaining thread, turn, or item terminology is documented as an internal or deferred exception in the ExecPlan.
- Exact validation commands are documented in the ExecPlan and marked deferred unless explicitly run during the milestone.
