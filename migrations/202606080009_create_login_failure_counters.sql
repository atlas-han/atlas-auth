CREATE TABLE login_failure_counters (
    subject TEXT PRIMARY KEY,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ NULL,
    last_failed_at TIMESTAMPTZ NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_login_failure_counters_locked_until
    ON login_failure_counters(locked_until)
    WHERE locked_until IS NOT NULL;
