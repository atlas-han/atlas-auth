CREATE TABLE account_recovery_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    purpose TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (purpose IN ('email_verification', 'password_reset'))
);

CREATE INDEX idx_account_recovery_tokens_user_purpose
    ON account_recovery_tokens(user_id, purpose, created_at DESC);
CREATE INDEX idx_account_recovery_tokens_active_hash
    ON account_recovery_tokens(token_hash, purpose, expires_at)
    WHERE consumed_at IS NULL;
