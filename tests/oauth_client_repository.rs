use atlas_auth::oauth::client::OAuthClient;
use atlas_auth::oauth::client_repository::{client_by_public_id_sql, ClientRecord};
use uuid::Uuid;

fn client_record() -> ClientRecord {
    ClientRecord {
        id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        public_client_id: "client-1".to_string(),
        client_secret_hash: None,
        client_type: "public".to_string(),
        allowed_redirect_uris: vec!["https://app.example.test/callback".to_string()],
        grant_types: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        scopes: vec!["openid".to_string(), "email".to_string()],
        status: "active".to_string(),
        trusted_first_party: false,
        access_token_ttl_seconds: Some(900),
        refresh_token_ttl_seconds: Some(1_209_600),
    }
}

#[test]
fn client_record_maps_to_oauth_policy_model_without_losing_prd_fields() {
    let record = client_record();

    let policy: OAuthClient = record.clone().into();

    assert_eq!(policy.public_client_id, "client-1");
    assert_eq!(policy.client_type, "public");
    assert_eq!(
        policy.allowed_redirect_uris,
        vec!["https://app.example.test/callback".to_string()]
    );
    assert_eq!(
        policy.grant_types,
        vec![
            "authorization_code".to_string(),
            "refresh_token".to_string()
        ]
    );
    assert_eq!(
        policy.scopes,
        vec!["openid".to_string(), "email".to_string()]
    );
    assert_eq!(policy.status, "active");
}

#[test]
fn client_lookup_sql_reads_only_active_clients_by_public_client_id() {
    let sql = client_by_public_id_sql();

    assert!(sql.contains("FROM clients"));
    assert!(sql.contains("public_client_id = $1"));
    assert!(sql.contains("status = 'active'"));
    assert!(sql.contains("grant_types"));
    assert!(sql.contains("scopes"));
    assert!(sql.contains("allowed_redirect_uris"));
    assert!(sql.contains("trusted_first_party"));
}
