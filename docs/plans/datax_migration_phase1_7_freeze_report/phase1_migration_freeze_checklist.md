# Phase 1 Migration Freeze Checklist

This checklist expands the Phase 1 Test Plan from
`docs/plans/Provisional-Datax-Migration-Plan-Phase1.md` into concrete commands
for the Phase 1.7 migration freeze checkpoint.

Codex must not run the build, format, fix, generation, or test commands in this
checklist unless the user explicitly asks for that exact command. The user runs
the commands, pastes the output, and the Phase 1.7 freeze report records the
evidence.

## Evidence Rules

- Run commands from the working directory shown for each section.
- Record the exact commit tested with `git rev-parse HEAD`.
- If a command fails, stop the sequence and paste the full failure output.
- If a command changes checked-in files, do not continue to the next validation
  category until the changed files are reviewed, fixed or accepted, committed,
  and the command is rerun.
- A zero-match search passes only when it prints no matches and exits
  successfully.
- An inventory search is not expected to be empty. Its output must be reviewed
  and each remaining match must be categorized in the final Rename Exception
  Register.

## Evidence Table Template

| Area | Command | Result | Evidence Notes |
| --- | --- | --- | --- |
| Baseline | `git rev-parse HEAD` | `Pending` | Commit tested. |
| Static search | `git diff --check` | `Pending` | Expect no output. |
| Generation | `just write-app-server-schema` | `Pending` | Expect no uncommitted drift after review. |
| Build | `cargo build` | `Pending` | Expect success. |
| Tests | `just test -p datax-tui` | `Pending` | Expect success. |
| Smoke | `./target/debug/datax --help` | `Pending` | Expect Datax help text. |

## 1. Baseline And Tooling

From the repository root, record the branch, commit, and local cleanliness:

```bash
cd /home/mbellary/wsl/projects/datax
git status --short --branch
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git log --oneline -10
```

Expected result:

- Branch is `main` for the final freeze baseline, or the active Phase 1.7
  branch before the freeze PR is merged.
- `git status --short --branch` has no uncommitted files before validation
  begins.
- The tested commit hash is recorded in the freeze report.

From the repository root, record tool versions used for the freeze run:

```bash
cd /home/mbellary/wsl/projects/datax
command -v git
command -v rg
command -v just
command -v cargo
command -v rustc
command -v python3
command -v node
git --version
rg --version
just --version
cargo --version
rustc --version
python3 --version
node --version
```

Expected result:

- Every command resolves to an executable path.
- Version output is captured in the freeze report.

## 2. Static Search Gates

From the repository root, run the whitespace check:

```bash
cd /home/mbellary/wsl/projects/datax
git diff --check
```

Expected result:

- No output.

From the repository root, prove the forbidden mixed-case spelling is absent:

```bash
cd /home/mbellary/wsl/projects/datax
rg -n --hidden "DataX" . \
  --glob '!.git/**' \
  --glob '!target/**' \
  --glob '!codex-rs/target/**'
```

Expected result:

- No output.

From the repository root, create the remaining Codex-name inventory:

```bash
cd /home/mbellary/wsl/projects/datax
rg -n --hidden "\b(Codex|codex|CODEX)\b" . \
  --glob '!.git/**' \
  --glob '!target/**' \
  --glob '!codex-rs/target/**' \
  --glob '!**/*.snap.new' \
  > /tmp/datax_phase1_remaining_codex_refs.txt
wc -l /tmp/datax_phase1_remaining_codex_refs.txt
sed -n '1,240p' /tmp/datax_phase1_remaining_codex_refs.txt
```

Expected result:

- The command may produce matches.
- Every match must be categorized as one of:
  - protected sandbox identifier,
  - external service contract,
  - upstream provenance or license text,
  - deferred filesystem path such as `codex-rs`,
  - internal implementation name explicitly deferred,
  - stale migration reference that must be fixed before freeze.

From the repository root, prove protected sandbox identifiers still exist:

```bash
cd /home/mbellary/wsl/projects/datax
rg -n "CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR|CODEX_SANDBOX_ENV_VAR|CODEX_SANDBOX_NETWORK_DISABLED|CODEX_SANDBOX" codex-rs
```

Expected result:

- Matches exist.
- The matched protected identifiers remain `CODEX_*`; they must not be renamed.

From the repository root, prove active public app-server method names do not use
the old resource names:

