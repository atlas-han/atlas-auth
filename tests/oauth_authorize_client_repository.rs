use actix_web::{test, web, App};
use atlas_auth::oauth::client_repository::{ClientRecord, OAuthClientRepository};
use atlas_auth::routes;
use serde_json::Value;
use uuid::Uuid;

fn registered_client() -> ClientRecord {
    ClientRecord {
        id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        public_client_id: "client-1".to_string(),
        client_secret_hash: None,
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
async fn authorize_rejects_unknown_client_when_repository_is_configured() {
    let repo = OAuthClientRepository::in_memory(vec![registered_client()]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repo))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/oauth/authorize?response_type=code&client_id=missing-client&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&scope=openid&state=state-1&code_challenge=abcdefghijklmnopqrstuvwxyz1234567890ABCDEFG&code_challenge_method=S256")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_client");
}

#[actix_rt::test]
async fn authorize_uses_registered_client_policy_for_redirect_and_scope() {
    let repo = OAuthClientRepository::in_memory(vec![registered_client()]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repo))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/oauth/authorize?response_type=code&client_id=client-1&redirect_uri=https%3A%2F%2Fevil.example.test%2Fcallback&scope=openid&state=state-1&code_challenge=abcdefghijklmnopqrstuvwxyz1234567890ABCDEFG&code_challenge_method=S256")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    assert_eq!(
        body["message"],
        "redirect_uri is not registered for this client"
    );
}

#[actix_rt::test]
async fn authorize_accepts_registered_client_policy() {
    let repo = OAuthClientRepository::in_memory(vec![registered_client()]);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(repo))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/oauth/authorize?response_type=code&client_id=client-1&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&scope=openid%20email&state=state-1&code_challenge=abcdefghijklmnopqrstuvwxyz1234567890ABCDEFG&code_challenge_method=S256")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}
