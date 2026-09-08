#!/usr/bin/env bash
# Linux / Steam Deck launcher — sets +x and starts the binary beside this script.
set -euo pipefail
cd "$(dirname "$0")"
BIN=$(find . -maxdepth 1 -type f \( -name 'RamaCycle*' -o -name 'RAMA*' \) ! -name '*.sh' ! -name '*.txt' | head -n 1)
if [[ -z "$BIN" ]]; then
  echo "No RamaCycle binary found in $(pwd)" >&2
  exit 1
fi
chmod +x "$BIN"
exec "$BIN" "$@"
