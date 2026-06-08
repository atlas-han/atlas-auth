# Atlas Auth

Atlas Auth is a Rust Actix-Web + PostgreSQL authentication server for Atlas mobile apps and web services.

## Current status

Phase 0/1 foundation is implemented:

- Actix-Web HTTP server bootstrap
- PostgreSQL pool configuration
- health endpoints
- password registration/login/refresh/logout endpoints
- Argon2id password hashing
- RS256 JWT access token issuing with `kid`, `jti`, and `scope` claims
- refresh token rotation with reuse detection table model
- OIDC Discovery and JWKS endpoints
- initial PostgreSQL migrations
- development plan and architecture docs under `docs/`

## Local development

```bash
cp .env.example .env
openssl genrsa 2048 > jwt-private.pem
openssl rsa -in jwt-private.pem -pubout > jwt-public.pem
# Put escaped PEM values into .env or source them from your secret manager:
# JWT_PRIVATE_KEY_PEM=$(awk 'NF {sub(/\r/, ""); printf "%s\\n",$0;}' jwt-private.pem)
# JWT_PUBLIC_KEY_PEM=$(awk 'NF {sub(/\r/, ""); printf "%s\\n",$0;}' jwt-public.pem)
docker compose up -d postgres
sqlx migrate run
cargo test
cargo run
```

Health endpoints:

```bash
curl http://127.0.0.1:8080/health/live
curl http://127.0.0.1:8080/health/ready
```
