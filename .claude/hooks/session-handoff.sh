#!/usr/bin/env bash
# atlas-auth harness — SessionStart hook (startup|resume|clear)
#
# Surfaces the most recent harness handoff artifact at the top of a fresh
# session. This implements the article's "structured handoff across context
# resets" idea: when a long-running build is split across sessions, the next
# agent starts by reading where the last one left off instead of re-deriving
# state. Handoffs are written by the /handoff command into .claude/harness/.
#
# Always exits 0; emits nothing when no handoff exists.
set -u

dir="${CLAUDE_PROJECT_DIR:-.}/.claude/harness"
[ -d "$dir" ] || exit 0

# newest handoff-*.md by name (timestamp-prefixed) or mtime
latest="$(ls -t "$dir"/handoff-*.md 2>/dev/null | head -n 1)"
[ -n "$latest" ] || exit 0
[ -f "$latest" ] || exit 0

# cap the surfaced content so we never flood context
body="$(head -n 120 "$latest")"

if command -v python3 >/dev/null 2>&1; then
  python3 - "$latest" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, "r", encoding="utf-8", errors="replace") as f:
    lines = f.readlines()[:120]
note = ("Latest harness handoff (" + path + "). Read this before continuing a "
        "multi-session build; the full file has more detail:\n\n" + "".join(lines))
print(json.dumps({"hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": note,
}}))
PY
else
  # best-effort fallback without JSON escaping guarantees
  printf 'Latest harness handoff: %s — read it before continuing a multi-session build.\n' "$latest"
fi

exit 0
