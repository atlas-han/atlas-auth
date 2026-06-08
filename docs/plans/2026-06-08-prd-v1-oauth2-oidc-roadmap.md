# Atlas-Auth PRD v1.0 OAuth2/OIDC Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** PRD v1.0 요구사항에 맞춰 Atlas-Auth를 OAuth 2.0/OAuth 2.1 best practice 기반 통합 Authorization Server로 발전시킨다.

**Architecture:** Actix-Web route는 protocol DTO/HTTP 변환만 담당하고, OAuth/OIDC/security 로직은 `auth`, `oauth`, `clients`, `keys` 모듈로 분리한다. PostgreSQL은 user/client/authorization code/refresh token/consent/key/audit source of truth가 되며, public token verification은 RS256 JWT + JWKS로 처리한다.

**Tech Stack:** Rust, Actix-Web, PostgreSQL, SQLx, Argon2id, jsonwebtoken/josekit, OpenTelemetry, Docker/Kubernetes.

---

## Current Baseline

Implemented:
- Actix-Web bootstrap, health endpoints, DB pool
- Password register/login/logout
- Argon2id password hashing with pepper
- Refresh token rotation/reuse-detection table model
- Audit event table and basic event writes
- Initial clients table
- `/.well-known/openid-configuration` discovery endpoint
- RS256 access token signing with `kid`, `jti`, and `scope` claims
- `/.well-known/jwks.json` JWKS endpoint for active public signing key
- PRD-aligned client policy schema fields for grant/scope/TTL/secret handling
- Authorization code schema for hashed one-time PKCE `S256` code exchange
- `/oauth/authorize` protocol-level validation rejects implicit flow and missing/non-S256 PKCE
- `/oauth/token` protocol-level validation rejects password grant and checks authorization-code request shape

Major PRD gaps:
- `/oauth/token` DB-backed code exchange, refresh rotation delegation, and client authentication
- `/oauth/revoke`, `/oauth/introspect`, `/userinfo`
- `/oauth/authorize` client lookup, login/consent UI, and code issuance
- Authorization Code + PKCE model and one-time code exchange
- OIDC `id_token`
- Client Credentials grant
- Consent model/flow
- Google/Facebook identity federation
- Admin clients/users APIs
- Rate limiting/account lockout/email verification/password reset
- Observability/security hardening

## Milestone 0 — Protocol Surface and Planning Guardrails

### Task 0.1: Add OIDC Discovery endpoint

**Objective:** Expose PRD-required discovery metadata for clients/resource servers.

**Files:**
- Create: `src/routes/oidc.rs`
- Modify: `src/routes/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/oidc_discovery.rs`

**Acceptance criteria:**
- `GET /.well-known/openid-configuration` returns issuer, authorize/token/userinfo/JWKS URIs
- Advertises only secure flows: `authorization_code`, `refresh_token`, `client_credentials`
- Advertises PKCE `S256` and RS256 ID token signing

**Verification:**
```bash
PATH=/opt/data/.cargo/bin:$PATH RUSTUP_HOME=/opt/data/.rustup CARGO_HOME=/opt/data/.cargo cargo +stable test openid_configuration_advertises_prd_required_endpoints_and_pkce
```

### Task 0.2: Update API contract with OIDC/OAuth target endpoints

**Objective:** Make the target API surface explicit before implementation.

**Files:**
- Modify: `docs/api-contract.md`

**Acceptance criteria:**
- Documents discovery, JWKS, authorize, token, revoke, introspect, userinfo
- Marks current vs planned endpoints clearly

## Milestone 1 — RS256 JWT and JWKS

### Task 1.1: Introduce signing key configuration

**Objective:** Replace HS secret assumptions with key-id based RS256 signing config.

**Files:**
- Modify: `src/config.rs`
- Modify: `.env.example`
- Test: `tests/config.rs` or unit tests in `src/config.rs`

**Acceptance criteria:**
- `JWT_SIGNING_KEY_ID` is required/defaultable for local dev
- Private/public key PEM paths or inline PEM envs are supported
- Test config can construct deterministic settings without secrets in repo

### Task 1.2: Implement RS256 access-token issuance

**Objective:** Issue PRD-compliant access tokens.

**Files:**
- Modify: `src/auth/token.rs`
- Test: unit tests in `src/auth/token.rs`

**Acceptance criteria:**
- Header includes `alg=RS256` and `kid`
- Claims include `sub`, `iss`, `aud`, `scope`, `exp`, `iat`, `jti`
- Token TTL remains configurable
- Tests decode header/claims and verify with public key

### Task 1.3: Add JWKS endpoint

**Objective:** Publish active public signing keys for resource-server local verification.

**Files:**
- Modify: `src/routes/oidc.rs` or create `src/routes/jwks.rs`
- Test: `tests/jwks.rs`

**Acceptance criteria:**
- `GET /.well-known/jwks.json` returns RFC 7517 `keys[]`
- Each key includes `kid`, `kty=RSA`, `alg=RS256`, `use=sig`, `n`, `e`
- Private key material is never exposed

