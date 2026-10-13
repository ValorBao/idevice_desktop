#!/usr/bin/env bash
# Run the read-only, non-interactive verify_* harnesses for one generation.
# Required environment:
#   IDEVICE_TEST_UDID
#   IDEVICE_TEST_GENERATION   legacy | core-device-remote | core-device-lockdown
# Optional:
#   DEVICE_RIG_LOG_DIR        default: ./device-rig-logs
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# shellcheck disable=SC1091
source "$(cd "$(dirname "$0")" && pwd)/classify-result.sh"
# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

UDID="${IDEVICE_TEST_UDID:?set IDEVICE_TEST_UDID}"
GENERATION="${IDEVICE_TEST_GENERATION:?set IDEVICE_TEST_GENERATION}"
LOG_DIR="${DEVICE_RIG_LOG_DIR:-$ROOT/device-rig-logs/${GENERATION}}"
mkdir -p "$LOG_DIR"

status_file="$LOG_DIR/result.txt"
: >"$status_file"

run_example() {
  local name="$1"
  shift
  local log="$LOG_DIR/${name}.log"
  echo "::group::${name}"
  echo "device-rig: cargo run --example ${name} -- $*"
  set +e
  cargo run --manifest-path src-tauri/Cargo.toml --example "$name" -- "$@" \
    >"$log" 2>&1
  local code=$?
  set -e
  cat "$log"
  echo "::endgroup::"
  local status
  status="$(classify_check "$code" "$log")"
  case "$status" in
    pass)
      echo "PASS ${name}" | tee -a "$status_file"
      ;;
    optional)
      echo "OPTIONAL ${name}" | tee -a "$status_file"
      ;;
    fail)
      echo "FAIL ${name} (exit ${code})" | tee -a "$status_file"
      return 1
      ;;
    *)
      echo "FAIL ${name} (unknown status ${status}, exit ${code})" | tee -a "$status_file"
      return 1
      ;;
  esac
}

failed=0

echo "device-rig: generation=${GENERATION} udid=${UDID}"
echo "generation=${GENERATION}" >"$LOG_DIR/meta.txt"
echo "udid=${UDID}" >>"$LOG_DIR/meta.txt"

if ! run_example verify_ddi "$UDID"; then failed=1; fi
if ! run_example verify_crash_transport "$UDID"; then failed=1; fi
if ! run_example verify_processes "$UDID"; then failed=1; fi
if ! run_example verify_processes_command "$UDID"; then failed=1; fi
if ! run_example verify_large_crash "$UDID"; then failed=1; fi
if ! run_example verify_trollstore "$UDID"; then failed=1; fi

case "$GENERATION" in
  legacy)
    if ! run_example verify_legacy_ddi; then failed=1; fi
    ;;
  core-device-remote|core-device-lockdown)
    if ! run_example verify_performance "$UDID"; then failed=1; fi
    if ! run_example verify_jit "$UDID"; then failed=1; fi
    ;;
  *)
    echo "device-rig: unknown generation ${GENERATION}" >&2
    exit 2
    ;;
esac

echo "::group::ignored cargo hardware tests"
set +e
cargo test --manifest-path src-tauri/Cargo.toml --lib \
  hardware_selected_device_is_unlocked_and_trusted -- --ignored --nocapture \
  >"$LOG_DIR/cargo-ignored.log" 2>&1
ignored_code=$?
set -e
cat "$LOG_DIR/cargo-ignored.log"
echo "::endgroup::"
ignored_status="$(classify_check "$ignored_code" "$LOG_DIR/cargo-ignored.log")"
case "$ignored_status" in
  pass)
    echo "PASS cargo-ignored" | tee -a "$status_file"
    ;;
  optional)
    echo "OPTIONAL cargo-ignored" | tee -a "$status_file"
    ;;
  *)
    echo "FAIL cargo-ignored (exit ${ignored_code})" | tee -a "$status_file"
    failed=1
    ;;
esac

if [[ $failed -ne 0 ]]; then
  echo "device-rig: one or more harnesses failed for ${GENERATION}" >&2
  exit 1
fi
echo "device-rig: ${GENERATION} harnesses passed"
