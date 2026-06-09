use actix_web::{http::header, test, web, App};
use atlas_auth::{app::AppState, auth::token::issue_access_token, config::Settings, routes};
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

#[actix_rt::test]
async fn userinfo_returns_subject_for_valid_bearer_access_token() {
    let state = test_state();
    let user_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let (access_token, _) = issue_access_token(&state.settings, user_id, "openid profile email")
        .expect("access token should issue");
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(routes::oidc::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/userinfo")
        .insert_header((header::AUTHORIZATION, format!("Bearer {access_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["sub"], user_id.to_string());
    assert_eq!(body["scope"], "openid profile email");
}

#[actix_rt::test]
async fn userinfo_rejects_missing_bearer_token() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state()))
            .configure(routes::oidc::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/userinfo").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}
