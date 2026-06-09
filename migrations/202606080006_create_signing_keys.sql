CREATE TABLE signing_keys (
    kid TEXT PRIMARY KEY,
    public_key TEXT NOT NULL,
    private_key_ciphertext TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    retired_at TIMESTAMPTZ NULL,
    CONSTRAINT chk_signing_keys_algorithm CHECK (algorithm IN ('RS256')),
    CONSTRAINT chk_signing_keys_status CHECK (status IN ('active', 'retired'))
);

CREATE INDEX idx_signing_keys_status_created_at ON signing_keys(status, created_at DESC);
