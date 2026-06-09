use atlas_auth::oauth::refresh_token_repository::{NewRefreshToken, RefreshTokenRepository};
use chrono::{Duration, Utc};
use uuid::Uuid;

fn new_refresh_token() -> NewRefreshToken {
    NewRefreshToken {
        id: Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap(),
        user_id: Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
        client_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        token_hash: "hash-1".to_string(),
        family_id: Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap(),
        scope: vec!["openid".to_string(), "email".to_string()],
        expires_at: Utc::now() + Duration::days(14),
    }
}

#[actix_rt::test]
async fn refresh_token_repository_saves_active_token_by_hash() {
    let repository = RefreshTokenRepository::in_memory();
    let token = new_refresh_token();

    repository.save(token.clone()).await.unwrap();
    let stored = repository
        .find_by_hash("hash-1")
        .await
        .unwrap()
        .expect("saved token should be found");

    assert_eq!(stored.token_hash, token.token_hash);
    assert_eq!(stored.user_id, token.user_id);
    assert_eq!(stored.client_id, token.client_id);
    assert_eq!(stored.family_id, token.family_id);
    assert_eq!(stored.scope, token.scope);
    assert!(!stored.revoked);
}
