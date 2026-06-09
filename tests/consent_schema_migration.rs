const CONSENT_SCHEMA_MIGRATION: &str =
    include_str!("../migrations/202606080005_create_consents.sql");

#[test]
fn consent_schema_migration_adds_prd_consent_table() {
    for required_fragment in [
        "CREATE TABLE consents",
        "id UUID PRIMARY KEY",
        "user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE",
        "client_id UUID NOT NULL REFERENCES clients(id) ON DELETE CASCADE",
        "scopes TEXT[] NOT NULL",
        "granted_at TIMESTAMPTZ NOT NULL DEFAULT now()",
        "UNIQUE(user_id, client_id)",
        "idx_consents_user_client",
    ] {
        assert!(
            CONSENT_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