## Milestone 2 — OAuth Client Registry and PKCE Code Flow

### Task 2.1: Expand clients schema

**Objective:** Align `clients` table with PRD client policy.

**Files:**
- Create: new migration under `migrations/`
- Test: migration smoke test if DB test harness is available

**Acceptance criteria:**
- `client_secret_hash nullable`, `grant_types[]`, `scopes[]`, access/refresh TTL overrides
- Redirect URI exact-match indexes/constraints where practical
- No plaintext client secret storage

### Task 2.2: Add authorization code schema

**Objective:** Store one-time short-lived PKCE authorization codes.

**Files:**
- Create: migration under `migrations/`

**Acceptance criteria:**
- `authorization_codes` has code hash, client/user/redirect/scope/challenge fields
- 60s expiry and consumed marker
- Indexes for lookup and cleanup

### Task 2.3: Implement `/oauth/authorize` request validation

**Objective:** Validate client, redirect URI, response type, scope, `state`, and PKCE challenge.

**Files:**
- Create: `src/oauth/authorize.rs`
- Modify: routes registration
- Test: `tests/oauth_authorize.rs`

**Acceptance criteria:**
- Rejects missing/invalid PKCE for public clients
- Rejects non-exact redirect URI
- Rejects implicit/password response types
- Returns OAuth-compliant error redirects where safe

### Task 2.4: Implement `/oauth/token` authorization_code exchange

**Objective:** Exchange one-time code + verifier for access/refresh/id tokens.

**Files:**
- Create: `src/oauth/token.rs`
- Test: `tests/oauth_token.rs`

**Acceptance criteria:**
- Verifies `BASE64URL(SHA256(code_verifier)) == code_challenge`
- Consumes code exactly once
- Issues access token, refresh token, and OIDC `id_token` for `openid` scope

## Milestone 3 — Refresh, Revoke, Introspection, Client Credentials

### Task 3.1: Move existing refresh endpoint to `/oauth/token` refresh grant

**Objective:** Make refresh token rotation RFC-compatible while preserving existing logic.

**Acceptance criteria:**
- `grant_type=refresh_token` supported at `/oauth/token`
- Refresh reuse revokes token family
- Legacy `/v1/auth/token/refresh` is either deprecated or internally delegates

### Task 3.2: Implement RFC 7009 `/oauth/revoke`

**Acceptance criteria:**
- Revokes refresh tokens by hash
- Authenticates confidential clients
- Always returns 200/204 per RFC semantics without leaking token existence

### Task 3.3: Implement RFC 7662 `/oauth/introspect`

**Acceptance criteria:**
- Confidential client auth required
- Opaque refresh tokens return active metadata
- Unknown/expired/revoked tokens return `{ "active": false }`

### Task 3.4: Implement Client Credentials grant

**Acceptance criteria:**
- Confidential clients only
- Validates secret hash and allowed scopes
- Issues access token with client subject/audience/scope

## Milestone 4 — User Experience and Federation

### Task 4.1: Email verification and password reset

**Acceptance criteria:**
- One-time hashed verification/reset tokens
- Expiry and audit events
- No account enumeration in responses

### Task 4.2: Login protection

**Acceptance criteria:**
- Rate limit by account/IP
- Account lockout policy
- Audit events include reason metadata

### Task 4.3: Google OIDC federation

**Acceptance criteria:**
- Verifies Google `id_token` signature/`aud`/`iss`/`exp`
- Links by `(provider, provider_user_id)`; email match requires explicit safe policy

### Task 4.4: Facebook OAuth federation

**Acceptance criteria:**
- Exchanges code for provider access token
- Fetches `/me` profile
- Stores provider identity without storing unnecessary provider token material

### Task 4.5: Consent and userinfo

**Acceptance criteria:**
- Stores user consent by client/scope
- Trusted first-party bypass is explicit in client policy
- `/userinfo` returns claims allowed by token scopes

## Milestone 5 — Admin APIs and Operations

### Task 5.1: Admin clients API

**Acceptance criteria:**
- CRUD clients
- Secret issuance/rotation returns plaintext secret only once
- Scope/grant/redirect validation

### Task 5.2: Admin users API

**Acceptance criteria:**
- Search/read/update user status
- No password hash/secret exposure
- Audit logs for admin changes

### Task 5.3: Observability

**Acceptance criteria:**
- JSON structured logs
- OpenTelemetry traces
- Prometheus metrics: token issuance rate, error rate, latency histograms

### Task 5.4: Security readiness

**Acceptance criteria:**
- OAuth Security BCP/RFC 9700 checklist documented
- OWASP ASVS checklist documented
- Load test validates token issuance P99 target or records bottlenecks

## Execution Rules

- Use strict TDD for every behavior change: RED → GREEN → REFACTOR.
- Prefer small PRs/commits per milestone task.
- Do not implement implicit grant or password grant.
- Do not store plaintext password, client secret, refresh token, provider token, or private signing key.
- Exact redirect URI matching only; no prefix/wildcard matching.
- Keep docs and API contract updated in the same task as behavior changes.
