# Phase 1 Datax Migration Test Plan

This test plan verifies the completed Phase 1 migration described in
`docs/plans/Provisional-Datax-Migration-Plan-Phase1.md`. It is intended for a
human runner. Codex must not run the build, format, fix, schema generation, or
test commands in this file unless the user explicitly asks for that exact
command.

Phase 1 acceptance means the repository builds, tests, generates checked-in
artifacts, and runs from source as Datax. Remaining Codex references are allowed
only when they are classified as protected sandbox identifiers, upstream
provenance, external service contracts, downstream Codex adapter/runtime
follow-up, `codex-sdk` follow-up, or internal-name follow-up already recorded
for Phase 2.

## Runner Rules

- Run commands in the order shown.
- Stop at the first failure in each section and paste the full output for
  review.
- If a generator, formatter, or fixer changes files, review the diff and commit
  the accepted changes before continuing.
- Record the tested commit hash.
- Do not delete historical planning files or protected sandbox identifiers just
  to reduce search output.
- A zero-output command passes only when it exits successfully and prints no
  matches.

## Evidence Record

Fill this table while running the plan.

| Area | Command Group | Result | Evidence Notes |
| --- | --- | --- | --- |
| Baseline | Git status and commit | `Pending` | |
| Tooling | Tool versions | `Pending` | |
| Static gates | Identity and protocol searches | `Pending` | |
| Schema generation | Config, app-server, hooks | `Pending` | |
| Formatting | `just fmt`, `just fmt-check`, `git diff --check` | `Pending` | |
| Fix/lint | Focused `just fix -p ...` commands | `Pending` | |
| Build | Workspace and CLI builds | `Pending` | |
| Tests | Targeted and full test suite | `Pending` | |
| Snapshots | Pending snapshot checks | `Pending` | |
| Install from source | `docs/install.md` flow | `Pending` | |
| Smoke | CLI, launcher, app-server checks | `Pending` | |
| Final cleanliness | Git status and diff checks | `Pending` | |

## 1. Baseline

Run from the repository root:

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch
    git rev-parse --abbrev-ref HEAD
    git rev-parse HEAD
    git log --oneline -10

Expected result:

- Final Phase 1 verification should be run from `main` after Phase 1 branches
  are merged.
- `git status --short --branch` should show no uncommitted files before
  validation starts.
- Record the exact commit hash from `git rev-parse HEAD`.

## 2. Tooling

Run from the repository root:

    cd /home/mbellary/wsl/projects/datax
    command -v git
    command -v rg
    command -v just
    command -v cargo
    command -v cargo-nextest
    command -v cargo-insta
    command -v rustc
    command -v rustup
    command -v python3
    command -v node
    command -v npm
    command -v dotslash
    git --version
    rg --version
    just --version
    cargo --version
    cargo nextest --version
    cargo insta --version
    rustc --version
    rustup --version
    python3 --version
    node --version
    npm --version
    dotslash --version

Expected result:

- Every tool resolves and prints a version.
- If `cargo-insta` is missing, install it before snapshot checks:

    cargo install --locked cargo-insta

- If `cargo-nextest`, `just`, or `dotslash` is missing, install the missing
  tool before continuing:

    cargo install --locked cargo-nextest
    cargo install --locked just
    cargo install --locked dotslash

## 3. Static Identity Gates

Run from the repository root:

    cd /home/mbellary/wsl/projects/datax
    git diff --check

Expected result:

- No output.

Run the forbidden spelling check:

    cd /home/mbellary/wsl/projects/datax
    rg -n --hidden "DataX" . \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new'

Expected result:

- No output.

Run the remaining Codex inventory:

    cd /home/mbellary/wsl/projects/datax
    rg -n --hidden "\b(Codex|codex|CODEX)\b" . \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      > /tmp/datax_phase1_remaining_codex_refs.txt
    wc -l /tmp/datax_phase1_remaining_codex_refs.txt
    sed -n '1,260p' /tmp/datax_phase1_remaining_codex_refs.txt

Expected result:

- Output is allowed.
- Every match must fit an accepted category:
  protected sandbox identifier, upstream provenance, external service contract,
  historical migration plan, downstream Codex adapter/runtime follow-up,
  `codex-sdk` follow-up, or classified internal-name follow-up.
- New active Datax-owned product, CLI, docs, build, package, or app-server
  identity matches are Phase 1 verification failures.

Prove the protected sandbox identifiers remain unchanged:

    cd /home/mbellary/wsl/projects/datax
    rg -n "CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR|CODEX_SANDBOX_ENV_VAR|CODEX_SANDBOX_NETWORK_DISABLED|CODEX_SANDBOX" datax-rs

Expected result:

- Matches exist.
- These identifiers remain `CODEX_*`; they must not be renamed.

Prove old source path references are gone from active project files:

    cd /home/mbellary/wsl/projects/datax
    rg -n --hidden "codex-rs|//codex-rs|\\./codex-rs|/codex-rs" . \
      --glob '!.git/**' \
      --glob '!.codex/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new'

