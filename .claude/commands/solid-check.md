---
description: Audit the current diff (or a given path) against SOLID principles
argument-hint: [optional file/module path; defaults to the working diff]
allowed-tools: Task, Read, Grep, Glob, Bash
---

Run a SOLID design audit using the `solid-reviewer` subagent and the `solid-rust`
skill.

## Scope
- If `$ARGUMENTS` names a file or module, audit that.
- Otherwise audit the current change set.

Current working diff (staged + unstaged):

!`git diff HEAD --stat 2>/dev/null; echo '---'; git diff HEAD 2>/dev/null | head -400`

## What to do
Spawn the `solid-reviewer` subagent with the scope above. It must:
- Evaluate all five principles, with special attention to **Liskov** — confirm the
  `Postgres` and `InMemory` variants of any touched repository are behaviorally
  interchangeable (a gap there makes the in-memory-backed tests lie), and to
  **Dependency Inversion** — confirm no domain logic reached for raw SQL/`PgPool`.
- Cite `file:line` for every finding, rate each High/Medium/Low, and propose the
  minimal behavior-preserving fix.
- Write the report to `.claude/harness/solid-<slug>.md` and return the path.

Then summarize the verdict here: per-principle OK/Issues, the top actions, and
whether anything is High severity (which should block merge). If the code is
clean, say so and name what was checked — don't manufacture findings.
