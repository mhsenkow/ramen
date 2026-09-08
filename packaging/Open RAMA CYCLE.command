#!/bin/bash
# Clear macOS quarantine on the sibling .app and launch it.
cd "$(dirname "$0")"
APP=$(find . -maxdepth 1 -name '*.app' -print -quit)
if [[ -z "$APP" ]]; then
  osascript -e 'display alert "RAMA CYCLE" message "No .app found next to this script. Unzip the full macOS build first."'
  exit 1
fi
xattr -cr "$APP" 2>/dev/null || true
open "$APP"
