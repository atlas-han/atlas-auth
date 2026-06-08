const CLIENT_SCHEMA_MIGRATION: &str = include_str!("../migrations/202606080002_expand_clients.sql");

#[test]
fn client_schema_migration_adds_prd_client_policy_fields() {
    for required_fragment in [
        "client_secret_hash TEXT NULL",
        "grant_types TEXT[] NOT NULL DEFAULT '{}'",
        "scopes TEXT[] NOT NULL DEFAULT '{}'",
        "access_token_ttl_seconds INTEGER NULL",
        "refresh_token_ttl_seconds INTEGER NULL",
        "trusted_first_party BOOLEAN NOT NULL DEFAULT false",
        "CHECK (client_type IN ('confidential', 'public'))",
        "CHECK (cardinality(allowed_redirect_uris) > 0)",
        "idx_clients_status",
        "idx_clients_grant_types",
        "idx_clients_scopes",
    ] {
        assert!(
            CLIENT_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
