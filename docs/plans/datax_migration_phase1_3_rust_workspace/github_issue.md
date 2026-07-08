## Scope

Implement Phase 1.3 of the Datax migration: Rust workspace stabilization.

This milestone is migration-only. It renames Rust workspace package names, dependency keys, crate identifiers, Bazel crate labels, lockfiles, and directly related generated metadata from the old product prefix to Datax naming.

## Out of Scope

- App-server Thread/Turn/Item to Chat/Interaction/Message protocol rename.
- Persistence directory, fixture, and snapshot rename unless directly required by Rust crate metadata.
- Product behavior changes or data engineering features.
- Protected sandbox identifiers: `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` and `CODEX_SANDBOX_ENV_VAR`.

## Acceptance Criteria

- Phase 1.3 ExecPlan is current and self-contained.
- Rust package names and dependency keys use `datax-*` where they represent internal Datax crates.
- Rust crate identifiers use `datax_*` where they represent renamed internal crates.
- Bazel crate target names and `crate_name` values align with the renamed Rust crate identifiers.
- `datax-rs/Cargo.lock` and any required Bazel lock metadata are refreshed or explicitly documented.
- Remaining Codex/codex references are documented in the ExecPlan exception register.
- Exact validation commands are documented in the ExecPlan and marked deferred unless explicitly run during the milestone.
