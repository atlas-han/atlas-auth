#!/usr/bin/env bash
#
# WorktreeRemove hook — integrate a worktree's work into the default branch
# BEFORE the worktree is cleaned up.
#
# Flow (per user config): rebase the worktree's branch onto the default branch
# (main), then fast-forward the default branch to include it. On ANY conflict
# or failure — including uncommitted changes — abort safely and BLOCK the
# removal (continue:false) so the worktree, and the work in it, are kept for
# manual handling. Clean / nothing-to-merge worktrees are allowed through
# untouched (so unchanged subagent/isolation worktrees still auto-clean).
#
# Reads the hook payload as JSON on stdin; operates entirely via `git -C` on
# absolute worktree paths, so it does not depend on the hook's cwd.

set -uo pipefail

LOG_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}/.claude/harness"
LOG="$LOG_DIR/worktree-premerge.log"

log() {
  mkdir -p "$LOG_DIR" 2>/dev/null || true
  printf '%s %s\n' "$(date '+%Y-%m-%dT%H:%M:%S')" "$*" >>"$LOG" 2>/dev/null || true
}

# Minimal JSON string encoder (handles the chars our messages can contain).
json_str() {
  local s=$1
  s=${s//\\/\\\\}
  s=${s//\"/\\\"}
  s=${s//$'\n'/\\n}
  s=${s//$'\t'/\\t}
  printf '"%s"' "$s"
}

# Prevent the worktree removal and surface why. Work is preserved.
block() {
  log "BLOCK: $1"
  printf '{"continue": false, "stopReason": %s, "systemMessage": %s}\n' \
    "$(json_str "$1")" "$(json_str "$1")"
  exit 0
}

# Let the removal proceed.
allow() {
  [ -n "${1:-}" ] && log "OK: $1"
  exit 0
}

INPUT=$(cat 2>/dev/null || true)

# Pull the first string value for a flat JSON key out of the payload.
extract() {
  printf '%s' "$INPUT" |
    sed -n 's/.*"'"$1"'"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
    head -n1
}

WT=""
for k in worktree_path worktreePath worktree_dir worktree path cwd dir directory; do
  v=$(extract "$k")
  if [ -n "$v" ]; then WT="$v"; break; fi
done
[ -z "$WT" ] && WT="${CLAUDE_WORKTREE_PATH:-}"
[ -z "$WT" ] && WT="$PWD"

log "fired: WT='$WT'"

# Must be a real git worktree.
if ! git -C "$WT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  allow "not a git worktree: $WT"
fi

# Resolve the worktree's branch. Detached HEAD => nothing to integrate.
BRANCH=$(git -C "$WT" symbolic-ref --quiet --short HEAD 2>/dev/null || true)
[ -z "$BRANCH" ] && allow "detached HEAD in $WT — nothing to merge"

# Determine the default branch (origin/HEAD if known, else 'main').
DEFAULT=$(git -C "$WT" symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null | sed 's#^origin/##')
[ -z "$DEFAULT" ] && DEFAULT="main"

# Already on the default branch => nothing to do.
[ "$BRANCH" = "$DEFAULT" ] && allow "$WT is on '$DEFAULT' — nothing to merge"

# The default branch must exist locally to rebase onto / advance.
if ! git -C "$WT" rev-parse --verify --quiet "refs/heads/$DEFAULT" >/dev/null 2>&1; then
  allow "default branch '$DEFAULT' not found locally — leaving worktree as-is"
fi

# Any commits on the branch not yet in the default branch?
AHEAD=$(git -C "$WT" rev-list --count "$DEFAULT".."$BRANCH" 2>/dev/null || echo 0)
[ "${AHEAD:-0}" = "0" ] && allow "'$BRANCH' has no commits ahead of '$DEFAULT' — nothing to merge"

# Refuse to drop uncommitted work on the floor.
if [ -n "$(git -C "$WT" status --porcelain 2>/dev/null)" ]; then
  block "Worktree '$WT' has uncommitted changes on '$BRANCH'. Commit or discard them, then retry cleanup. (worktree kept)"
fi

# Rebase the branch onto the default branch.
if ! git -C "$WT" rebase "$DEFAULT" >/dev/null 2>&1; then
  git -C "$WT" rebase --abort >/dev/null 2>&1 || true
  block "Rebase of '$BRANCH' onto '$DEFAULT' hit conflicts. Resolve them in '$WT', then retry cleanup. (worktree kept)"
fi
log "rebased '$BRANCH' onto '$DEFAULT' ($AHEAD commit(s))"

# Find the worktree that has the default branch checked out (if any).
MAIN_WT=$(git -C "$WT" worktree list --porcelain 2>/dev/null | awk -v def="refs/heads/$DEFAULT" '
  /^worktree /{wt=substr($0, 10)}
  $0 == "branch " def {print wt; exit}
')

if [ -n "$MAIN_WT" ]; then
  # Default branch is checked out somewhere; advance it there so its working
  # tree stays consistent. Refuse if that tree is dirty (we will not stomp it).
  if [ -n "$(git -C "$MAIN_WT" status --porcelain 2>/dev/null)" ]; then
    block "Cannot fast-forward '$DEFAULT': its working tree '$MAIN_WT' has uncommitted changes. Commit/stash there, then retry cleanup. (worktree kept)"
  fi
  if ! git -C "$MAIN_WT" merge --ff-only "$BRANCH" >/dev/null 2>&1; then
    block "Fast-forward merge of '$BRANCH' into '$DEFAULT' failed in '$MAIN_WT'. Merge manually, then retry cleanup. (worktree kept)"
  fi
else
  # Default branch not checked out anywhere — just move its ref forward.
  if ! git -C "$WT" branch -f "$DEFAULT" "$BRANCH" >/dev/null 2>&1; then
    block "Failed to advance '$DEFAULT' to '$BRANCH'. Merge manually, then retry cleanup. (worktree kept)"
  fi
fi

log "merged '$BRANCH' into '$DEFAULT' — allowing cleanup"
allow ""
