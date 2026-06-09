use actix_web::{test, web, App};
use atlas_auth::{
    auth::token::{hash_refresh_token, new_refresh_token},
    oauth::refresh_token_repository::{NewRefreshToken, RefreshTokenRepository},
    routes,
};
use chrono::{Duration, Utc};
use serde_json::Value;
use uuid::Uuid;

#[actix_rt::test]
async fn introspect_requires_token() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/introspect")
        .set_form([("token_type_hint", "access_token")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    assert_eq!(body["message"], "token is required");
}

#[actix_rt::test]
async fn introspect_rejects_unsupported_token_type_hint() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/introspect")
        .set_form([("token", "some-token"), ("token_type_hint", "id_token")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "unsupported_token_type");
}

#[actix_rt::test]
async fn introspect_returns_inactive_until_db_backing_is_connected() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/introspect")
        .set_form([("token", "some-token"), ("token_type_hint", "access_token")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["active"], false);
}

#[actix_rt::test]
async fn introspect_returns_active_refresh_token_metadata_when_repository_is_configured() {
    let refresh_token = new_refresh_token();
    let refresh_token_hash = hash_refresh_token(&refresh_token);
    let user_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let client_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let refresh_tokens = RefreshTokenRepository::in_memory();
    refresh_tokens
        .save(NewRefreshToken {
            id: Uuid::new_v4(),
            user_id,
            client_id,
            token_hash: refresh_token_hash,
            family_id: Uuid::new_v4(),
            scope: vec!["openid".to_string(), "email".to_string()],
            expires_at: Utc::now() + Duration::days(14),
        })
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(refresh_tokens))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/introspect")
        .set_form([
            ("token", refresh_token.as_str()),
            ("token_type_hint", "refresh_token"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["active"], true);
    assert_eq!(body["sub"], user_id.to_string());
    assert_eq!(body["client_id"], client_id.to_string());
    assert_eq!(body["scope"], "openid email");
}
