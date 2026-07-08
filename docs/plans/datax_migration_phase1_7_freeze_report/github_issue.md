# Phase 1.7: Migration freeze checkpoint

## Summary

Create the Phase 1 migration freeze checkpoint for Datax. This milestone records the final validation checklist, cleans active repository metadata that still points contributors or automation at Codex-owned names, and prepares the freeze report evidence ledger.

## Scope

- Maintain a PLANS.md-compliant ExecPlan for Phase 1.7.
- Maintain a concrete freeze checklist with exact commands for the user to run.
- Clean active Datax-owned metadata discovered by the freeze inventory:
  - root package script names,
  - GitHub issue templates,
  - pull request template,
  - CODEOWNERS,
  - active CI labels and package staging arguments,
  - contributor-facing docs.
- Preserve documented exceptions such as protected sandbox identifiers, `codex-rs` paths, external artifact sources, and historical migration plans.

## Out of Scope

- No Rust runtime behavior changes.
- No app-server protocol changes.
- No generated schema changes unless user-run freeze commands produce drift.
- No filesystem rename of `codex-rs`.
- No build, format, fix, generation, or test execution by Codex unless explicitly requested.

## Acceptance Criteria

- Phase 1.7 ExecPlan is current.
- Freeze checklist lists exact commands and expected evidence.
- Active Datax-owned metadata no longer uses stale Codex identity.
- Focused static searches are rerun and results are recorded.
- Remaining Codex-shaped references are documented as exceptions or follow-up risks.
- User-run freeze validation output can be recorded into the final freeze report.

