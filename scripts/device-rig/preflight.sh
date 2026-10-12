#!/usr/bin/env bash
# Build and run the device_preflight example against whatever usbmuxd sees.
# Extra arguments are forwarded (e.g. --json, --require-generation legacy).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

if ! command -v cargo >/dev/null 2>&1; then
  echo "device-rig: cargo is not on PATH" >&2
  exit 2
fi

exec cargo run --manifest-path src-tauri/Cargo.toml --example device_preflight -- "$@"
