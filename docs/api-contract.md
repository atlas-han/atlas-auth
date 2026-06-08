# API Contract

Base path: `/v1`

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
