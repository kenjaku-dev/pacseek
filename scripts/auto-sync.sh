#!/usr/bin/env bash
# Auto-commit + push pacseek when working tree settles.
#
#   scripts/auto-sync.sh           start (background)
#   scripts/auto-sync.sh --status  status
#   scripts/auto-sync.sh --stop    stop
#   scripts/auto-sync.sh --once    one pass (for testing/cron)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PIDFILE="${AUTO_SYNC_PIDFILE:-${TMPDIR:-/tmp}/pacseek-auto-sync.pid}"
LOG="${AUTO_SYNC_LOG:-${TMPDIR:-/tmp}/pacseek-auto-sync.log}"
DEBOUNCE="${AUTO_SYNC_DEBOUNCE:-8}"
POLL="${AUTO_SYNC_POLL:-3}"
REMOTE="${AUTO_SYNC_REMOTE:-origin}"
BRANCH="${AUTO_SYNC_BRANCH:-master}"
PUSH_REF="${AUTO_SYNC_PUSH_REF:-main}"

log() { printf '%s %s\n' "$(date -Iseconds)" "$*" >>"$LOG"; }

is_running() {
  [[ -f "$PIDFILE" ]] && kill -0 "$(cat "$PIDFILE" 2>/dev/null)" 2>/dev/null
}

stop() {
  if is_running; then
    kill "$(cat "$PIDFILE")" 2>/dev/null || true
    rm -f "$PIDFILE"
    echo "stopped"
  else
    echo "not running"
  fi
}

status() {
  if is_running; then
    echo "running pid=$(cat "$PIDFILE")"
    echo "log=$LOG"
  else
    echo "stopped"
  fi
}

commit_message() {
  local paths n area
  paths="$(git diff --cached --name-only)"
  n="$(printf '%s\n' "$paths" | sed '/^$/d' | wc -l | tr -d ' ')"
  area="repo"
  if printf '%s\n' "$paths" | grep -q '^pacseek-landing/' &&
    ! printf '%s\n' "$paths" | grep -qv '^pacseek-landing/'; then
    area="landing"
  elif printf '%s\n' "$paths" | grep -qE '^(src/|tests/|Cargo\.|rustfmt)' &&
    ! printf '%s\n' "$paths" | grep -qvE '^(src/|tests/|Cargo\.|rustfmt|Makefile|README|config\.example|INSTALL|context|LICENSE|\.github)'; then
    area="rust"
  fi
  local preview
  preview="$(printf '%s\n' "$paths" | sed '/^$/d' | head -3 | paste -sd ', ' -)"
  if ((n > 3)); then
    preview="$preview, +$((n - 3)) more"
  fi
  printf 'auto(%s): %s file(s) — %s\n' "$area" "$n" "$preview"
}

push_with_retry() {
  local attempt
  for attempt in 1 2 3; do
    if git push "$REMOTE" "HEAD:$PUSH_REF"; then
      return 0
    fi
    log "push failed (attempt $attempt)"
    sleep $((attempt * 2))
  done
  return 1
}

do_sync() {
  if [[ -f .git/index.lock ]]; then
    return 1
  fi

  if [[ -z "$(git status --porcelain)" ]]; then
    return 1
  fi

  git add -A
  if git diff --cached --quiet; then
    # only ignored/no-op changes
    return 1
  fi

  local msg
  msg="$(commit_message)"
  git -c user.name="${GIT_AUTHOR_NAME:-$(git config user.name || echo achraf)}" \
      -c user.email="${GIT_AUTHOR_EMAIL:-$(git config user.email || echo achraf@pacseek.local)}" \
      commit -q -m "$msg"
  log "committed: $msg"

  # keep local history linear if remote moved
  git fetch "$REMOTE" "$PUSH_REF" -q 2>/dev/null || true
  if ! git merge-base --is-ancestor "$REMOTE/$PUSH_REF" HEAD 2>/dev/null; then
    git rebase -q "$REMOTE/$PUSH_REF" 2>/dev/null || true
  fi

  if push_with_retry; then
    log "pushed to $REMOTE/$PUSH_REF"
    return 0
  fi
  log "push failed after retries (commit kept locally)"
  return 1
}

run_once() {
  do_sync || true
}

run_loop() {
  echo $$ >"$PIDFILE"
  trap 'rm -f "$PIDFILE"' EXIT
  log "watcher started (debounce=${DEBOUNCE}s poll=${POLL}s → $REMOTE/$PUSH_REF)"
  local quiet=0
  while true; do
    sleep "$POLL"
    if [[ -f .git/index.lock ]]; then
      quiet=0
      continue
    fi
    if [[ -z "$(git status --porcelain)" ]]; then
      quiet=0
      continue
    fi
    quiet=$((quiet + POLL))
    if ((quiet < DEBOUNCE)); then
      continue
    fi
    quiet=0
    do_sync || true
  done
}

case "${1:-}" in
  --stop) stop ;;
  --status) status ;;
  --once) run_once ;;
  --run) run_loop ;;
  *)
    if is_running; then
      status
      exit 0
    fi
    nohup "$0" --run >>"$LOG" 2>&1 &
    echo $! >"$PIDFILE"
    # give child a moment; it also writes pidfile — keep parent start pid if child not ready
    sleep 0.2
    if ! is_running; then
      echo $! >"$PIDFILE"
    fi
    echo "started pid=$(cat "$PIDFILE")"
    echo "log=$LOG"
    ;;
esac
