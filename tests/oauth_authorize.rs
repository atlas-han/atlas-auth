use actix_web::{test, App};
use atlas_auth::routes;
use serde_json::Value;

#[actix_rt::test]
async fn authorize_rejects_implicit_flow() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::get()
        .uri("/oauth/authorize?response_type=token&client_id=client-1&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&scope=openid&state=state-1&code_challenge=challenge&code_challenge_method=S256")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn authorize_requires_s256_pkce_challenge() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::get()
        .uri("/oauth/authorize?response_type=code&client_id=client-1&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&scope=openid&state=state-1")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    assert_eq!(body["message"], "S256 PKCE code_challenge is required");
}

#[actix_rt::test]
async fn authorize_validation_accepts_code_with_s256_pkce() {
    let app = test::init_service(App::new().configure(routes::oauth::configure)).await;

    let req = test::TestRequest::get()
        .uri("/oauth/authorize?response_type=code&client_id=client-1&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&scope=openid%20email&state=state-1&code_challenge=abcdefghijklmnopqrstuvwxyz1234567890ABCDEFG&code_challenge_method=S256")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}
