use actix_web::{test, web, App};
use atlas_auth::{app::AppState, config::Settings, routes};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;

fn test_state() -> AppState {
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
        jwt_private_key_pem: "unused-in-discovery-test".to_string(),
        jwt_public_key_pem: "unused-in-discovery-test".to_string(),
        password_pepper: "test-pepper".to_string(),
    };
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/atlas_auth_test")
        .expect("pool should be constructible lazily");
    AppState { pool, settings }
}

#[actix_rt::test]
async fn openid_configuration_advertises_prd_required_endpoints_and_pkce() {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(test_state()))
            .configure(routes::oidc::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/.well-known/openid-configuration")
        .to_request();
    let resp = test::call_and_read_body_json::<_, _, Value>(&app, req).await;

    assert_eq!(resp["issuer"], "https://auth.example.test");
    assert_eq!(
        resp["authorization_endpoint"],
        "https://auth.example.test/oauth/authorize"
    );
    assert_eq!(
        resp["token_endpoint"],
        "https://auth.example.test/oauth/token"
    );
    assert_eq!(
        resp["userinfo_endpoint"],
        "https://auth.example.test/userinfo"
    );
    assert_eq!(
        resp["jwks_uri"],
        "https://auth.example.test/.well-known/jwks.json"
    );
    assert!(resp["grant_types_supported"]
        .as_array()
        .unwrap()
        .contains(&Value::String("authorization_code".into())));
    assert!(resp["grant_types_supported"]
        .as_array()
        .unwrap()
        .contains(&Value::String("refresh_token".into())));
    assert!(resp["grant_types_supported"]
        .as_array()
        .unwrap()
        .contains(&Value::String("client_credentials".into())));
    assert_eq!(
        resp["code_challenge_methods_supported"],
        serde_json::json!(["S256"])
    );
    assert_eq!(
        resp["id_token_signing_alg_values_supported"],
        serde_json::json!(["RS256"])
    );
}
