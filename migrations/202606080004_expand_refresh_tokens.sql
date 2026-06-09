ALTER TABLE refresh_tokens
    ADD COLUMN client_id UUID NULL REFERENCES clients(id) ON DELETE CASCADE,
    ADD COLUMN scope TEXT[] NOT NULL DEFAULT '{}';

CREATE INDEX idx_refresh_tokens_client_id ON refresh_tokens(client_id);
CREATE INDEX idx_refresh_tokens_family_active
    ON refresh_tokens(family_id, client_id, expires_at)
    WHERE revoked_at IS NULL;
