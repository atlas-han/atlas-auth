const ACCOUNT_RECOVERY_SCHEMA_MIGRATION: &str =
    include_str!("../migrations/202606080008_create_account_recovery_tokens.sql");

#[test]
fn account_recovery_schema_migration_adds_email_verification_and_password_reset_tokens() {
    for required_fragment in [
        "CREATE TABLE account_recovery_tokens",
        "id UUID PRIMARY KEY",
        "user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE",
        "token_hash TEXT NOT NULL UNIQUE",
        "purpose TEXT NOT NULL",
        "expires_at TIMESTAMPTZ NOT NULL",
        "consumed_at TIMESTAMPTZ NULL",
        "CHECK (purpose IN ('email_verification', 'password_reset'))",
        "idx_account_recovery_tokens_user_purpose",
        "idx_account_recovery_tokens_active_hash",
    ] {
        assert!(
            ACCOUNT_RECOVERY_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
