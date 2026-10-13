#!/usr/bin/env bash
# Unit tests for classify-result.sh. Exit 0 only when every case holds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
# shellcheck disable=SC1091
source "$ROOT/classify-result.sh"

fail() {
  printf 'classify-result: %s\n' "$*" >&2
  exit 1
}

assert_status() {
  local want="$1"
  local code="$2"
  local body="$3"
  local log
  log="$(mktemp)"
  printf '%s\n' "$body" >"$log"
  local got
  got="$(classify_check "$code" "$log")"
  rm -f "$log"
  [[ "$got" == "$want" ]] || fail "expected $want for exit=$code log=$(printf %q "$body"), got $got"
}

# False green: the process exited 0 but printed RESULT: FAIL.
assert_status fail 0 "RESULT: FAIL — a device appears more than once"

# Required USB path could not open a service.
assert_status fail 0 "RESULT: BLOCKED — unable to open the crash-report service"

# Missing fixture is optional, even if someone later makes the process exit 1.
assert_status optional 0 "RESULT: INCONCLUSIVE — no report between 4 MiB and 64 MiB"

# RESULT: PASS wins over incidental OPTIONAL (Wi-Fi) notes in the same log.
assert_status pass 0 "$(printf 'OPTIONAL (Wi-Fi): no pair record\nRESULT: PASS — USB pairing ok\n')"

# Wi-Fi-only failure with no RESULT line must not be a required fail.
assert_status optional 0 "OPTIONAL (Wi-Fi): untrusted or session failed"

# Non-zero without a RESULT line is a required fail.
assert_status fail 1 "cargo exploded"
assert_status fail 101 ""

# Clean success.
assert_status pass 0 "RESULT: PASS — listed processes"
assert_status pass 0 "nothing special"

# Executing the script exits 1 on fail and 0 on pass/optional.
log="$(mktemp)"
printf 'RESULT: FAIL — boom\n' >"$log"
if bash "$ROOT/classify-result.sh" 0 "$log" >/dev/null; then
  rm -f "$log"
  fail "expected classify-result.sh to exit non-zero on RESULT: FAIL"
fi
printf 'RESULT: INCONCLUSIVE — skip\n' >"$log"
bash "$ROOT/classify-result.sh" 0 "$log" >/dev/null || fail "optional must exit 0"
printf 'RESULT: PASS\n' >"$log"
bash "$ROOT/classify-result.sh" 0 "$log" >/dev/null || fail "pass must exit 0"
rm -f "$log"

printf 'classify-result: %s cases ok\n' 9
