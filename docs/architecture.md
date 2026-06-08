# Architecture

## 목표

Atlas Auth는 여러 Atlas 모바일 앱과 웹 서비스가 공통 사용하는 중앙 인증 서버다.

## Architecture Style

```text
HTTP/API Layer
  └─ application service
       └─ domain/auth policy
            └─ infrastructure: PostgreSQL, JWT, password hashing, OAuth providers
```

## Runtime Components

```text
Client Apps/Web
   │
   ▼
Atlas Auth API Gateway / Load Balancer
   │
   ▼
Actix-Web Application
   ├─ Auth Routes
   ├─ Health Routes
   ├─ Token Service
   ├─ Password Service
   └─ PostgreSQL Repositories
        │
        ▼
PostgreSQL
```

## API Groups

- `/health/*`: runtime health
- `/v1/auth/password/*`: email/password authentication
- `/v1/auth/oauth/*`: Google/Facebook OAuth flow
- `/v1/auth/token/*`: refresh/revoke lifecycle
- `/v1/clients/*`: service client registry, future admin API

## Key Design Decisions

1. **Access token은 stateless JWT**로 발급한다.
2. **Refresh token은 PostgreSQL에 hash 저장**하고 rotation한다.
3. **Password hashing은 Argon2id**를 사용한다.
4. **Social login은 Authorization Code + PKCE**를 기준으로 한다.
5. **계정 병합은 explicit linking 없이 자동 수행하지 않는다.**
6. **audit event는 인증 주요 이벤트마다 기록한다.**
