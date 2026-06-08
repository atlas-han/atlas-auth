# API Contract

Base path: `/v1`

## OAuth/OIDC Discovery

### GET /.well-known/openid-configuration

Status: implemented.

Response `200`:

```json
{
  "issuer": "https://auth.example.com",
  "authorization_endpoint": "https://auth.example.com/oauth/authorize",
  "token_endpoint": "https://auth.example.com/oauth/token",
  "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post", "none"],
  "userinfo_endpoint": "https://auth.example.com/userinfo",
  "jwks_uri": "https://auth.example.com/.well-known/jwks.json",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code", "refresh_token", "client_credentials"],
  "subject_types_supported": ["public"],
  "id_token_signing_alg_values_supported": ["RS256"],
  "scopes_supported": ["openid", "profile", "email"],
  "code_challenge_methods_supported": ["S256"]
}
```

### GET /.well-known/jwks.json

Status: planned.

Publishes active RS256 public signing keys. Private key material must never be exposed.

## OAuth 2.0 Authorization Server Endpoints

### GET /oauth/authorize

Status: planned.

Authorization Code + PKCE entrypoint. Supports `response_type=code`, `client_id`, exact-match `redirect_uri`, `scope`, `state`, `code_challenge`, and `code_challenge_method=S256`.

### POST /oauth/token

Status: planned.

Supports:
- `grant_type=authorization_code`
- `grant_type=refresh_token`
- `grant_type=client_credentials`

### POST /oauth/revoke

Status: planned. RFC 7009 token revocation.

### POST /oauth/introspect

Status: planned. RFC 7662 token introspection for confidential clients.

### GET /userinfo

Status: planned. OIDC user info endpoint scoped by access token claims.

## Health

### GET /health/live

```json
{ "status": "ok" }
```

### GET /health/ready

```json
{ "status": "ready" }
```

## Password Register

### POST /v1/auth/password/register

Request:

```json
{
  "email": "user@example.com",
  "password": "correct horse battery staple"
}
```

Response `201`:

```json
{
  "user_id": "uuid",
  "access_token": "jwt",
  "refresh_token": "opaque-token",
  "token_type": "Bearer",
  "expires_in": 900
}
```

## Password Login

### POST /v1/auth/password/login

Request:

```json
{
  "email": "user@example.com",
  "password": "correct horse battery staple"
}
```

Response `200`: same as register.

## Refresh

### POST /v1/auth/token/refresh

Request:

```json
{ "refresh_token": "opaque-token" }
```

Response `200`: new access token and refresh token.

## Logout

### POST /v1/auth/logout

Request:

```json
{ "refresh_token": "opaque-token" }
```

Response `204`.

## Error Shape

```json
{
  "error": "invalid_credentials",
  "message": "Invalid email or password"
}
```
