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
async fn revoke_requires_token() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/revoke")
        .set_form([("token_type_hint", "refresh_token")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    assert_eq!(body["message"], "token is required");
}

#[actix_rt::test]
async fn revoke_rejects_unsupported_token_type_hint() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/revoke")
        .set_form([("token", "some-token"), ("token_type_hint", "id_token")])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "unsupported_token_type");
}

#[actix_rt::test]
async fn revoke_accepts_refresh_token_hint_shape() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/revoke")
        .set_form([
            ("token", "refresh-token"),
            ("token_type_hint", "refresh_token"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}

#[actix_rt::test]
async fn revoke_marks_refresh_token_as_revoked_when_repository_is_configured() {
    let refresh_token = new_refresh_token();
    let refresh_token_hash = hash_refresh_token(&refresh_token);
    let refresh_tokens = RefreshTokenRepository::in_memory();
    refresh_tokens
        .save(NewRefreshToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            client_id: Uuid::new_v4(),
            token_hash: refresh_token_hash.clone(),
            family_id: Uuid::new_v4(),
            scope: vec!["openid".to_string()],
            expires_at: Utc::now() + Duration::days(14),
        })
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(refresh_tokens.clone()))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/oauth/revoke")
        .set_form([
            ("token", refresh_token.as_str()),
            ("token_type_hint", "refresh_token"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    let revoked_record = refresh_tokens
        .find_by_hash(&refresh_token_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(revoked_record.revoked);
}
