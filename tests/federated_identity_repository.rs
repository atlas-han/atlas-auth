use atlas_auth::auth::federated_identity_repository::{
    FederatedIdentityRepository, NewFederatedIdentity,
};
use serde_json::json;
use uuid::Uuid;

fn google_identity() -> NewFederatedIdentity {
    NewFederatedIdentity {
        id: Uuid::parse_str("88888888-8888-8888-8888-888888888888").unwrap(),
        user_id: Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
        provider: "google".to_string(),
        provider_user_id: "google-sub-1".to_string(),
        email: Some("user@example.test".to_string()),
        profile: json!({"name": "Test User"}),
    }
}

#[actix_rt::test]
async fn federated_identity_repository_links_and_finds_provider_identity() {
    let repository = FederatedIdentityRepository::in_memory();
    let identity = google_identity();

    repository.link(identity.clone()).await.unwrap();
    let stored = repository
        .find_by_provider_user_id("google", "google-sub-1")
        .await
        .unwrap()
        .expect("linked identity should be found");

    assert_eq!(stored.user_id, identity.user_id);
    assert_eq!(stored.provider, "google");
    assert_eq!(stored.provider_user_id, "google-sub-1");
    assert_eq!(stored.email.as_deref(), Some("user@example.test"));
}

#[actix_rt::test]
async fn federated_identity_repository_upserts_existing_provider_identity() {
    let repository = FederatedIdentityRepository::in_memory();
    let mut identity = google_identity();
    repository.link(identity.clone()).await.unwrap();

    identity.user_id = Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap();
    identity.email = Some("linked@example.test".to_string());
    repository.link(identity.clone()).await.unwrap();

    let stored = repository
        .find_by_provider_user_id("google", "google-sub-1")
        .await
        .unwrap()
        .expect("updated identity should be found");

    assert_eq!(stored.user_id, identity.user_id);
    assert_eq!(stored.email.as_deref(), Some("linked@example.test"));
}
