# Phase 1.6 Pull Request

Created: https://github.com/mbellary/datax/pull/12

## Proposed Title

Phase 1.6: Test isolation pass for Datax migration

## Proposed Body

Closes #11.

## Summary

- Aligns active CI/test helper references with renamed Datax binaries and Cargo packages.
- Updates active validation command examples, test fixtures, and snapshots that still referenced old Codex package names.
- Updates the MCP/app-server reference text to current Datax `chat/*`, `interaction/*`, and `message/*` terminology.
- Records Phase 1.6 scope, file inventory, exceptions, and deferred user-run validation commands in the ExecPlan.

## Validation

Codex ran static checks only, per migration instructions:

- `git diff --check`
- `rg -n "CARGO_BIN_EXE_codex_(linux_sandbox|windows_sandbox_setup|command_runner)" .github codex-rs`
- `rg -n "cargo_bin\\(\\\"codex|should find binary for codex|codex-(linux-sandbox|mcp-server|execve-wrapper|exec-server|exec\\b)|cargo test -p codex|cargo insta pending-snapshots -p codex|just test -p codex|cargo run -p codex|cargo build -p codex" codex-rs .github docs scripts --glob '!target/**' --glob '!Cargo.lock' --glob '!*.snap.new'`
- `rg -n "thread/start|thread/read|thread/list|thread/resume|thread/fork|turn/start|turn/interrupt|item/" codex-rs/docs/codex_mcp_interface.md codex-rs/app-server/README.md codex-rs/app-server-protocol/src codex-rs/app-server/src`

Deferred to user:

- `just fmt`
- `cargo build`
- `just fix -p ...`
- `just test -p ...`
- `just test`
