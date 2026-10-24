#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Finder and some terminals don't load nvm or Cargo's shell configuration.
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
if ! node -e 'const [major,minor] = process.versions.node.split(".").map(Number); process.exit(major > 22 || (major === 22 && minor >= 12) ? 0 : 1)' >/dev/null 2>&1; then
  for leetlife_node in "$HOME"/.nvm/versions/node/v24*/bin "$HOME"/.nvm/versions/node/v22*/bin; do
    if [ -x "$leetlife_node/node" ]; then export PATH="$leetlife_node:$PATH"; break; fi
  done
fi
if ! node -e 'const [major,minor] = process.versions.node.split(".").map(Number); process.exit(major > 22 || (major === 22 && minor >= 12) ? 0 : 1)' >/dev/null 2>&1; then
  echo 'LeetLife needs Node 22.12 or newer to build. Install Node 24 LTS, then try again.' >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo 'LeetLife needs Rust to build. Install it from https://rustup.rs, then try again.' >&2
  exit 1
fi
if [ ! -d node_modules ]; then npm ci; fi

case "${1:-run}" in
  dev) exec npm run tauri -- dev ;;
  build) exec npm run tauri -- build --bundles app ;;
  run)
    leetlife_app="$PWD/src-tauri/target/release/bundle/macos/LeetLife.app"
    if [ ! -d "$leetlife_app" ]; then npm run tauri -- build --bundles app; fi
    exec open "$leetlife_app"
    ;;
  *) echo 'Usage: scripts/macos.sh [run|dev|build]' >&2; exit 1 ;;
esac
