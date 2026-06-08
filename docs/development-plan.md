# Atlas Auth Development Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Rust Actix-Web와 PostgreSQL 기반으로 여러 모바일 앱/웹 서비스가 공통 사용할 인증 서버를 구축한다.

**Architecture:** Clean Architecture에 가까운 계층형 구조를 사용한다. HTTP route는 DTO 검증과 응답 변환만 담당하고, 인증 도메인 로직은 service/domain 계층에 둔다. PostgreSQL은 user, identity, credential, refresh token, client, audit event의 source of truth가 된다.

**Tech Stack:** Rust, Actix-Web, PostgreSQL, SQLx, Argon2id, JWT, Docker, GitHub Actions.

---

## Phase 0 — Foundation

### Task 0.1: Repository bootstrap

**Objective:** Rust service 기본 구조, CI, Docker, docs 디렉터리를 만든다.

**Acceptance criteria**

- `cargo test` 실행 가능
- `.github/workflows/ci.yml`에 fmt/clippy/test 포함
- `docs/` 하위에 개발 문서 존재

### Task 0.2: Runtime configuration

**Objective:** 환경변수 기반 설정 로딩을 구현한다.

**Acceptance criteria**

- server host/port, database URL, JWT 설정, password pepper 설정 가능
- secret 값은 `.env.example`에 placeholder로만 제공

### Task 0.3: Health endpoints

**Objective:** liveness/readiness endpoint를 제공한다.

**Acceptance criteria**

- `GET /health/live`는 프로세스 생존 확인
- `GET /health/ready`는 PostgreSQL connectivity 확인

## Phase 1 — Password Authentication MVP

### Task 1.1: User and credential schema

**Objective:** 사용자와 password credential을 저장할 PostgreSQL schema를 만든다.

**Acceptance criteria**

- users table
- user_credentials table
- audit_events table
- email uniqueness 보장

### Task 1.2: Password hashing

**Objective:** Argon2id 기반 password hashing/verification을 구현한다.

**Acceptance criteria**

- plain password 저장 금지
- password verification test 존재
- wrong password는 실패

### Task 1.3: Register API

**Objective:** email/password 기반 회원가입 API를 구현한다.

**Acceptance criteria**

- `POST /v1/auth/password/register`
- email/password validation
- 중복 email 거부
- access/refresh token 발급

### Task 1.4: Login API

**Objective:** email/password 로그인 API를 구현한다.

**Acceptance criteria**

- `POST /v1/auth/password/login`
- password 검증
- 실패 시 민감 정보 없는 error 반환
- 성공 시 access/refresh token 발급

### Task 1.5: Refresh/logout API

**Objective:** refresh token rotation과 logout revoke를 구현한다.

**Acceptance criteria**

- `POST /v1/auth/token/refresh`
- `POST /v1/auth/logout`
- refresh token reuse detection
- logout 시 refresh token revoke

## Phase 2 — OAuth Social Login

### Task 2.1: OAuth client configuration

- Google/Facebook provider별 client id/secret/redirect URI 설정
- Authorization Code + PKCE flow 기준

### Task 2.2: Provider callback handling

- provider token exchange
- provider userinfo 조회
- `external_id` 기준 identity 연결

### Task 2.3: Account linking policy

- 같은 email의 provider account 처리 정책
- explicit linking 전 자동 병합 금지
- unlink 후 password/social fallback 정책

## Phase 3 — Client/Application Registry

- 모바일/웹 service별 client 등록
- redirect URI allowlist
- origin allowlist
- client status 관리
- token TTL policy override

## Phase 4 — Hardening & Operations

- rate limiting
- breached password check
- MFA extension point
- audit dashboard
- alerting
- key rotation
- backup/restore drill
- penetration test

## Milestone Definition

| Milestone | Scope | Done 기준 |
|---|---|---|
| M0 | Foundation | CI green, health endpoint, docs |
| M1 | Password Auth MVP | register/login/refresh/logout 통합 테스트 |
| M2 | Social Login | Google/Facebook callback 테스트 |
| M3 | Multi-client | 앱별 client policy 적용 |
| M4 | Production Hardening | rate limit, observability, security review |
