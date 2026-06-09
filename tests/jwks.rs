use actix_web::{test, web, App};
use atlas_auth::{
    app::AppState,
    auth::signing_key_repository::{NewSigningKey, SigningKeyRepository},
    config::Settings,
    routes,
};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;

fn test_state_with_public_key(public_key_pem: String) -> AppState {
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
        jwt_private_key_pem: "unused-in-jwks-test".to_string(),
        jwt_public_key_pem: public_key_pem,
        password_pepper: "test-pepper".to_string(),
    };
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/atlas_auth_test")
        .expect("pool should be constructible lazily");
    AppState { pool, settings }
}

#[actix_rt::test]
async fn jwks_endpoint_exposes_public_rs256_key_without_private_material() {
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::RsaPrivateKey;

    let private_key =
        RsaPrivateKey::new(&mut rand_core::OsRng, 2048).expect("test key should generate");
    let public_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("public key should encode");
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state_with_public_key(public_pem)))
            .configure(routes::oidc::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/.well-known/jwks.json")
        .to_request();
    let resp = test::call_and_read_body_json::<_, _, Value>(&app, req).await;
    let key = &resp["keys"][0];

    assert_eq!(key["kid"], "test-key-1");
    assert_eq!(key["kty"], "RSA");
    assert_eq!(key["alg"], "RS256");
    assert_eq!(key["use"], "sig");
    assert!(key["n"].as_str().unwrap().len() > 300);
    assert_eq!(key["e"], "AQAB");
    assert!(
        key.get("d").is_none(),
        "JWKS must not expose private exponent"
    );
}

#[actix_rt::test]
async fn jwks_endpoint_uses_latest_active_signing_key_repository_when_configured() {
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::RsaPrivateKey;

    let retired_private_key =
        RsaPrivateKey::new(&mut rand_core::OsRng, 2048).expect("retired key should generate");
    let active_private_key =
        RsaPrivateKey::new(&mut rand_core::OsRng, 2048).expect("active key should generate");
    let active_public_pem = active_private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("active public key should encode");
    let signing_keys = SigningKeyRepository::in_memory();
    signing_keys
        .save(NewSigningKey {
            kid: "retired-key".to_string(),
            public_key: retired_private_key
                .to_public_key()
                .to_public_key_pem(LineEnding::LF)
                .expect("retired public key should encode"),
            private_key_ciphertext: "encrypted-retired-key".to_string(),
            algorithm: "RS256".to_string(),
            status: "retired".to_string(),
        })
        .await
        .unwrap();
    signing_keys
        .save(NewSigningKey {
            kid: "active-db-key".to_string(),
            public_key: active_public_pem,
            private_key_ciphertext: "encrypted-active-key".to_string(),
            algorithm: "RS256".to_string(),
            status: "active".to_string(),
        })
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state_with_public_key(
                "invalid-settings-key".to_string(),
            )))
            .app_data(web::Data::new(signing_keys))
            .configure(routes::oidc::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/.well-known/jwks.json")
        .to_request();
    let resp = test::call_and_read_body_json::<_, _, Value>(&app, req).await;
    let key = &resp["keys"][0];

    assert_eq!(key["kid"], "active-db-key");
    assert_eq!(key["alg"], "RS256");
    assert!(key.get("d").is_none());
}
