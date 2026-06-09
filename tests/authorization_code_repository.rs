use atlas_auth::oauth::authorization_code::{
    issue_authorization_code, AuthorizationCodeRepository, NewAuthorizationCode,
};
use chrono::Utc;
use uuid::Uuid;

fn new_code() -> NewAuthorizationCode {
    let issued = issue_authorization_code();
    NewAuthorizationCode {
        id: Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
        code_hash: issued.code_hash,
        client_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        user_id: Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
        redirect_uri: "https://app.example.test/callback".to_string(),
        code_challenge: "abcdefghijklmnopqrstuvwxyz1234567890ABCDEFG".to_string(),
        code_challenge_method: "S256".to_string(),
        scope: vec!["openid".to_string(), "email".to_string()],
        expires_at: issued.expires_at,
    }
}

#[actix_rt::test]
async fn authorization_code_repository_saves_and_finds_unconsumed_code_by_hash() {
    let repository = AuthorizationCodeRepository::in_memory();
    let code = new_code();
    let code_hash = code.code_hash.clone();

    repository.save(code.clone()).await.unwrap();
    let stored = repository
        .find_unconsumed_by_hash(&code_hash, Utc::now())
        .await
        .unwrap()
        .expect("saved code should be found");

    assert_eq!(stored.code_hash, code_hash);
    assert_eq!(stored.client_id, code.client_id);
    assert_eq!(stored.user_id, code.user_id);
    assert_eq!(stored.redirect_uri, code.redirect_uri);
    assert_eq!(stored.code_challenge_method, "S256");
    assert!(!stored.consumed);
}

#[actix_rt::test]
async fn authorization_code_repository_does_not_find_consumed_code() {
    let repository = AuthorizationCodeRepository::in_memory();
    let code = new_code();
    let code_hash = code.code_hash.clone();

    repository.save(code).await.unwrap();
    repository.consume(&code_hash).await.unwrap();

    let stored = repository
        .find_unconsumed_by_hash(&code_hash, Utc::now())
        .await
        .unwrap();

    assert!(stored.is_none());
}
