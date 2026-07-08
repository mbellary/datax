# Phase 1.7 Migration Freeze Checkpoint PR

## Summary

- Adds the Phase 1.7 ExecPlan and concrete migration freeze checklist.
- Cleans active Datax-owned metadata that still used Codex identity.
- Records issue #13 as the tracking issue for the freeze checkpoint.

## Validation

- Completed by Codex:
  - `git diff --check`
  - focused stale package metadata `rg` search
  - focused GitHub metadata `rg` search
  - broad remaining Codex inventory `rg` search for exception classification
- Deferred to the user:
  - `just fmt`
  - `just write-config-schema`
  - `just write-app-server-schema`
  - `just write-app-server-schema --experimental`
  - `just write-hooks-schema`
  - `cargo build`
  - targeted `just test -p ...`
  - `just test`
  - CLI and app-server smoke checks from the freeze checklist

## Notes

Remaining Codex-shaped references are not treated as automatically passing or
failing. The Phase 1.7 freeze report must classify them as protected,
external, provenance, path-deferred, internal implementation-deferred, or
follow-up work before Phase 1 is accepted.

Closes #13.

