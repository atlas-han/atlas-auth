const SIGNING_KEY_SCHEMA_MIGRATION: &str =
    include_str!("../migrations/202606080006_create_signing_keys.sql");

#[test]
fn signing_key_schema_migration_adds_prd_key_rotation_table() {
    for required_fragment in [
        "CREATE TABLE signing_keys",
        "kid TEXT PRIMARY KEY",
        "public_key TEXT NOT NULL",
        "private_key_ciphertext TEXT NOT NULL",
        "algorithm TEXT NOT NULL",
        "status TEXT NOT NULL",
        "created_at TIMESTAMPTZ NOT NULL DEFAULT now()",
        "retired_at TIMESTAMPTZ NULL",
        "CHECK (algorithm IN ('RS256'))",
        "CHECK (status IN ('active', 'retired'))",
        "idx_signing_keys_status_created_at",
    ] {
        assert!(
            SIGNING_KEY_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
