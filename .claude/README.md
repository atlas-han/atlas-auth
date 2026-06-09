# atlas-auth Claude Code harness

A generator/evaluator harness for this repo, adapted from Anthropic's
[*Harness design for long-running application development*](https://www.anthropic.com/engineering/harness-design-long-running-apps).
The article's core ideas, mapped onto an Actix-Web + SQLx auth server:

- **Separate the agent doing the work from the agent judging it.** Tuning a
  standalone evaluator to be skeptical is far more tractable than getting the
  builder to be hard on its own work.
- **Plan high-level, build against a contract, grade against hard thresholds.**
- **Hand off state across context resets via files**, not a single long context.

## Roles (all senior experts)

| Agent | Article role | Who |
|-------|--------------|-----|
| `architect` | Planner | Principal architect, 15+ yrs — turns a 1-4 sentence ask into a high-level blueprint (stays high-level so detail errors don't cascade) |
| `developer` | Generator | Staff Rust engineer, 12+ yrs — implements against the blueprint, self-reviews before handoff |
| `evaluator` | Evaluator | Principal QA, 12+ yrs — **skeptical by default**, grades against the rubric's hard thresholds, runs the tests itself |
| `solid-reviewer` | (added) | Design specialist, 12+ yrs — audits SOLID, esp. Liskov parity between repo backends |
| `security-auditor` | (added) | AppSec engineer, 12+ yrs — token/OAuth/crypto threat model for the auth domain |
| `technical-writer` | (added) | Senior writer, 10+ yrs (EN/KO) — documents what actually shipped |

## Commands

| Command | What it does |
|---------|--------------|
| `/feature <1-4 sentence ask>` | Full loop: architect → developer → **skeptical** evaluator (iterate ≤3) → solid + security review → docs |
| `/solid-check [path]` | SOLID audit of the diff (or a path) via `solid-reviewer` |
| `/qa [focus]` | Skeptical evaluation of the current diff against the rubric |
| `/handoff <slug>` | Write a structured handoff artifact for the next session |

## Skills (shared knowledge)

- `solid-rust` — SOLID translated to idiomatic Rust + this codebase's layers,
  dual-backend repository enums, and `Option<web::Data>` injection.
- `harness-rubric` — the PASS/FAIL criteria with hard thresholds the evaluator
  grades against.

## Hooks (`settings.json`)

- **PostToolUse** (`hooks/rust-postedit.sh`) — auto-`rustfmt`s edited `.rs` files
  (keeps the `cargo fmt --check` CI gate green) and reminds about the paired
  `*_schema_migration.rs` test when a migration is edited.
- **SessionStart** (`hooks/session-handoff.sh`) — surfaces the newest
  `.claude/harness/handoff-*.md` so a fresh session resumes cleanly.

## Artifacts

Agents communicate through files under `.claude/harness/`:
`plan-<slug>.md`, `eval-<slug>.md`, `solid-<slug>.md`, `security-<slug>.md`,
`handoff-<slug>.md`. Safe to delete; they're a working scratchpad.

## When the harness is worth it

Per the article, the evaluator earns its cost when a task sits **at the edge of
what the model does reliably solo** — non-trivial features, security-sensitive
changes, refactors touching token/OAuth invariants. For a one-line fix, just edit
directly. Every component here encodes an assumption about what the model can't do
alone; re-examine them as the models improve.
