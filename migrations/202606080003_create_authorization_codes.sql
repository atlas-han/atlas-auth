CREATE TABLE authorization_codes (
    id UUID PRIMARY KEY,
    code_hash TEXT NOT NULL UNIQUE,
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    code_challenge_method TEXT NOT NULL,
    scope TEXT[] NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (code_challenge_method = 'S256'),
    CHECK (expires_at > created_at)
);

CREATE INDEX idx_authorization_codes_client_user
    ON authorization_codes(client_id, user_id);

CREATE INDEX idx_authorization_codes_expires_at
    ON authorization_codes(expires_at);

CREATE INDEX idx_authorization_codes_unconsumed
    ON authorization_codes(code_hash, expires_at)
    WHERE consumed_at IS NULL;
