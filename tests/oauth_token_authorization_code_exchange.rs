use actix_web::{test, web, App};
use atlas_auth::{
    app::AppState,
    auth::token::IdTokenClaims,
    config::Settings,
    oauth::{
        authorization_code::{
            issue_authorization_code, AuthorizationCodeRepository, NewAuthorizationCode,
        },
        client_repository::{ClientRecord, OAuthClientRepository},
        pkce::s256_code_challenge,
        refresh_token_repository::RefreshTokenRepository,
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

fn client_record(client_uuid: Uuid) -> ClientRecord {
    ClientRecord {
        id: client_uuid,
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

#[actix_rt::test]
async fn token_exchanges_authorization_code_with_pkce_and_consumes_code() {
    let client_uuid = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let user_uuid = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let issued = issue_authorization_code();
    let plaintext_code = issued.plaintext.clone();
    let authorization_codes = AuthorizationCodeRepository::in_memory();
    authorization_codes
        .save(NewAuthorizationCode {
            id: Uuid::new_v4(),
            code_hash: issued.code_hash,
            client_id: client_uuid,
            user_id: user_uuid,
            redirect_uri: "https://app.example.test/callback".to_string(),
            code_challenge: s256_code_challenge(code_verifier),
            code_challenge_method: "S256".to_string(),
            scope: vec!["openid".to_string(), "email".to_string()],
            expires_at: issued.expires_at,
        })
        .await
        .unwrap();
    let clients = OAuthClientRepository::in_memory(vec![client_record(client_uuid)]);
    let refresh_tokens = RefreshTokenRepository::in_memory();
    let state = test_state();
    let public_key = state.settings.jwt_public_key_pem.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .app_data(web::Data::new(clients))
            .app_data(web::Data::new(authorization_codes.clone()))
            .app_data(web::Data::new(refresh_tokens.clone()))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "authorization_code"),
            ("client_id", "client-1"),
            ("redirect_uri", "https://app.example.test/callback"),
            ("code", plaintext_code.as_str()),
            ("code_verifier", code_verifier),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 900);
    assert_eq!(body["scope"], "openid email");
    assert!(body["access_token"].as_str().unwrap().len() > 100);
    let id_token = body["id_token"]
        .as_str()
        .expect("openid authorization_code exchange should issue an id_token");
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&["client-1"]);
    validation.set_issuer(&["https://auth.example.test"]);
    let decoded_id_token = decode::<IdTokenClaims>(
        id_token,
        &DecodingKey::from_rsa_pem(public_key.as_bytes()).expect("public key should parse"),
        &validation,
    )
    .expect("id_token should verify with public key");
    assert_eq!(decoded_id_token.claims.sub, user_uuid.to_string());
    assert_eq!(decoded_id_token.claims.aud, "client-1");
    assert!(!decoded_id_token.claims.jti.is_empty());
    let refresh_token = body["refresh_token"]
        .as_str()
        .expect("authorization_code exchange should issue refresh token");
    assert!(refresh_token.len() >= 96);

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "authorization_code"),
            ("client_id", "client-1"),
            ("redirect_uri", "https://app.example.test/callback"),
            ("code", plaintext_code.as_str()),
            ("code_verifier", code_verifier),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}
