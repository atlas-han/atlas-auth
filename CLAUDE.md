# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Atlas Auth is a Rust (Actix-Web + PostgreSQL/SQLx) central authentication server for Atlas mobile apps and web services: password auth, OAuth 2.0 / OIDC provider endpoints, refresh-token rotation, and social login. Some design docs in `docs/` are written in Korean.

## Commands

```bash
# Build / run (run needs a populated .env — see below)
cargo build
cargo run

# Tests
cargo test                              # all tests
cargo test --test oauth_authorize       # one integration-test file (tests/oauth_authorize.rs)
cargo test refresh_token_rotation       # tests whose name matches a substring
cargo test --lib                        # only in-source #[cfg(test)] unit tests

# Lint / format (CI gates on these — see .github/workflows/ci.yml)
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Coverage gate (requires cargo-llvm-cov); 90% line floor on pure domain logic only
./scripts/coverage-unit.sh

# Database (most tests need NO database — see Testing)
docker compose up -d postgres           # local Postgres on :5432, user/pass/db = atlas_auth
sqlx migrate run                         # apply migrations/ (requires sqlx-cli + DATABASE_URL)
```

First-time setup (`.env` + RS256 keys) is documented in `README.md`. `Settings::from_env` (`src/config.rs`) loads via dotenvy; `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` are stored single-line with `\n` escapes and un-escaped at load. Required with no default: `DATABASE_URL`, `JWT_PRIVATE_KEY_PEM`, `JWT_PUBLIC_KEY_PEM`, `PASSWORD_PEPPER`.

Building needs a stable toolchain new enough for `edition2024` (Rust ≥ 1.85); a transitive dependency in `Cargo.lock` fails to resolve on older toolchains (e.g. Cargo 1.83) even though `Cargo.toml` pins `rust-version = 1.78`.

## Architecture

Layered, roughly Clean Architecture: **HTTP routes** (`src/routes/`, DTO validation + response shaping only) → **domain logic** (`src/auth/`, `src/oauth/`, mostly pure functions) → **repositories** (PostgreSQL) → **PostgreSQL**. JWT, Argon2, and PKCE are infrastructure helpers. `src/lib.rs` exposes the modules; `src/main.rs` is the binary that wires the server. `AppState { pool, settings }` is shared via `web::Data`.

Three cross-cutting patterns matter more than any single file:

**1. Dual-backend repositories.** Every repository is an enum (or struct) with a `Postgres(PgPool)` variant and an `InMemory(...)` variant — `OAuthClientRepository`, `RefreshTokenRepository`, `AuthorizationCodeRepository`, `ConsentRepository`, `FederatedIdentityRepository`, `AccountRecoveryRepository`, `LoginAttemptRepository`, `SigningKeyRepository` (the admin `AdminUserRepository` is in-memory only). Use `::postgres(pool)` in `main.rs`, `::in_memory(...)` in tests. This is what lets HTTP-handler tests run with zero database.

**2. `Option<web::Data<Repo>>` graceful degradation.** Many handlers take repositories as `Option<web::Data<Repo>>`. When a dependency is absent, the handler skips that behavior (or returns a protocol-level "validated" stub) instead of doing DB work. **Consequence:** a feature can be fully implemented and tested yet inert in the running binary, because `main.rs` never registers its repository. When wiring a new feature, construct its repository in `main.rs`, `.app_data(...)` it, and (for new route groups) add `.configure(...)`.

**3. What `main.rs` currently wires.** Repositories: `OAuthClientRepository`, `AuthorizationCodeRepository`, `RefreshTokenRepository` (all Postgres). Routes: `health`, `auth`, `oauth`, `oidc`. The following are implemented + tested but **not registered in `main.rs`**, with two distinct failure modes:
- *Silently degrade* (taken as `Option<web::Data<_>>`, so the handler runs without them): `SigningKeyRepository` → JWKS/signing falls back to settings PEM instead of DB rotation; `ConsentRepository` → consent check is skipped; `LoginAttemptRepository` → login lockout is skipped.
- *Fail at request time* (taken as a **required** `web::Data<_>`, so the route 500s on a missing extractor): `/v1/auth/email/verify` and `/v1/auth/password/reset` need `AccountRecoveryRepository`; `/v1/auth/social/{provider}/callback` needs `FederatedIdentityRepository`.
- *Unreachable*: `routes::admin` (the whole `/admin/*` surface, plus its in-memory `AdminUserRepository`) is never `.configure`d.

## Auth & token specifics

- **Access tokens**: stateless RS256 JWT; header carries `kid`; claims `sub/iss/aud/scope/iat/exp/jti` (`src/auth/token.rs`). ID tokens issued for OIDC when the `openid` scope is present.
- **Refresh tokens**: 96-char random, persisted **only as a SHA-256 hex hash** (`hash_refresh_token`), never plaintext. Rotation tracks a `family_id`; presenting an already-revoked token is reuse → the whole family is revoked.
- There are **two refresh-token paths**: the password flow in `routes/auth.rs` (direct SQL on `refresh_tokens`, audit events) and the OAuth flow via `oauth/refresh_token_repository.rs` + `auth/token.rs::rotate_refresh_token`. Keep them distinct.
- **Passwords**: Argon2id (`src/auth/password.rs`) with a server-side `PASSWORD_PEPPER`. Account-recovery tokens (email verify / password reset) are hashed with the same SHA-256 helper.
- **Signing keys**: come from the DB (`signing_keys` via `SigningKeyRepository::latest_active`) or fall back to settings PEM. The `*_from_repository_or_settings` helpers in `routes/oauth.rs` encode this fallback.

## OAuth / OIDC

- Authorization Code + **PKCE S256 is required**; implicit flow is rejected. Grants: `authorization_code`, `refresh_token`, `client_credentials` (the latter may not request `openid`).
- Clients: confidential clients authenticated by client-secret hash; `redirect_uri` is exact-match allowlisted; scopes are allowlisted per client. Non-`trusted_first_party` clients require prior stored consent (`ConsentRepository::has_granted_scopes`) when the consent repo is wired.
- **OAuth/admin handlers return `HttpResponse` directly** with their own OAuth-style `{error, message}` JSON (`OAuthErrorResponse`) and OAuth error codes — they do **not** use `AppError`.

## Error handling

`src/error.rs::AppError` (thiserror) implements Actix `ResponseError`, mapping to status codes and a `{error, message}` JSON body; a sqlx unique-violation maps to `Conflict`. This is used by `/v1/auth/*` and account-recovery handlers. OAuth and admin handlers bypass it (see above).

## Testing conventions

- `tests/` holds integration tests, **one concern per file** (~39 files).
- **HTTP tests** build an app with `actix_web::test::init_service(App::new().configure(routes::X::configure))` and inject in-memory repositories via `.app_data(...)` — no database.
- **`*_schema_migration.rs` tests assert on the SQL text** of migration files (`fs::read_to_string` + `contains`). Renaming a column/index therefore requires editing both the migration and its schema test.
- `*_repository.rs` tests exercise the in-memory variant; pure domain unit tests live in `#[cfg(test)]` modules in `src/` (e.g. `auth/token.rs`).
- The coverage gate (`scripts/coverage-unit.sh`) enforces 90% lines but **ignores** `main/config/db/error`, all repositories, the route handlers, and `authorization_code.rs` — it targets pure domain logic only.

## Migrations

`migrations/NNNN_name.sql`, timestamp-prefixed (e.g. `202606080001_init.sql`), applied with `sqlx migrate run`. Pair each schema change with the matching `*_schema_migration.rs` text assertion.
