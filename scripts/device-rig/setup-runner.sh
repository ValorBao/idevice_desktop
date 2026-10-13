#!/usr/bin/env bash
# Idempotent install of a GitHub Actions self-hosted runner on the owner's
# Apple Silicon Mac mini. The registration token is read from the environment
# and is never written into this script.
#
# Required:
#   GITHUB_REPOSITORY   owner/repo   OR   GITHUB_REPO_URL
#   GITHUB_RUNNER_TOKEN registration token from Settings → Actions → Runners
#                       OR GH_TOKEN / GITHUB_TOKEN with repo admin, used only
#                       to mint a registration token
#
# Optional:
#   RUNNER_DIR          default: $HOME/actions-runner
#   RUNNER_NAME         default: hostname
#   RUNNER_LABELS       extra labels; default: ios-devices
#                       (GitHub also applies self-hosted, macOS, ARM64)
#   REPLACE_RUNNER      1 to reconfigure an existing runner of the same name
set -euo pipefail

log() { printf 'device-rig: %s\n' "$*"; }
die() { printf 'device-rig: %s\n' "$*" >&2; exit 1; }

[[ "$(uname -s)" == Darwin ]] || die "this installer is for macOS only"
[[ "$(uname -m)" == arm64 ]] || die "the project builds Apple Silicon only"

RUNNER_DIR="${RUNNER_DIR:-$HOME/actions-runner}"
RUNNER_NAME="${RUNNER_NAME:-$(scutil --get LocalHostName 2>/dev/null || hostname -s)}"
RUNNER_LABELS="${RUNNER_LABELS:-ios-devices}"
REPO_URL="${GITHUB_REPO_URL:-}"
if [[ -z "$REPO_URL" ]]; then
  [[ -n "${GITHUB_REPOSITORY:-}" ]] || die "set GITHUB_REPOSITORY (owner/repo) or GITHUB_REPO_URL"
  REPO_URL="https://github.com/${GITHUB_REPOSITORY}"
fi

if ! xcode-select -p >/dev/null 2>&1; then
  die "Xcode Command Line Tools are missing. On the Mac mini run: xcode-select --install"
fi

if [[ ! -d /Applications/Xcode.app ]] && ! command -v xcodebuild >/dev/null 2>&1; then
  log "warning: Xcode.app not found; iOS 17+ DDI mounting via devicectl will not work until Xcode 15+ is installed"
fi

if ! command -v brew >/dev/null 2>&1; then
  log "installing Homebrew"
  NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  if [[ -x /opt/homebrew/bin/brew ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  fi
fi
command -v brew >/dev/null 2>&1 || die "Homebrew is not on PATH"

ensure_brew() {
  local formula="$1"
  if brew list --formula "$formula" >/dev/null 2>&1; then
    log "$formula already installed"
  else
    log "brew install $formula"
    brew install "$formula"
  fi
}

ensure_brew node
if ! command -v rustup >/dev/null 2>&1 && ! command -v cargo >/dev/null 2>&1; then
  log "installing rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
command -v rustup >/dev/null 2>&1 || die "rustup is not on PATH; reopen the shell or source ~/.cargo/env"
rustup component add rustfmt clippy >/dev/null

append_path_line() {
  local file="$1"
  local line="$2"
  [[ -f "$file" ]] || touch "$file"
  grep -Fqx "$line" "$file" 2>/dev/null || printf '\n%s\n' "$line" >>"$file"
}

BREW_PREFIX="$(brew --prefix)"
append_path_line "$HOME/.zprofile" "eval \"\$(${BREW_PREFIX}/bin/brew shellenv)\""
append_path_line "$HOME/.zprofile" "[[ -f \"\$HOME/.cargo/env\" ]] && source \"\$HOME/.cargo/env\""

mint_registration_token() {
  local api_token="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
  [[ -n "$api_token" ]] || return 1
  local repo="${REPO_URL#https://github.com/}"
  repo="${repo%.git}"
  python3 - "$api_token" "$repo" <<'PY'
import json, sys, urllib.request
token, repo = sys.argv[1], sys.argv[2]
req = urllib.request.Request(
    f"https://api.github.com/repos/{repo}/actions/runners/registration-token",
    method="POST",
    headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "idevice-desktop-device-rig",
    },
)
with urllib.request.urlopen(req) as response:
    print(json.load(response)["token"])
PY
}

REGISTRATION_TOKEN="${GITHUB_RUNNER_TOKEN:-}"
if [[ -z "$REGISTRATION_TOKEN" ]]; then
  REGISTRATION_TOKEN="$(mint_registration_token || true)"
fi

mkdir -p "$RUNNER_DIR"
cd "$RUNNER_DIR"

if [[ ! -x ./config.sh ]]; then
  log "downloading the latest GitHub Actions runner"
  latest="$(python3 - <<'PY'
import json, urllib.request
req = urllib.request.Request(
    "https://api.github.com/repos/actions/runner/releases/latest",
    headers={"User-Agent": "idevice-desktop-device-rig", "Accept": "application/vnd.github+json"},
)
with urllib.request.urlopen(req) as response:
    tag = json.load(response)["tag_name"]
print(tag[1:] if tag.startswith("v") else tag)
PY
)"
  archive="actions-runner-osx-arm64-${latest}.tar.gz"
  curl -fsSL -o "$archive" \
    "https://github.com/actions/runner/releases/download/v${latest}/${archive}"
  tar xzf "$archive"
  rm -f "$archive"
fi

if [[ -f .runner && "${REPLACE_RUNNER:-0}" != 1 ]]; then
  log "runner already configured in $RUNNER_DIR (set REPLACE_RUNNER=1 to reconfigure)"
else
  [[ -n "$REGISTRATION_TOKEN" ]] || die "set GITHUB_RUNNER_TOKEN (or GH_TOKEN with repo admin) to register the runner"
  replace_flag=()
  if [[ -f .runner || "${REPLACE_RUNNER:-0}" == 1 ]]; then
    replace_flag=(--replace)
  fi
  log "configuring runner $RUNNER_NAME for $REPO_URL"
  ./config.sh --unattended \
    --url "$REPO_URL" \
    --token "$REGISTRATION_TOKEN" \
    --name "$RUNNER_NAME" \
    --labels "$RUNNER_LABELS" \
    --work "_work" \
    "${replace_flag[@]+"${replace_flag[@]}"}"
fi
unset REGISTRATION_TOKEN GITHUB_RUNNER_TOKEN GH_TOKEN GITHUB_TOKEN

ENV_FILE="$RUNNER_DIR/.env"
touch "$ENV_FILE"
ensure_env() {
  local key="$1"
  local value="$2"
  if grep -q "^${key}=" "$ENV_FILE" 2>/dev/null; then
    return
  fi
  printf '%s=%s\n' "$key" "$value" >>"$ENV_FILE"
}
ensure_env PATH "/opt/homebrew/bin:/usr/local/bin:${HOME}/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin"

if [[ ! -x ./svc.sh ]]; then
  die "svc.sh is missing; the runner archive did not extract correctly"
fi

if ./svc.sh status >/dev/null 2>&1; then
  log "launchd service already installed"
else
  log "installing launchd service (LaunchAgent via svc.sh)"
  ./svc.sh install
fi
./svc.sh start || true
./svc.sh status || log "warning: service status check failed; look at ~/Library/LaunchAgents/actions.runner.*.plist"

log "done. Confirm the runner is Idle under the repo Settings → Actions → Runners."
log "A LaunchAgent starts with this user session. On a headless Mac mini enable automatic login for this account."
