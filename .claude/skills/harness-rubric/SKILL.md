---
name: harness-rubric
description: >
  The evaluation rubric the harness grades changes against — criteria with hard
  PASS/FAIL thresholds for atlas-auth (correctness, security/token invariants,
  main.rs wiring reality, test adequacy, convention/error-model adherence, SOLID
  design, docs). Use when running the evaluator agent, /qa, or the evaluation
  step of /feature, so grading is consistent and skeptical across runs.
---

# atlas-auth evaluation rubric

Borrowed from the article's harness: a fixed set of criteria, each with a **hard
threshold**. If any criterion falls below its threshold, the change **FAILS** and
the developer gets specific feedback. Grade skeptically — default to FAIL until
you have positive evidence (a test you ran, or code you read at `file:line`).
This rubric is most load-bearing when the task sits at the edge of what the model
does reliably solo; on routine changes it still catches regressions.

## Criteria & hard thresholds

| # | Criterion | PASS threshold | Automatic FAIL |
|---|-----------|----------------|----------------|
| 1 | **Correctness & spec fidelity** | Every item in the blueprint's definition of done is exercised and holds | Any DoD item unmet or untested |
| 2 | **Security & token invariants** | Refresh tokens stored only as SHA-256 hash; reuse revokes the family; PKCE S256 enforced; redirect exact-match + scope allowlists; consent for non-first-party; no secret/plaintext logged; two refresh paths stay distinct; Argon2id+pepper intact | Any security regression, no matter how "minor" |
| 3 | **Wiring reality** | Feature is reachable in `main.rs` (repo constructed, `.app_data`, route `.configure`); the degrade-vs-500 behavior matches intent | Implemented+tested but inert (unwired) when it was meant to be live |
| 4 | **Test adequacy** | In-memory-backed handler test(s) exist; domain logic has `#[cfg(test)]` unit tests; migration change has its paired `*_schema_migration.rs` text assertion; one concern per test file | New behavior with no test, or a schema change with no schema-test update |
| 5 | **Convention & error model** | Routes do validation/shaping only; `AppError` for `/v1/auth/*` & recovery, `OAuthErrorResponse` for OAuth/admin; idioms match neighbors | Wrong error type for the surface, or logic leaked into a route |
| 6 | **Design quality (SOLID)** | No High-severity SOLID finding (esp. a Liskov gap between repo variants); dependency inversion preserved | A Liskov gap that makes tests lie, or domain reaching for raw SQL |
| 7 | **Build & gates** | `cargo build`, `cargo test` (targeted or full), `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check` all green; coverage gate met when domain logic changed | Any gate red |
| 8 | **Docs** | User-visible or convention changes reflected in README / CLAUDE.md / docs; no doc claims a behavior the code doesn't have | Stale/incorrect docs for a shipped behavior |

## Weighting (where to push hardest)
Like the article weighted design+originality over craft, weight **#1 correctness,
#2 security, and #3 wiring reality** above the rest — those are where this auth
server's failures are most costly and where superficial QA misses the most.
Craft-level gates (#7 build/lint/format) are table stakes: they should pass by
default, and failing them is a fast, unambiguous FAIL.

## Reporting format
- **Overall: PASS / FAIL** (FAIL if any criterion below threshold).
- **Per-criterion**: criterion | PASS/FAIL | evidence (`file:line` or command
  output) | required fix if FAIL.
- **Findings** as contract-style, immediately-actionable items (what's wrong,
  where, and the fix) — no "consider maybe looking into" vagueness.
- **Command evidence**: the actual `cargo test` / `clippy` / `fmt` output.

## Iteration loop
On FAIL, hand the findings back to the developer and re-run after the fix. Stop
when the change PASSES all criteria, or after the round budget (default 3) — then
report the remaining gaps honestly rather than approving around them.
