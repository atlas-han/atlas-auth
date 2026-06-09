---
description: Write a structured handoff artifact so the next session resumes cleanly
argument-hint: <short topic/slug for this handoff>
allowed-tools: Read, Grep, Glob, Bash, Write
---

Produce a **structured handoff artifact** for the work in progress. This is the
article's answer to context limits: instead of trusting a long context to stay
coherent, capture enough state that a fresh agent (after a context reset or in the
next session) can pick the work up cleanly. The SessionStart hook surfaces the
newest handoff automatically.

Topic/slug: $ARGUMENTS

## Gather current state
- Branch & status: !`git status -sb 2>/dev/null | head -30`
- Recent commits: !`git log --oneline -8 2>/dev/null`
- Working diff summary: !`git diff HEAD --stat 2>/dev/null | tail -30`
- Existing harness artifacts: !`ls -t .claude/harness/*.md 2>/dev/null | head -10`

## Write the handoff
Write to `.claude/harness/handoff-<slug>.md` (create `.claude/harness/` if needed),
with these sections, kept tight and factual:

1. **Goal** — what we're building, in 2-3 sentences.
2. **State now** — what is done and verified (cite tests that pass), what is
   in-progress, what is untouched.
3. **Key decisions & constraints** — design choices already made and why, plus the
   invariants that must not regress (token hashing, rotation/reuse, PKCE, consent,
   `main.rs` wiring, migration↔schema-test pairing).
4. **Next steps** — an ordered, concrete checklist the next agent should execute.
5. **Landmines** — known gaps, failing tests, flaky areas, or things that look
   done but aren't (e.g. implemented-but-unwired repositories).
6. **Where to look** — the exact files and `.claude/harness/` artifacts to read
   first.

Make it self-contained: the next agent should need only this file plus the repo to
continue. End by printing the handoff path and a 3-line summary.
