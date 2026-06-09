---
description: Skeptically evaluate the current changes against the harness rubric
argument-hint: [optional focus area or definition-of-done; defaults to the diff]
allowed-tools: Task, Read, Grep, Glob, Bash
---

Run the skeptical **evaluator** on the current changes — a separate judgment from
whoever wrote the code.

## Context for the evaluator
Optional focus / definition-of-done from the caller: $ARGUMENTS

Current working diff:

!`git diff HEAD --stat 2>/dev/null; echo '---'; git diff HEAD 2>/dev/null | head -500`

Any existing blueprint for this work:

!`ls -t .claude/harness/plan-*.md 2>/dev/null | head -1`

## What to do
Spawn the `evaluator` subagent. It must grade the change against the
`harness-rubric` skill's criteria and hard thresholds, defaulting to FAIL until it
has positive evidence. Require it to actually run:
- `cargo build`
- the relevant `cargo test` (targeted or full — most tests need no DB)
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `./scripts/coverage-unit.sh` if domain logic changed

It writes `.claude/harness/eval-<slug>.md` and returns the path.

Then report here: the **overall PASS/FAIL**, the per-criterion table, the
contract-style bug findings, and the real command output it saw. Do not soften the
verdict — if a criterion is below threshold, it's a FAIL with a concrete fix.