Expected result:

- No output.

Prove active app-server method names do not use old public resource names:

    cd /home/mbellary/wsl/projects/datax
    rg -n "\"(thread/(start|read|list|resume|fork|rollback|search|unarchive)|turn/(start|interrupt)|item/)" \
      datax-rs/app-server \
      datax-rs/app-server-client \
      datax-rs/app-server-protocol \
      datax-rs/app-server-test-client \
      datax-rs/docs \
      --glob '!target/**' \
      --glob '!**/*.snap.new'

Expected result:

- No output, unless a match is explicitly documented as legacy compatibility.

Run the carried-forward follow-up inventories:

    cd /home/mbellary/wsl/projects/datax
    rg -n "codex-sdk|@openai/codex-sdk|openai_codex|@openai/codex" \
      sdk \
      datax-cli/scripts \
      .github/workflows/rust-release.yml \
      --glob '!target/**' \
      --glob '!**/*.snap.new'

    rg -l --hidden "\b(thread_id|turn_id|ThreadId|TurnId|codex_thread|codex_turn|codex_turns)\b" . \
      --glob '!.git/**' \
      --glob '!target/**' \
      --glob '!datax-rs/target/**' \
      --glob '!**/*.snap.new' \
      | awk -F/ '{ if ($1 == ".") print $2; else print $1 }' \
      | sort \
      | uniq -c \
      | sort -nr

Expected result:

- Output is allowed.
- The output is a Phase 2 follow-up inventory, not a Phase 1 failure, when it
  matches the already recorded `codex-sdk` and internal-name classifications.

## 4. Schema And Generated Artifact Drift

Run from the repository root. The root `justfile` defaults to `datax-rs` for
workspace commands.

    cd /home/mbellary/wsl/projects/datax
    just write-config-schema
    git status --short

Expected result:

- Command succeeds.
- `git status --short` is empty, or generated drift is reviewed and committed.

Run stable app-server schema generation:

    cd /home/mbellary/wsl/projects/datax
    just write-app-server-schema
    git status --short

Expected result:

- Command succeeds.
- `git status --short` is empty, or generated drift is reviewed and committed.

Run experimental app-server schema generation:

    cd /home/mbellary/wsl/projects/datax
    just write-app-server-schema --experimental
    git status --short

Expected result:

- Command succeeds.
- `git status --short` is empty, or generated drift is reviewed and committed.

Run hook schema generation:

    cd /home/mbellary/wsl/projects/datax
    just write-hooks-schema
    git status --short

Expected result:

- Command succeeds.
- `git status --short` is empty, or generated drift is reviewed and committed.

## 5. Formatting

Run from the repository root:

    cd /home/mbellary/wsl/projects/datax
    just fmt
    git status --short
    git diff --check

Expected result:

- `just fmt` succeeds.
- `git diff --check` prints no output.
- `git status --short` is empty, or formatting drift is reviewed and committed.

Run the formatter check after committing any formatter changes:

    cd /home/mbellary/wsl/projects/datax
    just fmt-check

Expected result:

- Command succeeds.

## 6. Focused Fix/Lint Passes

Run from the repository root:

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

Expected result:

- Every `just fix -p ...` command succeeds.
- `git diff --check` prints no output.
- `git status --short` is empty, or fix changes are reviewed and committed.

## 7. Build Validation

Run from the Cargo workspace directory:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    cargo build

Expected result:

- Workspace build succeeds.

Build the CLI binary explicitly:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    cargo build -p datax-cli --bin datax
    test -x target/debug/datax

Expected result:

- Build succeeds.
- `datax-rs/target/debug/datax` exists and is executable.

Build release artifacts if you want to verify the release path:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    cargo build --release -p datax-cli --bin datax
    test -x target/release/datax

Expected result:

- Release build succeeds.
- `datax-rs/target/release/datax` exists and is executable.

## 8. Targeted Rust Tests

Run from the repository root and stop on the first failing command:

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

Expected result:

- Every command succeeds.
- If a command fails, stop and paste the full output before continuing.

Run the full suite after targeted tests pass:

    cd /home/mbellary/wsl/projects/datax
    just test

Expected result:

- Full suite succeeds.
- Any failure must be documented with exact command, output, owner, and whether
  it is a migration-specific deferral.

## 9. Snapshot And Fixture Review

Run from the Cargo workspace directory:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    cargo insta pending-snapshots

Expected result:

- No pending snapshots.

If pending snapshots are reported, inspect before accepting:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    find . -name '*.snap.new' -print
    cargo insta show path/to/the/file.snap.new

Expected result:

- Each pending snapshot is reviewed as intentional or rejected.
- Do not accept all snapshots blindly.

If the only pending snapshots are intentional Phase 1 rename outputs, accept
them after review:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    cargo insta accept
    cd /home/mbellary/wsl/projects/datax
    git status --short

