#!/usr/bin/env bash
# atlas-auth harness — PostToolUse hook (Edit|Write|MultiEdit)
#
# Two lightweight quality gates that run after every file edit:
#   1. Auto-format edited Rust files with rustfmt (cargo fmt defaults, edition 2024)
#      so the `cargo fmt --all -- --check` CI gate never trips on whitespace.
#   2. When a migration SQL is touched, remind about the paired
#      tests/*_schema_migration.rs text assertion (a project convention).
#
# Design notes:
#   - rustfmt on a single file is fast; clippy is intentionally NOT run here
#     (too slow for per-edit) — that belongs to the evaluator agent.
#   - Always exits 0 so a missing tool or parse error never blocks the session.
set -u

input="$(cat)"

# --- extract tool_input.file_path from the hook JSON on stdin ---------------
file_path=""
if command -v jq >/dev/null 2>&1; then
  file_path="$(printf '%s' "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null)"
elif command -v python3 >/dev/null 2>&1; then
  file_path="$(printf '%s' "$input" | python3 -c 'import sys,json
try:
    ti = json.load(sys.stdin).get("tool_input", {})
    print(ti.get("file_path") or ti.get("path") or "")
except Exception:
    pass' 2>/dev/null)"
fi

[ -z "$file_path" ] && exit 0
[ -f "$file_path" ] || exit 0

# --- 1. format Rust ---------------------------------------------------------
case "$file_path" in
  *.rs)
    if command -v rustfmt >/dev/null 2>&1; then
      rustfmt --edition 2024 "$file_path" >/dev/null 2>&1 || true
    fi
    ;;
esac

# --- 2. migration / schema-test pairing reminder ----------------------------
case "$file_path" in
  *migrations/*.sql)
    base="$(basename "$file_path" .sql)"
    printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"Migration %s.sql was edited. Per project convention, update the paired tests/*_schema_migration.rs text assertion (fs::read_to_string + .contains(...)) so the schema test still matches the SQL. Renaming a column/index requires editing BOTH the migration and its schema test."}}\n' "$base"
    ;;
esac

exit 0
