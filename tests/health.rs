use actix_web::{test, web, App};
use atlas_auth::{app::AppState, config::Settings, routes};
use sqlx::postgres::PgPoolOptions;

#[actix_rt::test]
async fn live_endpoint_returns_ok() {
    let settings = Settings {
        app_env: "test".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        database_url: "postgres://localhost/atlas_auth_test".to_string(),
        jwt_issuer: "atlas-auth".to_string(),
        jwt_audience: "atlas-services".to_string(),
        jwt_access_token_ttl_seconds: 900,
        jwt_refresh_token_ttl_seconds: 2_592_000,
        jwt_signing_secret: "test-secret-at-least-32-bytes".to_string(),
        password_pepper: "test-pepper".to_string(),
    };
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/atlas_auth_test")
        .expect("pool should be constructible lazily");
    let state = AppState { pool, settings };
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(routes::health::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/health/live").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
}
