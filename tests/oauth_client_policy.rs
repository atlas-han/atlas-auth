use atlas_auth::oauth::client::{validate_authorize_client, OAuthClient};

fn public_client() -> OAuthClient {
    OAuthClient {
        public_client_id: "client-1".to_string(),
        client_secret_hash: None,
        client_type: "public".to_string(),
        allowed_redirect_uris: vec!["https://app.example.test/callback".to_string()],
        grant_types: vec!["authorization_code".to_string()],
        scopes: vec!["openid".to_string(), "email".to_string()],
        status: "active".to_string(),
    }
}

#[test]
fn authorize_client_accepts_exact_redirect_uri_and_allowed_scope() {
    let client = public_client();

    let result = validate_authorize_client(
        &client,
        "https://app.example.test/callback",
        &["openid".to_string(), "email".to_string()],
    );

    assert!(result.is_ok());
}

#[test]
fn authorize_client_rejects_partial_redirect_uri_match() {
    let client = public_client();

    let result = validate_authorize_client(
        &client,
        "https://app.example.test/callback/evil",
        &["openid".to_string()],
    );

    assert_eq!(
        result,
        Err("redirect_uri is not registered for this client")
    );
}

#[test]
fn authorize_client_rejects_unregistered_scope() {
    let client = public_client();

    let result = validate_authorize_client(
        &client,
        "https://app.example.test/callback",
        &["admin".to_string()],
    );

    assert_eq!(
        result,
        Err("Requested scope is not allowed for this client")
    );
}

#[test]
fn authorize_client_rejects_client_without_authorization_code_grant() {
    let mut client = public_client();
    client.grant_types = vec!["client_credentials".to_string()];

    let result = validate_authorize_client(
        &client,
        "https://app.example.test/callback",
        &["openid".to_string()],
    );

    assert_eq!(
        result,
        Err("Client is not allowed to use authorization_code grant")
    );
}