Expected result:

- Accepted snapshot changes are reviewed and committed.

Prove no ignored pending snapshot files remain:

    cd /home/mbellary/wsl/projects/datax/datax-rs
    find . -name '*.snap.new' -print

Expected result:

- No output.

## 10. Install From Source Verification

This section verifies the flow described in `docs/install.md`. Run it in the
current checkout first. If you want a fresh-clone proof, use the fresh-clone
variant afterward.

Current checkout flow:

    cd /home/mbellary/wsl/projects/datax
    rustup component add rustfmt
    rustup component add clippy
    cargo install --locked just
    cargo install --locked dotslash
    cargo install --locked cargo-nextest
    cargo install --locked cargo-insta
    cd /home/mbellary/wsl/projects/datax/datax-rs
    cargo build
    cargo run --bin datax -- --help
    cargo run --bin datax -- --version

Expected result:

- Tool installation commands either install the tool or report it is already
  installed.
- `cargo build` succeeds.
- `cargo run --bin datax -- --help` prints Datax help.
- `cargo run --bin datax -- --version` prints Datax version output.

Fresh-clone flow:

    cd /tmp
    rm -rf /tmp/datax-install-smoke
    git clone https://github.com/mbellary/datax.git /tmp/datax-install-smoke
    cd /tmp/datax-install-smoke/datax-rs
    cargo build
    cargo run --bin datax -- --help
    cargo run --bin datax -- --version

Expected result:

- A fresh clone builds without the original Codex checkout.
- Help and version output identify Datax/datax.

Optional local binary install smoke:

    cd /home/mbellary/wsl/projects/datax
    cargo install --path datax-rs/cli --bin datax --locked --force
    command -v datax
    datax --help
    datax --version

Expected result:

- Cargo installs the local `datax` binary to Cargo's bin directory.
- `datax --help` and `datax --version` run without referring to a `codex`
  executable.

## 11. CLI, Launcher, And App-Server Smoke Checks

Run CLI identity checks from the repository root:

    cd /home/mbellary/wsl/projects/datax
    ./datax-rs/target/debug/datax --help
    ./datax-rs/target/debug/datax --version
    ./datax-rs/target/debug/datax app-server --help

Expected result:

- Commands exit successfully.
- Output identifies Datax/datax, except documented external, model, or
  historical references.

Run the npm launcher staging smoke:

    cd /home/mbellary/wsl/projects/datax
    bash -lc 'set -euo pipefail; launcher_stage="$(mktemp -d /tmp/datax-launcher-smoke.XXXXXX)"; python3 datax-cli/scripts/build_npm_package.py --version 0.0.0-dev --staging-dir "$launcher_stage"; mkdir -p "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin"; printf "%s\n" "#!/usr/bin/env sh" "printf \"datax 0.0.0-dev\\n\"" > "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin/datax"; chmod +x "$launcher_stage/vendor/x86_64-unknown-linux-musl/bin/datax"; node "$launcher_stage/bin/datax.js" --version'

Expected result:

- Command exits successfully.
- Output includes `datax 0.0.0-dev`.

Run app-server test-client initialization:

    cd /home/mbellary/wsl/projects/datax
    just app-server-test-client model-list

Expected result:

- Command exits successfully.
- The test client starts `./target/debug/datax app-server`.
- The response is a successful `model/list` response.

Run chat-list smoke:

    cd /home/mbellary/wsl/projects/datax
    just app-server-test-client chat-list --limit 5

Expected result:

- Command exits successfully.
- Output uses chat terminology.

Run live message smoke only if credentials and network access are configured:

    cd /home/mbellary/wsl/projects/datax
    just app-server-test-client send-message-v2 "Reply with exactly: Datax smoke ok"

Expected result:

- Command exits successfully.
- The app-server accepts `chat/start` and `interaction/start`.
- Streamed output uses chat, interaction, and message terminology.
- If credentials or network access are unavailable, record the command as
  skipped with that exact reason.

## 12. Final Cleanliness And Acceptance

Run from the repository root:

    cd /home/mbellary/wsl/projects/datax
    git status --short --branch
    git diff --check
    git diff --stat

Expected result:

- `git status --short --branch` shows no uncommitted files.
- `git diff --check` prints no output.
- `git diff --stat` prints no file changes.

Phase 1 is accepted only when:

- Fresh or current checkout builds from source as Datax.
- Required generated artifacts are current.
- Formatting and focused fix/lint passes are clean.
- Targeted tests and full test suite pass, or failures are explicitly
  documented with owner and reason.
- CLI and app-server smoke checks use Datax naming.
- The npm launcher smoke resolves `datax`, not `codex`.
- No Datax-owned `codex-rs` path references remain.
- Public app-server protocol surfaces use chat, interaction, and message
  terminology.
- Remaining Codex references are classified as protected, provenance, external,
  downstream adapter/runtime follow-up, `codex-sdk` follow-up, or internal-name
  follow-up.
