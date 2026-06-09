use std::fs;

#[test]
fn refresh_token_schema_tracks_client_scope_and_rotation_family() {
    let migration = fs::read_to_string("migrations/202606080004_expand_refresh_tokens.sql")
        .expect("refresh token expansion migration should exist");

    assert!(migration.contains("ADD COLUMN client_id UUID"));
    assert!(migration.contains("REFERENCES clients(id)"));
    assert!(migration.contains("ADD COLUMN scope TEXT[]"));
    assert!(migration.contains("idx_refresh_tokens_client_id"));
    assert!(migration.contains("idx_refresh_tokens_family_active"));
    assert!(migration.contains("revoked_at IS NULL"));
}
