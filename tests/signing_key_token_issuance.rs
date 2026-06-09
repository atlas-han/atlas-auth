use actix_web::{test, web, App};
use atlas_auth::{
    app::AppState,
    auth::{
        signing_key_repository::{NewSigningKey, SigningKeyRepository},
        token::Claims,
    },
    config::Settings,
    oauth::client::hash_client_secret,
    oauth::client_repository::{ClientRecord, OAuthClientRepository},
    routes,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

fn test_key_pair() -> (String, String) {
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::RsaPrivateKey;

    let private_key = RsaPrivateKey::new(&mut rand_core::OsRng, 2048).unwrap();
    let private_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .unwrap()
        .to_string();
    let public_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    (private_pem, public_pem)
}

fn test_state() -> AppState {
    let (private_pem, public_pem) = test_key_pair();
    let settings = Settings {
        app_env: "test".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        database_url: "postgres://localhost/atlas_auth_test".to_string(),
        jwt_issuer: "https://auth.example.test".to_string(),
        jwt_audience: "atlas-services".to_string(),
        jwt_access_token_ttl_seconds: 900,
        jwt_refresh_token_ttl_seconds: 2_592_000,
        jwt_signing_key_id: "settings-key".to_string(),
        jwt_private_key_pem: private_pem,
        jwt_public_key_pem: public_pem,
        password_pepper: "test-pepper".to_string(),
    };
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/atlas_auth_test")
        .unwrap();
    AppState { pool, settings }
}

fn confidential_client() -> ClientRecord {
    ClientRecord {
        id: Uuid::new_v4(),
        public_client_id: "service-1".to_string(),
        client_secret_hash: Some(hash_client_secret("secret-1")),
        client_type: "confidential".to_string(),
        allowed_redirect_uris: vec![],
        grant_types: vec!["client_credentials".to_string()],
        scopes: vec!["metrics.read".to_string()],
        status: "active".to_string(),
        trusted_first_party: true,
        access_token_ttl_seconds: Some(900),
        refresh_token_ttl_seconds: None,
    }
}

#[actix_rt::test]
async fn token_endpoint_signs_access_tokens_with_latest_active_repository_key() {
    let (old_private, old_public) = test_key_pair();
    let (active_private, active_public) = test_key_pair();
    let signing_keys = SigningKeyRepository::in_memory();
    signing_keys
        .save(NewSigningKey {
            kid: "old-key".to_string(),
            public_key: old_public,
            private_key_ciphertext: old_private,
            algorithm: "RS256".to_string(),
            status: "retired".to_string(),
        })
        .await
        .unwrap();
    signing_keys
        .save(NewSigningKey {
            kid: "active-db-key".to_string(),
            public_key: active_public.clone(),
            private_key_ciphertext: active_private,
            algorithm: "RS256".to_string(),
            status: "active".to_string(),
        })
        .await
        .unwrap();

    let clients = OAuthClientRepository::in_memory(vec![confidential_client()]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state()))
            .app_data(web::Data::new(clients))
            .app_data(web::Data::new(signing_keys))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "client_credentials"),
            ("client_id", "service-1"),
            ("client_secret", "secret-1"),
            ("scope", "metrics.read"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    let access_token = body["access_token"].as_str().unwrap();
    let header = decode_header(access_token).unwrap();
    assert_eq!(header.kid.as_deref(), Some("active-db-key"));

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&["atlas-services"]);
    validation.set_issuer(&["https://auth.example.test"]);
    let decoded = decode::<Claims>(
        access_token,
        &DecodingKey::from_rsa_pem(active_public.as_bytes()).unwrap(),
        &validation,
    )
    .expect("repository-signed token should verify with active public key");
    assert_eq!(decoded.claims.sub, "service-1");
    assert_eq!(decoded.claims.scope, "metrics.read");
}
