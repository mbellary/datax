## Summary

- Implements Phase 1.4: app-server model and protocol rename for the Datax migration.
- Renames public app-server v2 protocol surfaces from Thread/Turn/Item to Chat/Interaction/Message terminology.
- Keeps behavior equivalent and defers persistence/fixture/snapshot cleanup outside generated app-server protocol artifacts to later Phase 1 milestones.

## Validation

Validation is staged per the migration execution model. Exact commands are recorded in:

- `docs/plans/datax_migration_phase1_4_app_server_protocol/app_server_protocol_rename_execplan.md`

Current status:

- Completed: `just write-app-server-schema` was run by the user and the generated schema artifacts were committed.
- Completed: `just fmt` from `codex-rs`.
- Completed: lightweight acceptance searches recorded in the ExecPlan.
- Deferred: targeted app-server protocol/app-server test commands remain staged for the post-implementation validation pass.

## Notes

This pull request is migration-only and should not include product behavior changes or data engineering features.
