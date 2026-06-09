use actix_web::{test, web, App};
use atlas_auth::{
    app::AppState,
    auth::token::{hash_refresh_token, new_refresh_token},
    config::Settings,
    oauth::{
        client_repository::{ClientRecord, OAuthClientRepository},
        refresh_token_repository::{NewRefreshToken, RefreshTokenRepository},
    },
    routes,
};
use chrono::{Duration, Utc};
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
async fn refresh_token_grant_rotates_refresh_token_and_issues_access_token() {
    let client_uuid = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let user_uuid = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let family_uuid = Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap();
    let refresh_token = new_refresh_token();
    let refresh_token_hash = hash_refresh_token(&refresh_token);
    let refresh_tokens = RefreshTokenRepository::in_memory();
    refresh_tokens
        .save(NewRefreshToken {
            id: Uuid::new_v4(),
            user_id: user_uuid,
            client_id: client_uuid,
            token_hash: refresh_token_hash.clone(),
            family_id: family_uuid,
            scope: vec!["openid".to_string(), "email".to_string()],
            expires_at: Utc::now() + Duration::days(14),
        })
        .await
        .unwrap();
    let clients = OAuthClientRepository::in_memory(vec![client_record(client_uuid)]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state()))
            .app_data(web::Data::new(clients))
            .app_data(web::Data::new(refresh_tokens.clone()))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "refresh_token"),
            ("client_id", "client-1"),
            ("refresh_token", refresh_token.as_str()),
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
    let rotated_refresh_token = body["refresh_token"].as_str().unwrap();
    assert_ne!(rotated_refresh_token, refresh_token);

    let old_token = refresh_tokens
        .find_by_hash(&refresh_token_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(old_token.revoked);
    let new_token_hash = hash_refresh_token(rotated_refresh_token);
    let new_token = refresh_tokens
        .find_by_hash(&new_token_hash)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(new_token.family_id, family_uuid);
    assert_eq!(new_token.user_id, user_uuid);
    assert_eq!(new_token.client_id, client_uuid);
    assert_eq!(
        new_token.scope,
        vec!["openid".to_string(), "email".to_string()]
    );
    assert!(!new_token.revoked);
}

#[actix_rt::test]
async fn refresh_token_reuse_revokes_entire_token_family() {
    let client_uuid = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let user_uuid = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let family_uuid = Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap();
    let reused_refresh_token = new_refresh_token();
    let reused_refresh_token_hash = hash_refresh_token(&reused_refresh_token);
    let active_refresh_token = new_refresh_token();
    let active_refresh_token_hash = hash_refresh_token(&active_refresh_token);
    let refresh_tokens = RefreshTokenRepository::in_memory();
    refresh_tokens
        .save(NewRefreshToken {
            id: Uuid::new_v4(),
            user_id: user_uuid,
            client_id: client_uuid,
            token_hash: reused_refresh_token_hash.clone(),
            family_id: family_uuid,
            scope: vec!["openid".to_string(), "email".to_string()],
            expires_at: Utc::now() + Duration::days(14),
        })
        .await
        .unwrap();
    refresh_tokens
        .save(NewRefreshToken {
            id: Uuid::new_v4(),
            user_id: user_uuid,
            client_id: client_uuid,
            token_hash: active_refresh_token_hash.clone(),
            family_id: family_uuid,
            scope: vec!["openid".to_string(), "email".to_string()],
            expires_at: Utc::now() + Duration::days(14),
        })
        .await
        .unwrap();
    refresh_tokens
        .revoke_by_hash(&reused_refresh_token_hash)
        .await
        .unwrap();
    let clients = OAuthClientRepository::in_memory(vec![client_record(client_uuid)]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state()))
            .app_data(web::Data::new(clients))
            .app_data(web::Data::new(refresh_tokens.clone()))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "refresh_token"),
            ("client_id", "client-1"),
            ("refresh_token", reused_refresh_token.as_str()),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_grant");
    let reused_record = refresh_tokens
        .find_by_hash(&reused_refresh_token_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(reused_record.revoked);
    let active_record = refresh_tokens
        .find_by_hash(&active_refresh_token_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(active_record.revoked);
}
