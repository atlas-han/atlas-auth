ALTER TABLE clients
    ADD COLUMN client_secret_hash TEXT NULL,
    ADD COLUMN grant_types TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN scopes TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN access_token_ttl_seconds INTEGER NULL,
    ADD COLUMN refresh_token_ttl_seconds INTEGER NULL,
    ADD COLUMN trusted_first_party BOOLEAN NOT NULL DEFAULT false,
    ADD CONSTRAINT chk_clients_type CHECK (client_type IN ('confidential', 'public')),
    ADD CONSTRAINT chk_clients_redirect_uris_not_empty CHECK (cardinality(allowed_redirect_uris) > 0),
    ADD CONSTRAINT chk_clients_access_token_ttl_positive CHECK (
        access_token_ttl_seconds IS NULL OR access_token_ttl_seconds > 0
    ),
    ADD CONSTRAINT chk_clients_refresh_token_ttl_positive CHECK (
        refresh_token_ttl_seconds IS NULL OR refresh_token_ttl_seconds > 0
    ),
    ADD CONSTRAINT chk_clients_public_has_no_secret CHECK (
        client_type <> 'public' OR client_secret_hash IS NULL
    );

UPDATE clients
SET grant_types = ARRAY['authorization_code', 'refresh_token'],
    scopes = ARRAY['openid', 'profile', 'email']
WHERE cardinality(grant_types) = 0;

CREATE INDEX idx_clients_status ON clients(status);
CREATE INDEX idx_clients_grant_types ON clients USING GIN(grant_types);
CREATE INDEX idx_clients_scopes ON clients USING GIN(scopes);
