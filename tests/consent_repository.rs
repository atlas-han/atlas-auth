use atlas_auth::oauth::consent_repository::{ConsentRepository, NewConsent};
use uuid::Uuid;

fn new_consent(scopes: Vec<&str>) -> NewConsent {
    NewConsent {
        id: Uuid::parse_str("77777777-7777-7777-7777-777777777777").unwrap(),
        user_id: Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
        client_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        scopes: scopes.into_iter().map(str::to_string).collect(),
    }
}

#[actix_rt::test]
async fn consent_repository_saves_and_checks_scope_coverage() {
    let repository = ConsentRepository::in_memory();
    let consent = new_consent(vec!["openid", "email"]);

    repository.save(consent.clone()).await.unwrap();

    assert!(repository
        .has_granted_scopes(consent.user_id, consent.client_id, &["openid".to_string()])
        .await
        .unwrap());
    assert!(repository
        .has_granted_scopes(
            consent.user_id,
            consent.client_id,
            &["openid".to_string(), "email".to_string()],
        )
        .await
        .unwrap());
    assert!(!repository
        .has_granted_scopes(
            consent.user_id,
            consent.client_id,
            &["openid".to_string(), "profile".to_string()],
        )
        .await
        .unwrap());
}

#[actix_rt::test]
async fn consent_repository_upserts_expanded_scope_set() {
    let repository = ConsentRepository::in_memory();
    let original = new_consent(vec!["openid"]);
    let expanded = new_consent(vec!["openid", "profile"]);

    repository.save(original.clone()).await.unwrap();
    repository.save(expanded.clone()).await.unwrap();

    assert!(repository
        .has_granted_scopes(
            original.user_id,
            original.client_id,
            &["openid".to_string(), "profile".to_string()],
        )
        .await
        .unwrap());
}
