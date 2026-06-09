use actix_web::{test, web, App};
use atlas_auth::oauth::{
    client_repository::{ClientRecord, OAuthClientRepository},
    consent_repository::{ConsentRepository, NewConsent},
};
use atlas_auth::routes;
use serde_json::Value;
use uuid::Uuid;

fn registered_client(trusted_first_party: bool) -> ClientRecord {
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
        scopes: vec![
            "openid".to_string(),
            "email".to_string(),
            "profile".to_string(),
        ],
        status: "active".to_string(),
        trusted_first_party,
        access_token_ttl_seconds: Some(900),
        refresh_token_ttl_seconds: Some(1_209_600),
    }
}

fn authorize_uri(user_id: Uuid, scope: &str) -> String {
    format!(
        "/oauth/authorize?response_type=code&client_id=client-1&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&scope={scope}&state=state-1&code_challenge=abcdefghijklmnopqrstuvwxyz1234567890ABCDEFG&code_challenge_method=S256&user_id={user_id}"
    )
}

#[actix_rt::test]
async fn authorize_allows_previously_consented_scope_set() {
    let user_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let client = registered_client(false);
    let consents = ConsentRepository::in_memory();
    consents
        .save(NewConsent {
            id: Uuid::new_v4(),
            user_id,
            client_id: client.id,
            scopes: vec!["openid".to_string(), "email".to_string()],
        })
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(OAuthClientRepository::in_memory(vec![
                client,
            ])))
            .app_data(web::Data::new(consents))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&authorize_uri(user_id, "openid%20email"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}

#[actix_rt::test]
async fn authorize_requires_consent_for_ungranted_scope() {
    let user_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let client = registered_client(false);
    let consents = ConsentRepository::in_memory();
    consents
        .save(NewConsent {
            id: Uuid::new_v4(),
            user_id,
            client_id: client.id,
            scopes: vec!["openid".to_string()],
        })
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(OAuthClientRepository::in_memory(vec![
                client,
            ])))
            .app_data(web::Data::new(consents))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&authorize_uri(user_id, "openid%20email"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "consent_required");
}

#[actix_rt::test]
async fn authorize_skips_consent_for_trusted_first_party_client() {
    let user_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(OAuthClientRepository::in_memory(vec![
                registered_client(true),
            ])))
            .app_data(web::Data::new(ConsentRepository::in_memory()))
            .configure(routes::oauth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&authorize_uri(user_id, "openid%20email"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}
