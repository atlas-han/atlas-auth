use atlas_auth::auth::account_recovery_repository::{
    AccountRecoveryRepository, AccountTokenPurpose, NewAccountRecoveryToken,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

fn new_token(purpose: AccountTokenPurpose) -> NewAccountRecoveryToken {
    NewAccountRecoveryToken {
        id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        user_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
        token_hash: "hashed-token".to_string(),
        purpose,
        expires_at: Utc::now() + Duration::minutes(30),
    }
}

#[actix_rt::test]
async fn account_recovery_repository_stores_email_verification_token_by_hash() {
    let repository = AccountRecoveryRepository::in_memory();
    let token = new_token(AccountTokenPurpose::EmailVerification);

    repository.save(token.clone()).await.unwrap();
    let stored = repository
        .find_active_by_hash(
            "hashed-token",
            AccountTokenPurpose::EmailVerification,
            Utc::now(),
        )
        .await
        .unwrap()
        .expect("active email verification token should be found");

    assert_eq!(stored.user_id, token.user_id);
    assert_eq!(stored.token_hash, token.token_hash);
    assert_eq!(stored.purpose, AccountTokenPurpose::EmailVerification);
    assert!(stored.consumed_at.is_none());
}

#[actix_rt::test]
async fn account_recovery_repository_consumes_password_reset_tokens_once() {
    let repository = AccountRecoveryRepository::in_memory();
    let token = new_token(AccountTokenPurpose::PasswordReset);

    repository.save(token).await.unwrap();
    repository
        .consume(
            "hashed-token",
            AccountTokenPurpose::PasswordReset,
            Utc::now(),
        )
        .await
        .unwrap();

    let stored = repository
        .find_active_by_hash(
            "hashed-token",
            AccountTokenPurpose::PasswordReset,
            Utc::now(),
        )
        .await
        .unwrap();

    assert!(stored.is_none());
}

#[actix_rt::test]
async fn account_recovery_repository_rejects_duplicate_token_hash() {
    // The Postgres backend enforces `token_hash TEXT NOT NULL UNIQUE`, so the
    // in-memory backend must reject a duplicate the same way (Liskov parity):
    // a unique violation that `AppError` maps to `Conflict`, not a silent insert.
    let repository = AccountRecoveryRepository::in_memory();
    let token = new_token(AccountTokenPurpose::EmailVerification);

    repository.save(token.clone()).await.unwrap();
    let error = repository
        .save(token)
        .await
        .expect_err("a duplicate token_hash must be rejected like the Postgres UNIQUE constraint");

    match error {
        sqlx::Error::Database(db_error) => assert!(
            db_error.is_unique_violation(),
            "a duplicate insert should surface as a unique violation so it maps to AppError::Conflict",
        ),
        other => panic!("expected a database unique violation, got {other:?}"),
    }
}

#[actix_rt::test]
async fn account_recovery_repository_ignores_expired_tokens() {
    let repository = AccountRecoveryRepository::in_memory();
    let mut token = new_token(AccountTokenPurpose::PasswordReset);
    token.expires_at = Utc::now() - Duration::seconds(1);

    repository.save(token).await.unwrap();

    let stored = repository
        .find_active_by_hash(
            "hashed-token",
            AccountTokenPurpose::PasswordReset,
            Utc::now(),
        )
        .await
        .unwrap();

    assert!(stored.is_none());
}
