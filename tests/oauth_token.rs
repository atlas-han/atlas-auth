use actix_web::{test, App};
use atlas_auth::routes;
use serde_json::Value;

#[actix_rt::test]
async fn token_rejects_password_grant() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "password"),
            ("username", "user@example.test"),
            ("password", "password123456"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "unsupported_grant_type");
}

#[actix_rt::test]
async fn authorization_code_token_request_requires_code_and_verifier() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "authorization_code"),
            ("client_id", "client-1"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    assert_eq!(body["message"], "code and code_verifier are required");
}

#[actix_rt::test]
async fn token_validation_accepts_supported_grant_shape() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::post()
        .uri("/oauth/token")
        .set_form([
            ("grant_type", "authorization_code"),
            ("client_id", "client-1"),
            ("code", "authorization-code"),
            ("code_verifier", "verifier-with-enough-length-1234567890"),
            ("redirect_uri", "https://app.example.test/callback"),
        ])
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}
