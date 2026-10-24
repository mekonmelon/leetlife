#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
# After the first build, the app runs on its own: no terminal server needed.
if [ -d src-tauri/target/release/bundle/macos/LeetLife.app ]; then
  open src-tauri/target/release/bundle/macos/LeetLife.app
else
  bash scripts/macos.sh run
fi
