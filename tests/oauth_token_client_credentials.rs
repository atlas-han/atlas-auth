use actix_web::{test, web, App};
use atlas_auth::{
    app::AppState,
    auth::token::Claims,
    config::Settings,
    oauth::{
        client::hash_client_secret,
        client_repository::{ClientRecord, OAuthClientRepository},
    },
    routes,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

fn test_state() -> AppState {
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::RsaPrivateKey;

    let private_key =
        RsaPrivateKey::new(&mut rand_core::OsRng, 2048).expect("test key should generate");
    let private_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .expect("private key should encode")
        .to_string();
    let public_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("public key should encode");
    let settings = Settings {
        app_env: "test".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        database_url: "postgres://localhost/atlas_auth_test".to_string(),
        jwt_issuer: "https://auth.example.test".to_string(),
        jwt_audience: "atlas-services".to_string(),
        jwt_access_token_ttl_seconds: 900,
        jwt_refresh_token_ttl_seconds: 2_592_000,
        jwt_signing_key_id: "test-key-1".to_string(),
        jwt_private_key_pem: private_pem,
        jwt_public_key_pem: public_pem,
        password_pepper: "test-pepper".to_string(),
    };
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/atlas_auth_test")
        .expect("pool should be constructible lazily");
    AppState { pool, settings }
}

fn confidential_client() -> ClientRecord {
    ClientRecord {
        id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        public_client_id: "service-client".to_string(),
        client_secret_hash: Some(hash_client_secret("correct-secret")),
        client_type: "confidential".to_string(),
        allowed_redirect_uris: vec!["https://service.example.test/callback".to_string()],
        grant_types: vec!["client_credentials".to_string()],
        scopes: vec!["jobs:run".to_string(), "jobs:read".to_string()],
        status: "active".to_string(),
        trusted_first_party: true,
        access_token_ttl_seconds: Some(900),
        refresh_token_ttl_seconds: None,
    }
}

#[actix_rt::test]
async fn token_issues_access_token_for_confidential_client_credentials_without_refresh_token() {
    let state = test_state();
    let public_key = state.settings.jwt_public_key_pem.clone();
    let clients = OAuthClientRepository::in_memory(vec![confidential_client()]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .app_data(web::Data::new(clients))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "client_credentials"),
            ("client_id", "service-client"),
            ("client_secret", "correct-secret"),
            ("scope", "jobs:read"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 900);
    assert_eq!(body["scope"], "jobs:read");
    assert!(body.get("refresh_token").is_none());

    let access_token = body["access_token"].as_str().expect("access token");
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&["atlas-services"]);
    validation.set_issuer(&["https://auth.example.test"]);
    let decoded = decode::<Claims>(
        access_token,
        &DecodingKey::from_rsa_pem(public_key.as_bytes()).expect("public key should parse"),
        &validation,
    )
    .expect("client credentials token should verify");

    assert_eq!(decoded.claims.sub, "service-client");
    assert_eq!(decoded.claims.scope, "jobs:read");
    assert!(!decoded.claims.jti.is_empty());
}

#[actix_rt::test]
async fn token_rejects_client_credentials_with_wrong_secret() {
    let clients = OAuthClientRepository::in_memory(vec![confidential_client()]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state()))
            .app_data(web::Data::new(clients))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "client_credentials"),
            ("client_id", "service-client"),
            ("client_secret", "wrong-secret"),
            ("scope", "jobs:read"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_client");
}
