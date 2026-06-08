# Security Policy

## Password

- Plain password 저장 금지
- Argon2id 사용
- password pepper는 secret manager에서 주입
- password 변경 시 기존 refresh token revoke

## Token

- Access token TTL: 기본 5~15분
- Refresh token TTL: 기본 30일
- Refresh token은 원문 저장 금지, SHA-256 hash만 저장
- Refresh token rotation 적용
- Reuse detection 발생 시 해당 user의 active refresh token revoke

## OAuth

- 모바일/SPA는 Authorization Code + PKCE 사용
- redirect URI allowlist 필수
- provider access token 장기 저장 금지
- external provider identity는 provider + provider_user_id로 unique 처리

## Browser Security

- 웹 cookie 사용 시 `HttpOnly`, `Secure`, `SameSite=Lax/Strict` 적용
- SPA token storage는 XSS risk를 고려해 별도 정책 문서화 필요

## Audit Events

필수 기록 이벤트:

- user_registered
- login_succeeded
- login_failed
- refresh_rotated
- refresh_reuse_detected
- logout
- provider_linked
- provider_unlinked
- password_changed
