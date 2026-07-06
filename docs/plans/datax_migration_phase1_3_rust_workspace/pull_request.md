## Summary

- Implements Phase 1.3: Rust workspace stabilization for the Datax migration.
- Renames internal Rust crate/package identity surfaces from the old product prefix to Datax naming.
- Keeps app-server model rename and persistence/fixture rename deferred to later Phase 1 milestones.

## Validation

Validation is staged per the migration execution model. Exact commands are recorded in:

- `docs/plans/datax_migration_phase1_3_rust_workspace/rust_workspace_stabilization_execplan.md`

Current status:

- Deferred: targeted format/build/test/static checks, unless explicitly run during this milestone.

## Notes

This pull request is migration-only and should not include product behavior changes.
