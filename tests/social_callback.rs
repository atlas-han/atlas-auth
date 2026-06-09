use actix_web::{test, web, App};
use atlas_auth::{auth::federated_identity_repository::FederatedIdentityRepository, routes};
use serde_json::Value;
use uuid::Uuid;

#[actix_rt::test]
async fn social_callback_links_google_identity_to_existing_user_hint() {
    let identities = FederatedIdentityRepository::in_memory();
    let user_id = Uuid::new_v4();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(identities.clone()))
            .configure(routes::auth::configure),
    )
    .await;

    let uri = format!(
        "/v1/auth/social/google/callback?provider_user_id=google-123&email=user@example.test&user_id={user_id}"
    );
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["provider"], "google");
    assert_eq!(body["user_id"], user_id.to_string());

    let stored = identities
        .find_by_provider_user_id("google", "google-123")
        .await
        .unwrap()
        .expect("provider identity should be linked");
    assert_eq!(stored.user_id, user_id);
    assert_eq!(stored.email.as_deref(), Some("user@example.test"));
}

#[actix_rt::test]
async fn social_callback_reuses_existing_provider_identity_for_account_linking() {
    let identities = FederatedIdentityRepository::in_memory();
    let user_id = Uuid::new_v4();
    identities
        .link(
            atlas_auth::auth::federated_identity_repository::NewFederatedIdentity {
                id: Uuid::new_v4(),
                user_id,
                provider: "facebook".to_string(),
                provider_user_id: "fb-123".to_string(),
                email: Some("user@example.test".to_string()),
                profile: serde_json::json!({"name":"User"}),
            },
        )
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(identities))
            .configure(routes::auth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/v1/auth/social/facebook/callback?provider_user_id=fb-123&email=changed@example.test")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, actix_web::http::StatusCode::OK);
    assert_eq!(body["user_id"], user_id.to_string());
    assert_eq!(body["linked_existing"], true);
}

#[actix_rt::test]
async fn social_callback_rejects_unsupported_provider() {
    let identities = FederatedIdentityRepository::in_memory();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(identities))
            .configure(routes::auth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/v1/auth/social/kakao/callback?provider_user_id=kakao-123")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}