```bash
cd /home/mbellary/wsl/projects/datax
rg -n "\"(thread/(start|read|list|resume|fork)|turn/(start|interrupt)|item/)" \
  codex-rs/app-server \
  codex-rs/app-server-client \
  codex-rs/app-server-protocol \
  codex-rs/app-server-test-client \
  codex-rs/docs/codex_mcp_interface.md \
  --glob '!target/**' \
  --glob '!**/*.snap.new'
```

Expected result:

- No output.

From the repository root, inventory remaining public concept wording in
app-server-facing documents and generated v2 artifacts:

```bash
cd /home/mbellary/wsl/projects/datax
rg -n "\b(Thread|thread|Turn|turn|Item|item)\b" \
  codex-rs/app-server/README.md \
  codex-rs/docs/codex_mcp_interface.md \
  codex-rs/app-server-protocol/schema/json/v2 \
  codex-rs/app-server-protocol/schema/typescript/v2 \
  --glob '!target/**'
```

Expected result:

- The command may produce matches.
- Each match must be reviewed as public API wording. Stale Thread/Turn/Item
  product terminology must be fixed before freeze unless it is explicitly
  documented as legacy compatibility or internal model context.

From the repository root, prove old Cargo package command examples are absent
from active non-historical files:

```bash
cd /home/mbellary/wsl/projects/datax
rg -n "just (test|fix) -p codex|cargo (run|build|test|nextest).*codex-|--bin codex\b|cargo_bin\(\"codex" \
  .github \
  README.md \
  docs \
  justfile \
  scripts \
  datax-cli \
  codex-rs \
  --glob '!target/**' \
  --glob '!codex-rs/target/**' \
  --glob '!docs/plans/datax_migration_phase1_*_*/**' \
  --glob '!**/*.snap.new'
```

Expected result:

- No output, except if the command identifies a deliberate compatibility
  reference that is then added to the final exception register.

## 3. Generated Artifact Drift

From the repository root, regenerate config schema artifacts:

```bash
cd /home/mbellary/wsl/projects/datax
just write-config-schema
git status --short
```

Expected result:

- The generation command succeeds.
- `git status --short` is empty, or any generated diff is reviewed and committed
  before freeze.

From the repository root, regenerate stable app-server schema and TypeScript
artifacts:

```bash
cd /home/mbellary/wsl/projects/datax
just write-app-server-schema
git status --short
```

Expected result:

- The generation command succeeds.
- `git status --short` is empty, or any generated diff is reviewed and committed
  before freeze.

From the repository root, regenerate experimental app-server schema artifacts:

```bash
cd /home/mbellary/wsl/projects/datax
just write-app-server-schema --experimental
git status --short
```

Expected result:

- The generation command succeeds.
- `git status --short` is empty, or any generated diff is reviewed and committed
  before freeze.

From the repository root, regenerate hook schema artifacts:

```bash
cd /home/mbellary/wsl/projects/datax
just write-hooks-schema
git status --short
```

Expected result:

- The generation command succeeds.
- `git status --short` is empty, or any generated diff is reviewed and committed
  before freeze.

## 4. Formatting And Lint/Fix

From the repository root, run repository formatting:

```bash
cd /home/mbellary/wsl/projects/datax
just fmt
git status --short
git diff --check
```

Expected result:

- `just fmt` succeeds.
- `git diff --check` prints no output.
- `git status --short` is empty, or formatting changes are reviewed and
  committed before freeze.

From the repository root, run focused fix/lint passes for the Phase 1 migration
surface:

```bash
cd /home/mbellary/wsl/projects/datax
just fix -p datax-cli
just fix -p datax-config
just fix -p datax-core
just fix -p datax-state
just fix -p datax-linux-sandbox
just fix -p datax-app-server-protocol
just fix -p datax-app-server
just fix -p datax-app-server-test-client
just fix -p datax-tui
git status --short
git diff --check
```

Expected result:

- Every `just fix -p ...` command succeeds.
- `git diff --check` prints no output.
- `git status --short` is empty, or fix changes are reviewed and committed
  before freeze.

## 5. Build Validation

From the repository root, build the Rust workspace:

```bash
cd /home/mbellary/wsl/projects/datax
cargo build
```

Expected result:

- Build succeeds.

From the repository root, build the CLI binary used by smoke tests:

```bash
cd /home/mbellary/wsl/projects/datax
cargo build -p datax-cli --bin datax
```

Expected result:

- Build succeeds.
- `codex-rs/target/debug/datax` exists.

From the repository root, run the npm launcher staging smoke test:

