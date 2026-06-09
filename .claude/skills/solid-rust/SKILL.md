---
name: solid-rust
description: >
  SOLID design principles translated to idiomatic Rust and to atlas-auth's Clean
  Architecture (layered routes → domain → repositories, dual-backend repository
  enums, Option<web::Data> injection). Use when reviewing or designing for design
  quality, when the user mentions SOLID / single responsibility / open-closed /
  Liskov / interface segregation / dependency inversion, or when running
  /solid-check or the solid-reviewer agent.
---

# SOLID in Rust — atlas-auth edition

Rust has no classes or implementation inheritance, so apply the *intent* of each
principle through traits, enums, modules, generics, and ownership — not a literal
class-based reading. Below: what each principle means here, the concrete red
flags in this codebase, and the minimal fix. Only flag what the code actually
shows; do not manufacture violations.

## S — Single Responsibility
One module / type / function, one reason to change.
- **This codebase's natural seams:** routes (`src/routes/`) validate DTOs and
  shape responses; domain (`src/auth/`, `src/oauth/`) holds logic as mostly-pure
  functions; repositories own SQL. A responsibility crossing that boundary is the
  most common SRP violation here.
- **Red flags:** a handler doing validation + business rules + SQL; a function
  that mints a token *and* hashes *and* persists; a "utils" module accreting
  unrelated helpers; one repository method that both queries and mutates
  unrelated state.
- **Fix:** push logic down into the domain layer where it can be unit-tested
  (the coverage gate targets pure domain logic), keep the handler thin.

## O — Open/Closed
Open to extension, closed to modification — in Rust via traits, enums, generics.
- **Red flags:** adding a new social provider / grant type / repository backend
  forces edits to many scattered `match` arms or copy-pasted blocks; behavior
  that should be data/strategy is hard-coded in a conditional ladder.
- **Nuance:** a single exhaustive `match` on a small closed enum is *good* Rust
  (the compiler forces you to handle new variants) — don't "fix" it into dynamic
  dispatch for its own sake. The violation is *duplicated* logic that must change
  in lockstep across the codebase, not one well-placed match.
- **Fix:** introduce a trait or consolidate the variant logic behind one
  extension point — but only when there is a second real caller/variant.

## L — Liskov Substitution  (highest-value check in this repo)
Subtypes must honor the supertype's contract. Here, every repository's
`Postgres(PgPool)` and `InMemory(...)` variants must be **behaviorally
interchangeable**.
- **Red flags:** the in-memory variant skips a uniqueness / expiry / single-use /
  reuse-detection / consent check that Postgres enforces (or vice versa). Tests
  run against the in-memory variant, so a behavioral gap means **the tests pass
  against behavior the real backend doesn't have** — false confidence, and a
  potential security hole that no test catches.
- **How to check:** read both variants of the touched repository side by side and
  confirm they agree on invariants and error semantics (e.g. duplicate insert →
  same conflict behavior; expired/revoked row → same rejection).
- **Fix:** bring the lagging variant up to the same contract; add a test that
  asserts the shared invariant.

## I — Interface Segregation
No consumer depends on methods it doesn't use.
- **Red flags:** one fat repository enum/trait serving unrelated call sites; a
  handler taking dependencies (`web::Data<...>`) it never exercises; a trait so
  broad that the in-memory variant must stub half of it.
- **Fix:** split by cohesive use — but weigh against this codebase's preference
  for a single repository type per aggregate; don't shatter a cohesive repository
  into micro-traits.

## D — Dependency Inversion
High-level policy depends on abstractions, not concretions.
- **This codebase already inverts well:** handlers depend on repository
  abstractions injected as `Option<web::Data<Repo>>`; the enum hides Postgres vs
  in-memory; signing keys come from `SigningKeyRepository` or a settings fallback.
  **Protect this.**
- **Red flags:** domain logic (`src/auth/`, `src/oauth/`) reaching for `PgPool` /
  raw SQL directly; a handler hard-wired to `Repo::Postgres(...)` instead of the
  abstraction; business rules importing infrastructure types.
- **Fix:** depend on the repository surface, not the backend; keep SQL inside the
  repository layer.

## Severity guide
- **High** — a Liskov gap that makes tests lie, or a DIP break that leaks
  infrastructure into the domain (correctness/security risk).
- **Medium** — real design debt that will bite the next change (SRP leak into a
  handler, OCP duplication across the codebase).
- **Low** — stylistic / single-caller niceties. Don't over-abstract for one
  caller; "simplest solution that works" beats premature generality.
