#!/bin/bash

# Set "chatgpt.cliExecutable": "/Users/<USERNAME>/code/datax/scripts/debug-codex.sh" in VSCode settings to always get the
# latest codex-rs binary when debugging the extension.


set -euo pipefail

CODEX_RS_DIR=$(realpath "$(dirname "$0")/../codex-rs")
(cd "$CODEX_RS_DIR" && cargo run --quiet --bin datax -- "$@")
