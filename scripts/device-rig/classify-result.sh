#!/usr/bin/env bash
# Classify one device-rig check from its process exit code and log.
# Prints exactly one of: pass, fail, optional
#
# First matching rule wins:
#   RESULT: FAIL or RESULT: BLOCKED  -> fail  (required check did not hold)
#   RESULT: INCONCLUSIVE             -> optional (missing fixture, not a product bug)
#   RESULT: PASS                     -> pass  (optional Wi-Fi notes do not override)
#   OPTIONAL (Wi-Fi) and exit 0      -> optional
#   non-zero exit                    -> fail
#   else                             -> pass
#
# Sourced by run-harnesses.sh. When executed, prints the status and exits 1 on fail.

classify_check() {
  local code="${1:-0}"
  local log="${2:-}"
  local text=""
  if [[ -n "$log" && -f "$log" ]]; then
    text=$(<"$log")
  fi

  if [[ "$text" == *"RESULT: FAIL"* || "$text" == *"RESULT: BLOCKED"* ]]; then
    printf '%s\n' fail
    return 0
  fi
  if [[ "$text" == *"RESULT: INCONCLUSIVE"* ]]; then
    printf '%s\n' optional
    return 0
  fi
  if [[ "$text" == *"RESULT: PASS"* ]]; then
    printf '%s\n' pass
    return 0
  fi
  if [[ "$text" == *"OPTIONAL (Wi-Fi)"* && "$code" -eq 0 ]]; then
    printf '%s\n' optional
    return 0
  fi
  if [[ "$code" -ne 0 ]]; then
    printf '%s\n' fail
    return 0
  fi
  printf '%s\n' pass
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  status="$(classify_check "${1:-0}" "${2:-}")"
  printf '%s\n' "$status"
  [[ "$status" != fail ]]
fi
