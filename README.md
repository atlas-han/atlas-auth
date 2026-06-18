# Atlas Auth

Atlas Auth is a central authentication server for Atlas mobile apps and web services,
built in Rust on Actix-Web and PostgreSQL. It provides password authentication, an
OAuth 2.0 / OpenID Connect provider, refresh-token rotation with reuse detection, and
social (federated) login.

- **Language / framework:** Rust, [Actix-Web](https://actix.rs/) 4.9
- **Database:** PostgreSQL 16 via [SQLx](https://github.com/launchbadge/sqlx) 0.8 (`tls-rustls`)
- **Tokens:** stateless RS256 JWT access tokens + opaque, hashed refresh tokens
- **Password hashing:** Argon2id with a server-side pepper

Design docs live in [`docs/`](docs/) (some are written in Korean).

## Table of contents

- [Features](#features)
- [Architecture](#architecture)
- [Project layout](#project-layout)
- [Prerequisites](#prerequisites)
- [Configuration](#configuration)
- [Local development](#local-development)
- [API reference](#api-reference)
- [Testing](#testing)
- [Lint, format & coverage](#lint-format--coverage)
- [Docker](#docker)
- [Wiring status & roadmap](#wiring-status--roadmap)
- [Further documentation](#further-documentation)

## Features

- **Password auth** — register / login / logout with Argon2id hashing and a server-side `PASSWORD_PEPPER`.
- **Access tokens** — stateless RS256 JWTs. The header carries a `kid`; claims include `sub`, `iss`, `aud`, `scope`, `iat`, `exp`, and `jti`.
- **Refresh tokens** — 96-char random tokens, persisted **only** as a SHA-256 hex hash (never in plaintext). Rotation tracks a `family_id`; presenting an already-revoked token is treated as reuse and revokes the whole family.
- **OAuth 2.0 Authorization Server** — Authorization Code flow with **mandatory PKCE (S256)**; the implicit flow is rejected. Supported grants: `authorization_code`, `refresh_token`, and `client_credentials`.
- **OpenID Connect provider** — Discovery (`/.well-known/openid-configuration`), JWKS, and UserInfo endpoints. ID tokens are issued when the `openid` scope is present.
- **Signing-key rotation** — active RS256 keys can be served from the database (`signing_keys`), with a fallback to the configured PEM in settings.
- **Consent enforcement** — non-`trusted_first_party` clients require previously stored consent before scopes are granted.
- **Login lockout** — repeated failed logins can be rate-limited / locked out via a failure-counter table.
- **Social login** — federated identity callback that links external accounts to local users.
- **Account recovery** — email-verification and password-reset tokens, hashed the same way as refresh tokens.
- **Admin API** — client and user management endpoints (CRUD + client-secret rotation).

> Some of the features above are fully implemented and tested but not yet wired into the
> running binary. See [Wiring status & roadmap](#wiring-status--roadmap) for the exact
> state of each.

## Architecture

The codebase follows a layered, roughly Clean Architecture shape:

```
HTTP routes (src/routes/)        DTO validation + response shaping only
        │
        ▼
Domain logic (src/auth/, src/oauth/)   mostly pure functions (token, password, pkce, ...)
        │
        ▼
Repositories                     dual-backend: Postgres + InMemory
        │
        ▼
PostgreSQL
```

Three cross-cutting patterns are worth knowing before you read the source:

1. **Dual-backend repositories.** Every repository has a `Postgres(PgPool)` variant and an
   `InMemory(...)` variant. `main.rs` uses `::postgres(pool)`; tests use `::in_memory(...)`.
   This is what lets HTTP-handler tests run with **no database**.
2. **`Option<web::Data<Repo>>` graceful degradation.** Several handlers take their
   repositories as `Option<…>` and skip the corresponding behavior when the dependency is
   absent. A feature can therefore be fully implemented yet inert simply because `main.rs`
   doesn't register its repository.
3. **Two error conventions.** `/v1/auth/*` and account-recovery handlers use
   `AppError` (`src/error.rs`), which maps to a `{ "error", "message" }` JSON body and the
   right status code. OAuth and admin handlers return `HttpResponse` directly with their own
   OAuth-style error codes.

For a deeper write-up, see [`docs/architecture.md`](docs/architecture.md).

## Project layout

```
src/
  main.rs        Binary entrypoint: loads settings, connects the pool, wires routes
  lib.rs         Library surface (re-exports modules)
  app.rs         AppState { pool, settings } shared via web::Data
  config.rs      Settings::from_env (dotenvy-backed)
  db.rs          PgPool connection helper
  error.rs       AppError + Actix ResponseError mapping
  auth/          Password, JWT token, signing keys, login attempts, recovery, federation
  oauth/         OAuth client, PKCE, authorization codes, consent, refresh-token repo
  routes/        health, auth, oauth, oidc, admin handlers
migrations/      Timestamp-prefixed SQL migrations (apply with `sqlx migrate run`)
scripts/         coverage-unit.sh (domain-logic coverage gate)
tests/           ~37 integration-test files, one concern per file
docs/            Architecture, data model, API contract, security policy, plans
```

## Prerequisites

- **Rust** — a stable toolchain. `Cargo.toml` currently declares `edition = "2021"` and
  `rust-version = "1.78"`; CI runs the latest stable toolchain. If dependency resolution
  fails on an older local compiler, upgrade to current stable first.
- **Docker** (optional) — for the bundled PostgreSQL via `docker compose`.
- **`sqlx-cli`** (optional) — only needed to run migrations against a real database:
  `cargo install sqlx-cli --no-default-features --features rustls,postgres`.
- **`cargo-llvm-cov`** (optional) — only needed for the coverage gate.
- **OpenSSL** — to generate the RS256 signing key pair for local development.

> Most tests need **no database** and no key material, so you can clone, `cargo test`, and
> start reading immediately.

## Configuration

`Settings::from_env` (`src/config.rs`) loads configuration via `dotenvy`, so a `.env` file in
the project root is picked up automatically. Copy [`.env.example`](.env.example) to `.env` as
a starting point.

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `APP_ENV` | no | `local` | Deployment environment label. |
| `SERVER_HOST` | no | `127.0.0.1` | Bind address. |
| `SERVER_PORT` | no | `8080` | Bind port. |
| `DATABASE_URL` | **yes** | — | PostgreSQL connection string. |
| `JWT_ISSUER` | no | `atlas-auth` | `iss` claim / OIDC issuer. |
| `JWT_AUDIENCE` | no | `atlas-services` | `aud` claim. |
| `JWT_ACCESS_TOKEN_TTL_SECONDS` | no | `900` | Access-token lifetime (15 min). |
| `JWT_REFRESH_TOKEN_TTL_SECONDS` | no | `2592000` | Refresh-token lifetime (30 days). |
| `JWT_SIGNING_KEY_ID` | no | `local-dev-key` | `kid` for the settings-PEM fallback key. |
| `JWT_PRIVATE_KEY_PEM` | **yes** | — | RS256 private key, PEM. |
| `JWT_PUBLIC_KEY_PEM` | **yes** | — | RS256 public key, PEM. |
| `PASSWORD_PEPPER` | **yes** | — | Server-side secret mixed into password hashes. |

### PEM key format

PEM keys are stored **single-line** in `.env` with literal `\n` escapes; `config.rs`
un-escapes them at load time (`JWT_PRIVATE_KEY_PEM` and `JWT_PUBLIC_KEY_PEM`). Generate and
escape them like this:

```bash
openssl genrsa 2048 > jwt-private.pem
openssl rsa -in jwt-private.pem -pubout > jwt-public.pem

# Escape newlines into single-line .env values:
echo "JWT_PRIVATE_KEY_PEM=$(awk 'NF {sub(/\r/, ""); printf "%s\\n",$0;}' jwt-private.pem)" >> .env
echo "JWT_PUBLIC_KEY_PEM=$(awk 'NF {sub(/\r/, ""); printf "%s\\n",$0;}' jwt-public.pem)"  >> .env
```

In production, source these from a secret manager rather than committing them.

## Local development

```bash
# 1. Configuration
cp .env.example .env
# .env.example masks the password; docker-compose.yml uses atlas_auth / atlas_auth locally.
# Update DATABASE_URL before running migrations or the server:
perl -pi -e 's#^DATABASE_URL=.*#DATABASE_URL=postgres://atlas_auth:atlas_auth@localhost:5432/atlas_auth#' .env
# OIDC discovery builds endpoint URLs from JWT_ISSUER; for local smoke tests this should be a URL.
perl -pi -e 's#^JWT_ISSUER=.*#JWT_ISSUER=http://127.0.0.1:8080#' .env
# generate RS256 keys and append the escaped PEM values (see "PEM key format" above)
# set a real PASSWORD_PEPPER

# 2. Database (only needed to actually run the server, not for most tests)
docker compose up -d postgres     # Postgres 16 on :5432, user/pass/db = atlas_auth
sqlx migrate run                  # applies migrations/ (needs sqlx-cli + DATABASE_URL)

# 3. Build, test, run
cargo build
cargo test                        # the bulk run without a database
cargo run                         # starts the HTTP server on SERVER_HOST:SERVER_PORT
```

Smoke-test the running server:

```bash
curl http://127.0.0.1:8080/health/live      # {"status":"ok"}
curl http://127.0.0.1:8080/health/ready     # {"status":"ready"} once the DB is reachable

# Register a user (requires DB + migrations)
curl -X POST http://127.0.0.1:8080/v1/auth/password/register \
  -H 'content-type: application/json' \
  -d '{"email":"user@example.com","password":"correct horse battery staple"}'
```

## API reference

Full request/response shapes are in [`docs/api-contract.md`](docs/api-contract.md). The
endpoint surface:

### Health

| Method | Path | Description |
| --- | --- | --- |
| GET | `/health/live` | Liveness probe → `{"status":"ok"}`. |
| GET | `/health/ready` | Readiness probe; runs `SELECT 1` → `200` ready / `503` not ready. |

### Password auth (`/v1/auth`)

| Method | Path | Description |
| --- | --- | --- |
| POST | `/v1/auth/password/register` | Create a user; returns access + refresh tokens (`201`). |
| POST | `/v1/auth/password/login` | Authenticate; returns access + refresh tokens (`200`). |
| POST | `/v1/auth/token/refresh` | Rotate a refresh token; returns a new token pair (`200`). |
| POST | `/v1/auth/logout` | Revoke a refresh token (`204`). |
| POST | `/v1/auth/email/verify` | Consume an email-verification token. |
| POST | `/v1/auth/password/reset` | Consume a password-reset token. |
| GET | `/v1/auth/social/{provider}/callback` | Federated-login callback; links/creates the local account. |

### OAuth 2.0

| Method | Path | Description |
| --- | --- | --- |
| GET | `/oauth/authorize` | Authorization Code + PKCE (S256) entrypoint. |
| POST | `/oauth/token` | Token endpoint: `authorization_code`, `refresh_token`, `client_credentials`. |
| POST | `/oauth/revoke` | RFC 7009 token revocation. |
| POST | `/oauth/introspect` | RFC 7662 token introspection (confidential clients). |

### OpenID Connect

| Method | Path | Description |
| --- | --- | --- |
| GET | `/.well-known/openid-configuration` | OIDC Discovery document. |
| GET | `/.well-known/jwks.json` | Active RS256 public signing keys (JWKS). |
| GET | `/userinfo` | UserInfo, scoped by the access-token claims. |

### Admin (`/admin`) — not yet wired into the binary

| Method | Path | Description |
| --- | --- | --- |
| POST | `/admin/clients` | Create an OAuth client. |
| PUT | `/admin/clients/{client_id}` | Update a client. |
| DELETE | `/admin/clients/{client_id}` | Delete a client. |
| POST | `/admin/clients/{client_id}/secret/rotate` | Rotate the client secret. |
| POST | `/admin/users` | Create a user. |
| GET / PUT / DELETE | `/admin/users/{user_id}` | Read / update / delete a user. |

### Error shape (`AppError`)

```json
{ "error": "invalid_credentials", "message": "Invalid email or password" }
```

OAuth and admin handlers return OAuth-style `{ "error", "message" }` bodies with OAuth error
codes instead.

## Testing

Integration tests live in [`tests/`](tests/), one concern per file (~37 files). Most use
`actix_web::test::init_service` with in-memory repositories, so **they need no database**.

```bash
cargo test                              # everything
cargo test --lib                        # only in-source #[cfg(test)] unit tests
cargo test --test oauth_authorize       # one integration-test file (tests/oauth_authorize.rs)
cargo test refresh_token_rotation       # tests whose name matches a substring
```

Conventions:

- **HTTP tests** build the app with `App::new().configure(routes::X::configure)` and inject
  in-memory repositories via `.app_data(...)`.
- **`*_schema_migration.rs` tests assert on the SQL text** of migration files. Renaming a
  column or index means editing both the migration and its schema test.
- **`*_repository.rs` tests** exercise the in-memory repository variant; pure domain unit
  tests live in `#[cfg(test)]` modules inside `src/`.

## Lint, format & coverage

CI (`.github/workflows/ci.yml`) gates on formatting, Clippy, and tests:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

A separate coverage gate enforces a **90% line floor on pure domain logic only** (it ignores
`main`/`config`/`db`/`error`, all repositories, route handlers, and `authorization_code.rs`):

```bash
./scripts/coverage-unit.sh             # requires cargo-llvm-cov
```

## Docker

A multi-stage [`Dockerfile`](Dockerfile) builds a release binary and copies it, plus the
`migrations/`, into a slim Debian runtime image listening on `8080`:

```bash
docker build -t atlas-auth .
docker run --rm -p 8080:8080 --env-file .env atlas-auth
```

[`docker-compose.yml`](docker-compose.yml) provides a PostgreSQL 16 service for local
development (it does not run the app container).

## Wiring status & roadmap

Several features are implemented and tested but **not yet registered in `main.rs`**, so they
behave differently in the running binary than the test suite suggests. When wiring one up,
construct its repository in `main.rs`, `.app_data(...)` it, and `.configure(...)` any new
route group.

**Currently wired in `main.rs`:**

- Repositories (all Postgres): `OAuthClientRepository`, `AuthorizationCodeRepository`,
  `RefreshTokenRepository`.
- Routes: `health`, `auth`, `oauth`, `oidc`.

**Implemented + tested but not wired** — three distinct failure modes:

- *Silently degrades* (taken as `Option<web::Data<_>>`, so the handler runs without it):
  - `SigningKeyRepository` → JWKS/signing falls back to the settings PEM instead of DB rotation.
  - `ConsentRepository` → the consent check is skipped.
  - `LoginAttemptRepository` → login lockout is skipped.
- *Fails at request time* (taken as a **required** `web::Data<_>`, so the route 500s on the
  missing extractor):
  - `/v1/auth/email/verify` and `/v1/auth/password/reset` need `AccountRecoveryRepository`.
  - `/v1/auth/social/{provider}/callback` needs `FederatedIdentityRepository`.
- *Unreachable* — the whole `/admin/*` surface (and its in-memory `AdminUserRepository`) is
  never `.configure`d.

See [`docs/development-plan.md`](docs/development-plan.md) and
[`docs/plans/`](docs/plans/) for the broader roadmap.

## Further documentation

| Document | Contents |
| --- | --- |
| [`docs/architecture.md`](docs/architecture.md) | Layering and component overview. |
| [`docs/data-model.md`](docs/data-model.md) | Database schema and entities. |
| [`docs/api-contract.md`](docs/api-contract.md) | Full request/response shapes. |
| [`docs/security-policy.md`](docs/security-policy.md) | Token, password, and key-handling policy. |
| [`docs/development-plan.md`](docs/development-plan.md) | Phased delivery plan. |
| [`CLAUDE.md`](CLAUDE.md) | Guidance for working in this repo. |
