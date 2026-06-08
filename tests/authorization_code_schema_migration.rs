const AUTHORIZATION_CODE_SCHEMA_MIGRATION: &str =
    include_str!("../migrations/202606080003_create_authorization_codes.sql");

#[test]
fn authorization_code_schema_supports_pkce_and_one_time_exchange() {
    for required_fragment in [
        "CREATE TABLE authorization_codes",
        "code_hash TEXT NOT NULL UNIQUE",
        "client_id UUID NOT NULL REFERENCES clients(id) ON DELETE CASCADE",
        "user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE",
        "redirect_uri TEXT NOT NULL",
        "code_challenge TEXT NOT NULL",
        "code_challenge_method TEXT NOT NULL",
        "scope TEXT[] NOT NULL DEFAULT '{}'",
        "expires_at TIMESTAMPTZ NOT NULL",
        "consumed_at TIMESTAMPTZ NULL",
        "CHECK (code_challenge_method = 'S256')",
        "idx_authorization_codes_client_user",
        "idx_authorization_codes_expires_at",
        "idx_authorization_codes_unconsumed",
    ] {
        assert!(
            AUTHORIZATION_CODE_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
