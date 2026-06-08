# Atlas Auth

Atlas Auth is a Rust Actix-Web + PostgreSQL authentication server for Atlas mobile apps and web services.

## Current status

Phase 0/1 foundation is implemented:

- Actix-Web HTTP server bootstrap
- PostgreSQL pool configuration
- health endpoints
- password registration/login/refresh/logout endpoints
- Argon2id password hashing
- JWT access token issuing
- refresh token rotation with reuse detection table model
- initial PostgreSQL migrations
- development plan and architecture docs under `docs/`

## Local development

```bash
cp .env.example .env
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
