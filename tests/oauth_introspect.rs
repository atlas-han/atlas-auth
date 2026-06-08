use actix_web::{test, App};
use atlas_auth::routes;
use serde_json::Value;

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
