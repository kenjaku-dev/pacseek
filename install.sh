#!/usr/bin/env bash
# pacseek — automated build + install for Arch-family Linux.
# Usage:
#   ./install.sh                 # release build + sudo install to /usr/local/bin
#   ./install.sh --user          # install to ~/.cargo/bin (no sudo for binary)
#   ./install.sh --prefix /usr  # custom system prefix
#   ./install.sh --yes --no-sync
#
# What it does:
#   1. checks for Arch-family pacman (required: libalpm + pacman-conf)
#   2. installs missing build deps (base-devel, git, rust, pkgconf, pacman-contrib)
#   3. optionally refreshes pacman sync DB (needed for repo search)
#   4. cargo build --release --locked
#   5. installs binary (+ verifies --version and a smoke search)
set -euo pipefail

PREFIX="/usr/local"
USER_MODE=0
ASSUME_YES=0
SYNC_DB=1
BIN_NAME="pacseek"

log() { printf '[install] %s\n' "$*"; }
die() { printf '[install] ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
  sed -n '2,12p' "$0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix) PREFIX="${2:-}"; if [[ -z "$PREFIX" ]]; then die "--prefix needs a value"; fi; shift 2 ;;
    --prefix=*) PREFIX="${1#*=}"; shift ;;
    --user) USER_MODE=1; shift ;;
    --yes|-y) ASSUME_YES=1; shift ;;
    --no-sync) SYNC_DB=0; shift ;;
    --help|-h) usage; exit 0 ;;
    *) die "unknown flag: $1 (see --help)" ;;
  esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# 1. Arch-family check (libalpm only exists here)
if ! command -v pacman >/dev/null 2>&1; then
  die "pacman not found — pacseek needs Arch/Artix/CachyOS/Endeavour (libalpm). Generic Linux is not supported for repo search."
fi

SUDO=""
if [[ $EUID -ne 0 ]]; then
  command -v sudo >/dev/null 2>&1 || die "need root or sudo to install system packages"
  SUDO="sudo"
fi

# 2. Build deps
MISSING=()
for pkg in git pkgconf pacman-contrib; do
  pacman -Qi "$pkg" >/dev/null 2>&1 || MISSING+=("$pkg")
done
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
  MISSING+=("rust")
fi
# base-devel is a group; check for its key tools instead
if ! command -v gcc >/dev/null 2>&1 || ! command -v make >/dev/null 2>&1 || ! command -v makepkg >/dev/null 2>&1; then
  # pacman -Sg base-devel would list the group; install group if tools missing
  NEED_BASE_DEVEL=1
else
  NEED_BASE_DEVEL=0
fi

if [[ ${#MISSING[@]} -gt 0 || $NEED_BASE_DEVEL -eq 1 ]]; then
  INSTALL_PKGS=("${MISSING[@]}")
  [[ $NEED_BASE_DEVEL -eq 1 ]] && INSTALL_PKGS+=("base-devel")
  log "installing missing deps: ${INSTALL_PKGS[*]}"
  if [[ $ASSUME_YES -eq 1 ]]; then
    $SUDO pacman -Sy --noconfirm --needed "${INSTALL_PKGS[@]}"
  else
    $SUDO pacman -Sy --needed "${INSTALL_PKGS[@]}"
  fi
else
  log "build deps OK (git, rust, pkgconf, base-devel, pacman-contrib)"
fi

command -v cargo >/dev/null 2>&1 || die "cargo still missing after dep install"
log "rustc: $(rustc --version)"

# 3. Sync DB (repo search needs /var/lib/pacman/sync/*.db)
if [[ $SYNC_DB -eq 1 ]]; then
  log "refreshing pacman DB (sudo pacman -Sy)"
  if [[ $ASSUME_YES -eq 1 ]]; then
    $SUDO pacman -Sy --noconfirm || log "DB refresh failed — continuing (repo search may be empty)"
  else
    $SUDO pacman -Sy || log "DB refresh failed — continuing (repo search may be empty)"
  fi
fi

# 4. Build
log "building release binary..."
cargo build --release --locked
ls -lh target/release/pacseek

# 5. Install
if [[ $USER_MODE -eq 1 ]]; then
  log "installing to ~/.cargo/bin via cargo install"
  cargo install --path . --force --locked
  TARGET="$HOME/.cargo/bin/$BIN_NAME"
  [[ ":$PATH:" == *":$HOME/.cargo/bin:"* ]] || log "NOTE: ~/.cargo/bin not in PATH — add: export PATH=\"\$HOME/.cargo/bin:\$PATH\""
else
  DEST="$PREFIX/bin/$BIN_NAME"
  log "installing to $DEST"
  $SUDO install -Dm755 "target/release/$BIN_NAME" "$DEST"
  TARGET="$DEST"
fi

# 6. Verify (no pipe to head here: SIGPIPE + pipefail would fake a failure)
log "verifying..."
"$TARGET" --version
"$TARGET" --show-config || true
SMOKE_OUT="$("$TARGET" firefox --no-tui --limit 2 --no-color 2>&1 || true)"
if [[ -n "$SMOKE_OUT" ]]; then
  printf '%s\n' "$SMOKE_OUT" | head -n 8
  log "smoke search OK"
else
  log "smoke search had no output (AUR offline or empty DB?) — binary still installed"
fi

# 7. Default config (best effort, never overwrites)
CONFIG_PATH="$("$TARGET" --show-config 2>/dev/null | head -n 1)"
if [[ -n "$CONFIG_PATH" && ! -f "$CONFIG_PATH" ]]; then
  "$TARGET" --init-config 2>/dev/null || true
fi

log "done: $TARGET"
