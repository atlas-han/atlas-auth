CREATE TABLE federated_identities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_user_id TEXT NOT NULL,
    email CITEXT NULL,
    profile JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_federated_identities_provider CHECK (provider IN ('google', 'facebook')),
    UNIQUE(provider, provider_user_id)
);

CREATE INDEX idx_federated_identities_user_id ON federated_identities(user_id);