```bash
cd /home/mbellary/wsl/projects/datax
bash -lc 'set -euo pipefail; launcher_stage="$(mktemp -d /tmp/datax-launcher-smoke.XXXXXX)"; python3 datax-cli/scripts/build_npm_package.py --version 0.0.0-dev --staging-dir "$launcher_stage"; mkdir -p "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin"; printf "%s\n" "#!/usr/bin/env sh" "printf \"datax 0.0.0-dev\\n\"" > "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin/datax"; chmod +x "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin/datax"; node "$launcher_stage/bin/datax.js" --version'
```

Expected result:

- Command exits successfully.
- Output includes `datax 0.0.0-dev`.

## 6. Targeted Rust Tests

From the repository root, run targeted tests for the migration-critical crates:

```bash
cd /home/mbellary/wsl/projects/datax
just test -p datax-utils-home-dir
just test -p datax-config
just test -p datax-state
just test -p datax-linux-sandbox
just test -p datax-app-server-protocol
just test -p datax-app-server
just test -p datax-app-server-test-client
just test -p datax-cli
just test -p datax-core
just test -p datax-tui
```

Expected result:

- Every command succeeds.
- If one command fails, stop and provide the failure output before running the
  remaining commands.

From the repository root, run the complete Rust test suite:

```bash
cd /home/mbellary/wsl/projects/datax
just test
```

Expected result:

- The full suite succeeds, or any failure is documented as a migration-specific
  skip/deferral with an explicit owner and follow-up.

## 7. CLI And App-Server Smoke Checks

From the repository root, run CLI identity smoke checks:

```bash
cd /home/mbellary/wsl/projects/datax
./codex-rs/target/debug/datax --help
./codex-rs/target/debug/datax --version
./codex-rs/target/debug/datax app-server --help
```

Expected result:

- Commands exit successfully.
- Help/version output identifies the executable as Datax/datax, not Codex/codex,
  except for documented external or historical references.

From the repository root, initialize the app-server through the checked-in test
client:

```bash
cd /home/mbellary/wsl/projects/datax
just app-server-test-client model-list
```

Expected result:

- Command exits successfully.
- The test client starts `./target/debug/datax app-server` and receives a
  successful `model/list` response.

From the repository root, list chats through the checked-in test client:

```bash
cd /home/mbellary/wsl/projects/datax
just app-server-test-client chat-list --limit 5
```

Expected result:

- Command exits successfully.
- Output uses chat terminology.

From the repository root, run a live chat/interaction smoke only when the
environment has the required model credentials and network access:

```bash
cd /home/mbellary/wsl/projects/datax
just app-server-test-client send-message-v2 "Reply with exactly: Datax smoke ok"
```

Expected result:

- Command exits successfully.
- The app-server accepts `chat/start` and `interaction/start`.
- Streamed notifications use `chat/*`, `interaction/*`, and `message/*`
  terminology.
- If credentials or network access are unavailable, record the command as
  skipped with that explicit reason and rely on `just test -p datax-app-server`
  for offline JSON-RPC protocol evidence.

## 8. Snapshot And Fixture Review

From the repository root, check for pending TUI snapshots:

```bash
cd /home/mbellary/wsl/projects/datax
cargo insta pending-snapshots -p datax-tui
```

Expected result:

- No pending snapshots.

From the repository root, prove old TUI snapshot fixture names are absent:

```bash
cd /home/mbellary/wsl/projects/datax
find codex-rs/tui -path '*/snapshots/codex_tui__*.snap' -print
```

Expected result:

- No output.

From the repository root, inventory ignored generated snapshot files:

```bash
cd /home/mbellary/wsl/projects/datax
find codex-rs -name '*.snap.new' -print
```

Expected result:

- No output before declaring freeze.

## 9. Final Git Cleanliness

From the repository root, confirm there is no uncommitted drift after all
checks:

```bash
cd /home/mbellary/wsl/projects/datax
git status --short --branch
git diff --check
git diff --stat
```

Expected result:

- Branch and commit match the freeze report.
- `git status --short --branch` shows no uncommitted files.
- `git diff --check` prints no output.
- `git diff --stat` prints no file changes.

## 10. Freeze Decision Inputs

The Phase 1.7 freeze report must include:

- Tested branch and commit.
- Command results from every checklist section.
- Final Rename Exception Register.
- Final public surface status for CLI, app-server protocol, config/state paths,
  generated artifacts, snapshots, and package metadata.
- Any skipped or deferred command with exact reason.
- A decision of either `Accepted` or `Not Accepted`.
- Follow-up work that is allowed only after Phase 1, including Phase 2 product
  evolution tasks.

