#!/usr/bin/env bash
# pacseek manual updater: fast-forward pull + release rebuild + reinstall.
# Usage:
#   ./update.sh                 # rebuild + sudo reinstall to /usr/local/bin
#   ./update.sh --user          # rebuild + reinstall to ~/.cargo/bin (no sudo)
#   ./update.sh --prefix /usr   # system reinstall to a custom prefix
#   ./update.sh --check         # show repo vs installed versions, change nothing
#
# Safety:
#   - refuses on a dirty working tree (commit or stash first)
#   - git pull is --ff-only: never merges or rewrites your local commits
#   - unpushed local commits are kept as-is when the remote is an ancestor
set -euo pipefail

PREFIX="/usr/local"
USER_MODE=0
CHECK_ONLY=0
BIN_NAME="pacseek"

log() { printf '[update] %s\n' "$*"; }
die() { printf '[update] ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
  sed -n '2,14p' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      PREFIX="${2:-}"
      if [[ -z "$PREFIX" ]]; then die "--prefix needs a value"; fi
      shift 2
      ;;
    --prefix=*) PREFIX="${1#*=}"; shift ;;
    --user) USER_MODE=1; shift ;;
    --check) CHECK_ONLY=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown flag: $1 (see --help)" ;;
  esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# Fail fast: a broken toolchain (Arch Rust/LLVM ABI skew) otherwise dies
# mid-build with a cryptic `rustc -vV` error.
if ! rustc -vV >/dev/null 2>&1; then
  die "rustc is broken ($(rustc -vV 2>&1 | head -n 1)). Run: sudo pacman -Syu (partial upgrades break the Rust/LLVM ABI), then re-run $0"
fi

repo_version() {
  grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2
}

installed_version() {
  # $1 = binary path; prints version or "not installed"
  if [[ -x "$1" ]]; then
    "$1" --version 2>/dev/null || echo "unknown"
  else
    echo "not installed"
  fi
}

if [[ $CHECK_ONLY -eq 1 ]]; then
  echo "repo:         $(repo_version) ($(git log --oneline -1))"
  echo "/usr/local:   $(installed_version /usr/local/bin/$BIN_NAME)"
  echo "\$HOME/.cargo: $(installed_version "$HOME/.cargo/bin/$BIN_NAME")"
  echo "in PATH:      $(installed_version "$(command -v $BIN_NAME 2>/dev/null || echo /nonexistent)")"
  git fetch origin -q 2>/dev/null || true
  git status --short --branch | head -n 5
  exit 0
fi

# 1. Refuse on dirty tree — updating over uncommitted work is how code gets lost.
if [[ -n "$(git status --porcelain)" ]]; then
  git status --short | head -n 10
  die "working tree dirty — commit or stash first, then re-run"
fi

# 2. Fast-forward only: keeps unpushed local commits, never merges.
log "pulling latest (ff-only)..."
git pull --ff-only 2>&1 | tail -n 3 || die "git pull --ff-only failed (diverged history?) — resolve manually, then re-run"

log "repo version: $(repo_version)"

# 3. Rebuild.
log "building release binary..."
cargo build --release --locked
ls -lh "target/release/$BIN_NAME"

# 4. Reinstall to the same place the user already uses.
if [[ $USER_MODE -eq 1 ]]; then
  log "installing to ~/.cargo/bin via cargo install"
  cargo install --path . --force --locked
  TARGET="$HOME/.cargo/bin/$BIN_NAME"
  [[ ":$PATH:" == *":$HOME/.cargo/bin:"* ]] || log "NOTE: ~/.cargo/bin not in PATH — your shell uses $(command -v $BIN_NAME)"
else
  SUDO=""
  if [[ $EUID -ne 0 ]]; then
    command -v sudo >/dev/null 2>&1 || die "need root or sudo to write $PREFIX/bin"
    SUDO="sudo"
  fi
  DEST="$PREFIX/bin/$BIN_NAME"
  log "installing to $DEST"
  $SUDO install -Dm755 "target/release/$BIN_NAME" "$DEST"
  TARGET="$DEST"
fi

# 5. Verify (captured, not piped: SIGPIPE + pipefail would fake a failure).
log "verifying..."
"$TARGET" --version
INSTALLED_VER="$("$TARGET" --version 2>/dev/null || true)"
if [[ "$INSTALLED_VER" != *"$(repo_version)"* ]]; then
  log "WARNING: installed ($INSTALLED_VER) differs from repo $(repo_version)"
fi
SMOKE_OUT="$("$TARGET" firefox --no-tui --limit 2 --no-color 2>&1 || true)"
if [[ -n "$SMOKE_OUT" ]]; then
  printf '%s\n' "$SMOKE_OUT" | head -n 8
  log "smoke search OK"
else
  log "smoke search had no output (AUR offline or empty DB?) — binary still installed"
fi

log "done: $TARGET ($("$TARGET" --version))"
