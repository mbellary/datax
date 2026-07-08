#!/bin/bash

# Set "chatgpt.cliExecutable": "/Users/<USERNAME>/code/datax/scripts/debug-codex.sh" in VSCode settings to always get the
# latest datax-rs binary when debugging the extension.


set -euo pipefail

CODEX_RS_DIR=$(realpath "$(dirname "$0")/../datax-rs")
(cd "$CODEX_RS_DIR" && cargo run --quiet --bin datax -- "$@")
