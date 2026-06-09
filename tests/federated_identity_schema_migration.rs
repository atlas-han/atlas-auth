const FEDERATED_IDENTITY_SCHEMA_MIGRATION: &str =
    include_str!("../migrations/202606080007_create_federated_identities.sql");

#[test]
fn federated_identity_schema_migration_adds_prd_social_login_table() {
    for required_fragment in [
        "CREATE TABLE federated_identities",
        "id UUID PRIMARY KEY",
        "user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE",
        "provider TEXT NOT NULL",
        "provider_user_id TEXT NOT NULL",
        "email CITEXT NULL",
        "profile JSONB NOT NULL DEFAULT '{}'",
        "created_at TIMESTAMPTZ NOT NULL DEFAULT now()",
        "UNIQUE(provider, provider_user_id)",
        "CHECK (provider IN ('google', 'facebook'))",
        "idx_federated_identities_user_id",
    ] {
        assert!(
            FEDERATED_IDENTITY_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
