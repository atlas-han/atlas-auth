# Data Model

## Core Tables

```text
users
- id uuid pk
- email citext unique
- email_verified_at timestamptz null
- status text
- created_at timestamptz
- updated_at timestamptz

user_credentials
- user_id uuid pk/fk users.id
- password_hash text
- password_changed_at timestamptz
- created_at timestamptz
- updated_at timestamptz

user_identities
- id uuid pk
- user_id uuid fk users.id
- provider text
- provider_user_id text
- email text null
- created_at timestamptz
- unique(provider, provider_user_id)

refresh_tokens
- id uuid pk
- user_id uuid fk users.id
- token_hash text unique
- family_id uuid
- expires_at timestamptz
- revoked_at timestamptz null
- replaced_by uuid null
- created_at timestamptz

clients
- id uuid pk
- public_client_id text unique
- name text
- client_type text (`confidential` | `public`)
- client_secret_hash text null
- allowed_redirect_uris text[]
- allowed_origins text[]
- grant_types text[]
- scopes text[]
- access_token_ttl_seconds integer null
- refresh_token_ttl_seconds integer null
- trusted_first_party boolean
- status text
- created_at timestamptz
- updated_at timestamptz

audit_events
- id uuid pk
- user_id uuid null
- event_type text
- ip_address inet null
- user_agent text null
- metadata jsonb
- created_at timestamptz
```

## Notes

- `citext` extension으로 email case-insensitive uniqueness를 보장한다.
- refresh token 원문은 저장하지 않는다.
- client secret은 `client_secret_hash`에만 저장하고 public client는 secret을 가질 수 없다.
- client별 허용 grant/scope 및 token TTL override를 `clients`에서 관리한다.
- social identity는 provider별 immutable external id 기준으로 연결한다.
